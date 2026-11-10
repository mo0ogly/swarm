//! Compatibility fixture: compile against retained engine192 producing Binding v2.
//! Keep its real lease alive; never build this fixture against the new v3 engine.
//! This is not an operator command, grant token or production entrypoint.
use serde_json::{json, Value};
use std::{
    error::Error,
    fmt, fs,
    io::{self, BufRead, Write},
    path::Path,
};
use swarm_engine::{MissionWorkerRequest, RootStore, SystemEnvironment};

/// Retain every independent fixture/action/close cause. source exposes the first;
/// Debug retains the complete collection, with no claim that failure rolls back.
#[derive(Debug)]
struct Failures(Vec<Box<dyn Error>>);
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "legacy fixture failed in {} phase(s)", self.0.len())
    }
}
impl Error for Failures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|e| e.as_ref())
    }
}

/// Inspect only this fixture's known direct source chain for its static EOF.
/// RuntimeError Debug intentionally redacts private IO text. Emit a safe witness
/// only when the actual retained io::Error has the expected kind and cause;
/// this is not a generic wrapper traversal or evidence of cleanup success.
fn retained_eof(error: &(dyn Error + 'static)) -> bool {
    let mut next = Some(error);
    while let Some(error) = next {
        if let Some(io) = error.downcast_ref::<io::Error>() {
            if io.kind() == io::ErrorKind::UnexpectedEof
                && io.to_string() == "legacy finish signal missing"
            {
                return true;
            }
        }
        next = error.source();
    }
    false
}
/// Public start/claim/prepare creates genuine v2 provenance on an owned fixture.
/// The retained engine192 must write both profile hashes and no FIFO sequence;
/// this fixture neither manufactures v2 metadata nor grants retroactive FIFO.
/// Fixture-v2 readiness includes the exact original public start Value passed
/// to the old Root, distinct from its internal canonical audit representation.
/// JSON line readiness is emitted only under the running native heartbeat;
/// stdin newline requests exit, EOF refuses. Caller runs the NEW normal CLI
/// while this old engine remains the live owning process. No model is launched
/// by this example. Marker/read/SQL/clock/output/action/close causes are retained.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("legacy fixture requires one request path".into());
    }
    let input: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let root = Path::new(input["root"].as_str().ok_or("root missing")?).canonicalize()?;
    let work = input["work"].as_str().ok_or("work missing")?;
    let marker: Value = serde_json::from_slice(&fs::read(
        root.join(".swarm/verification-task-profile.json"),
    )?)?;
    if marker != input {
        return Err("owned legacy fixture marker mismatch".into());
    }
    let mut store = RootStore::open(&root, true)?;
    let mut owning_lease = None;
    let action = (|| -> Result<(), Box<dyn Error>> {
        let start = json!({"schema_version":1,"event_id":"legacy-profile-start",
        "expected_revision":store.read_work(work)?.revision,"reason":"Owned v2 compatibility witness",
        "slots":1,"profile":{"provider":"fixture","role":"worker","workspace":".",
        "level":"simple","capture_output":true}});
        store.execute_external(
            work,
            "mission.start",
            &serde_json::to_vec(&start)?,
            &mut SystemEnvironment,
        )?;
        owning_lease = Some(store.claim_mission_supervision(
            work,
            "legacy-profile-fixture",
            &swarm_engine::system_timestamp()?,
        )?);
        let lease = owning_lease.as_ref().ok_or("legacy lease missing")?;
        let request = MissionWorkerRequest {
            schema_version: 1,
            event_id: "legacy-automatic".into(),
            expected_revision: store.read_work(work)?.revision,
            task_id: "task".into(),
        };
        let queued =
            store.prepare_mission_worker(lease, &request, false, &mut SystemEnvironment)?;
        let binding: Value = serde_json::from_str(
            queued
                .agent
                .native_mission
                .as_ref()
                .ok_or("binding missing")?
                .0
                .get(),
        )?;
        let parent_hash = binding.get("parent_profile_sha256").and_then(Value::as_str);
        if binding["schema_version"] != 2
            || !parent_hash.is_some_and(|hash| {
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            || binding.get("workspace_turn_seq").is_some()
        {
            return Err("fixture was not compiled against retained v2 engine192".into());
        }
        store.run_mission_cycle(lease, |_,_,monitor| {
            if !monitor.is_running() { return Err(swarm_engine::RootStoreError::Refused("legacy-heartbeat-not-running")); }
            let bytes = serde_json::to_vec(&json!({"fixture_schema_version":2,"ready":true,"agent":queued.agent.id,"binding":binding,"public_start_request":start.clone()}))
                .map_err(|e|swarm_engine::RootStoreError::Runtime(swarm_engine::RuntimeError::new(swarm_engine::RuntimeOperation::Output,io::Error::other(e))))?;
            let io = (|| -> io::Result<()> {
                let mut out = io::stdout().lock();out.write_all(&bytes)?;out.write_all(b"\n")?;out.flush()?;
                let mut line=String::new();
                if io::stdin().lock().read_line(&mut line)? == 0 {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"legacy finish signal missing"));
                }
                Ok(())
            })();
            io.map_err(|e|swarm_engine::RootStoreError::Runtime(swarm_engine::RuntimeError::new(swarm_engine::RuntimeOperation::Output,e)))
        })?;
        Ok(())
    })();
    let mut errors = Vec::<Box<dyn Error>>::new();
    if let Err(e) = action {
        if retained_eof(e.as_ref()) {
            if let Err(output) = writeln!(io::stderr().lock(), "legacy EOF cause retained") {
                errors.push(Box::new(output));
            }
        }
        errors.push(e);
    }
    if let Some(lease) = owning_lease {
        if let Err(e) = lease.close() {
            errors.push(Box::new(e));
        }
    }
    if let Err(e) = store.close() {
        errors.push(Box::new(e));
    }
    if !errors.is_empty() {
        return Err(Box::new(Failures(errors)));
    }
    Ok(())
}
