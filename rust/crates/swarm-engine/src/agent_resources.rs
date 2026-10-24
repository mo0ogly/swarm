//! Read-only Linux resource observation of an agent's recorded processes for the
//! live views: resident memory, CPU time and thread count of the supervisor and
//! of the provider process with its descendants.
//!
//! A PID is read only when its `/proc` starttime equals the identity the engine
//! recorded for it and the agent ran on this host tuple; otherwise the slot says
//! why (`ended`, `identity-mismatch`, `not-recorded`, `unavailable`) and carries
//! no number. Nothing is signalled, written or inferred about progress.
use crate::{observe_linux_process, Agent};
use serde::Serialize;
use std::collections::BTreeMap;

/// Resources of one recorded process (plus, for the provider, its descendants).
/// `cpu_ns` is cumulative on-CPU time from `/proc/PID/schedstat`; a rate needs
/// two samples. Absent numbers mean the kernel field was unreadable, not zero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProcessSlot {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_ns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<u64>,
    /// Live descendants included in the numbers (provider slot only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descendants: Option<u32>,
}
impl ProcessSlot {
    fn empty(state: &'static str) -> Self {
        Self {
            state,
            pid: None,
            rss_bytes: None,
            cpu_ns: None,
            threads: None,
            descendants: None,
        }
    }
}

/// One observation of an agent's processes. `observation` is `observed` when
/// this host may read them, `other-host` when the agent ran elsewhere (no read).
/// `sampled_at_ms` is wall-clock Unix time in milliseconds, for client rates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentResources {
    pub observation: &'static str,
    pub supervisor: ProcessSlot,
    pub provider: ProcessSlot,
    pub sampled_at_ms: u128,
}

/// Parent links of every live process, read once per call so a batch of agents
/// shares one `/proc` scan. Unreadable entries (processes ending during the
/// scan) are skipped: they cannot be attributed safely.
pub struct ProcessTable {
    parents: BTreeMap<u32, u32>,
}
impl ProcessTable {
    /// Scan `/proc`. An unreadable `/proc` directory is an error for the caller
    /// to report; individual vanished entries are not.
    pub fn scan() -> std::io::Result<Self> {
        let mut parents = BTreeMap::new();
        for entry in std::fs::read_dir("/proc")? {
            let Ok(entry) = entry else { continue };
            let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            if let Ok(observation) = observe_linux_process(pid) {
                parents.insert(pid, observation.parent_pid());
            }
        }
        Ok(Self { parents })
    }
    /// All live descendants of `root` (excluding `root`), cycle-safe.
    fn descendants(&self, root: u32) -> Vec<u32> {
        let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (&pid, &parent) in &self.parents {
            children.entry(parent).or_default().push(pid);
        }
        let (mut out, mut stack) = (Vec::new(), vec![root]);
        while let Some(pid) = stack.pop() {
            for &child in children.get(&pid).into_iter().flatten() {
                if child != root && !out.contains(&child) {
                    out.push(child);
                    stack.push(child);
                }
            }
        }
        out
    }
}

/// Observe the recorded supervisor and provider processes of `agent`. `host` is
/// the current host tuple (see [`crate::SupervisorIdentity::host`]); `table`
/// supplies descendants. Reads only; never fails as a whole — each slot carries
/// its own state so one vanished process cannot hide the other.
pub fn observe_agent_resources(agent: &Agent, host: &str, table: &ProcessTable) -> AgentResources {
    let sampled_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if agent.host.is_empty() || agent.host != host {
        return AgentResources {
            observation: "other-host",
            supervisor: ProcessSlot::empty("not-read"),
            provider: ProcessSlot::empty("not-read"),
            sampled_at_ms,
        };
    }
    let supervisor = slot(agent.supervisor_pid, &agent.supervisor_identity, None);
    let provider = slot(agent.child_pid, &agent.child_identity, Some(table));
    AgentResources {
        observation: "observed",
        supervisor,
        provider,
        sampled_at_ms,
    }
}

/// Verify identity, then read the process (and descendants when `table`).
fn slot(pid: i64, identity: &str, table: Option<&ProcessTable>) -> ProcessSlot {
    let Ok(pid) = u32::try_from(pid) else {
        return ProcessSlot::empty("not-recorded");
    };
    if pid == 0 || identity.is_empty() {
        return ProcessSlot::empty("not-recorded");
    }
    match observe_linux_process(pid) {
        Err(error) if error.confirms_absence() => return ProcessSlot::empty("ended"),
        Err(_) => return ProcessSlot::empty("unavailable"),
        Ok(observation) if observation.process_stamp() != identity => {
            return ProcessSlot::empty("identity-mismatch")
        }
        Ok(_) => {}
    }
    let own = read_usage(pid);
    let mut total = own.clone();
    let mut descendants = None;
    if let Some(table) = table {
        let found = table.descendants(pid);
        let mut counted = 0u32;
        for child in found {
            let usage = read_usage(child);
            if usage.alive {
                counted += 1;
                total.add(&usage);
            }
        }
        descendants = Some(counted);
    }
    ProcessSlot {
        state: "observed",
        pid: Some(pid),
        rss_bytes: total.rss_bytes,
        cpu_ns: total.cpu_ns,
        threads: total.threads,
        descendants,
    }
}

#[derive(Clone, Default)]
struct Usage {
    alive: bool,
    rss_bytes: Option<u64>,
    cpu_ns: Option<u64>,
    threads: Option<u64>,
}
impl Usage {
    /// Sum known values; an unknown component makes the total unknown rather
    /// than silently smaller. Overflow also remains unknown; no zero is invented.
    fn add(&mut self, other: &Usage) {
        /// Combine known inputs without mutation; missing/overflow returns unknown.
        fn sum(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            a?.checked_add(b?)
        }
        self.rss_bytes = sum(self.rss_bytes, other.rss_bytes);
        self.cpu_ns = sum(self.cpu_ns, other.cpu_ns);
        self.threads = sum(self.threads, other.threads);
    }
}

/// `VmRSS`/`Threads` from `/proc/PID/status` (kB is the kernel ABI unit) and the
/// first `schedstat` field (ns on CPU). Missing kernel threads have no VmRSS:
/// they report 0 resident bytes, which is the kernel's own statement.
fn read_usage(pid: u32) -> Usage {
    let Ok(status) = crate::supervisor_identity::proc_text(&format!("/proc/{pid}/status"), "live-status") else {
        return Usage::default();
    };
    let field = |name: &str| {
        status.lines().find_map(|line| {
            let rest = line.strip_prefix(name)?.strip_prefix(':')?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
    };
    let rss_bytes = match field("VmRSS") {
        Some(kb) => kb.checked_mul(1024),
        None if status.lines().any(|l| l.starts_with("Name:")) => Some(0),
        None => None,
    };
    let cpu_ns = crate::supervisor_identity::proc_text(&format!("/proc/{pid}/schedstat"), "live-schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok());
    Usage {
        alive: true,
        rss_bytes,
        cpu_ns,
        threads: field("Threads"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SupervisorIdentity;
    use std::os::unix::process::CommandExt;

    /// Test child in its own process group. Drop kills the whole group and
    /// reaps the child, so a failing assertion cannot leave grandchildren (e.g.
    /// the shell's `sleep`) orphaned on the host.
    struct GroupChild(std::process::Child);
    impl GroupChild {
        fn spawn(program: &str, args: &[&str]) -> Self {
            let child = std::process::Command::new(program)
                .args(args)
                .process_group(0)
                .spawn()
                .unwrap();
            Self(child)
        }
        fn stop(&mut self) {
            if let Some(group) = rustix::process::Pid::from_raw(self.0.id() as i32) {
                let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
            }
            let _ = self.0.wait();
        }
    }
    impl Drop for GroupChild {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Fixture metadata for one supplied child and current supervisor. Supplied
    /// host/stamp may be deliberately invalid; this grants no signal authority.
    fn agent_with(child: &std::process::Child, identity: &str, host: &str) -> Agent {
        let me = SupervisorIdentity::current().unwrap();
        Agent {
            host: host.into(),
            supervisor_pid: i64::from(me.pid() as u32),
            supervisor_identity: me.process_stamp().into(),
            child_pid: i64::from(child.id()),
            child_identity: identity.into(),
            ..Default::default()
        }
    }

    #[test]
    fn verified_processes_are_read_with_descendants() {
        // A shell whose own child sleeps: provider slot must count one descendant.
        let mut group = GroupChild::spawn("sh", &["-c", "sleep 30 & wait"]);
        let child = &group.0;
        let me = SupervisorIdentity::current().unwrap();
        let stamp = loop {
            let o = observe_linux_process(child.id()).unwrap();
            let table = ProcessTable::scan().unwrap();
            if !table.descendants(child.id()).is_empty() {
                break o.process_stamp().to_owned();
            }
            std::thread::yield_now();
        };
        let agent = agent_with(child, &stamp, me.host());
        let sleeper = ProcessTable::scan().unwrap().descendants(child.id())[0];
        let sleeper_stamp = observe_linux_process(sleeper).unwrap().process_stamp().to_owned();
        let table = ProcessTable::scan().unwrap();
        let r = observe_agent_resources(&agent, me.host(), &table);
        assert_eq!(r.observation, "observed");
        assert_eq!(r.supervisor.state, "observed");
        assert!(r.supervisor.rss_bytes.unwrap() > 0);
        assert!(r.supervisor.cpu_ns.is_some());
        assert_eq!(r.provider.state, "observed");
        assert_eq!(r.provider.pid, Some(child.id()));
        assert_eq!(r.provider.descendants, Some(1));
        assert!(r.provider.threads.unwrap() >= 2);
        group.stop();
        // SIGKILL to the group is delivered asynchronously and cannot be ignored:
        // wait until the grandchild (same PID *and* starttime) is no longer live.
        while observe_linux_process(sleeper).is_ok_and(|o| o.process_stamp() == sleeper_stamp) {
            std::thread::yield_now();
        }
        // Reaped: the recorded provider PID is gone.
        let after = observe_agent_resources(&agent, me.host(), &ProcessTable::scan().unwrap());
        assert_eq!(after.provider.state, "ended");
        assert_eq!(after.provider.rss_bytes, None);
    }

    #[test]
    fn wrong_identity_or_host_reads_nothing() {
        let group = GroupChild::spawn("sleep", &["30"]);
        let child = &group.0;
        let me = SupervisorIdentity::current().unwrap();
        let table = ProcessTable::scan().unwrap();
        let forged = agent_with(child, "1", me.host());
        let r = observe_agent_resources(&forged, me.host(), &table);
        assert_eq!(r.provider.state, "identity-mismatch");
        assert_eq!((r.provider.pid, r.provider.rss_bytes), (None, None));
        let elsewhere = agent_with(child, "1", "other:boot:ns");
        let r = observe_agent_resources(&elsewhere, me.host(), &table);
        assert_eq!(r.observation, "other-host");
        assert_eq!(r.supervisor.state, "not-read");
        let none = Agent { host: me.host().into(), ..Default::default() };
        let r = observe_agent_resources(&none, me.host(), &table);
        assert_eq!((r.supervisor.state, r.provider.state), ("not-recorded", "not-recorded"));
    }
}
