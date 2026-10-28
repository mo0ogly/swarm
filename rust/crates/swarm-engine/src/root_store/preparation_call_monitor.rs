//! Reap an owned preparation child without silently leaving active SQL state.
use super::{
    finished, preparation_read::cause, RootInstanceIdentity, RootStore, RootStoreError as E,
};
use std::{
    path::{Path, PathBuf},
    process::Child,
    sync::mpsc,
};

/// Registration observed before opening this child's gate. It is process/Root
/// association, not agent authentication, and never authorizes raw-PID signals.
#[derive(Clone)]
pub(super) struct Registration {
    pub root: RootInstanceIdentity,
    pub pid: i64,
    pub stamp: String,
    pub host: String,
}
/// Retained safe monitor evidence. No raw IO messages, source paths or secrets.
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Notice {
    pub code: String,
    pub io_kind: Option<String>,
    pub errno: Option<i32>,
}
impl Notice {
    /// Static phase plus typed IO classification, not proof of remote effects.
    pub(super) fn new(code: &'static str, error: Option<&std::io::Error>) -> Self {
        Self {
            code: code.into(),
            io_kind: error.map(|e| format!("{:?}", e.kind())),
            errno: error.and_then(std::io::Error::raw_os_error),
        }
    }
}
/// Reopen only the registered physical datastore, record monitor evidence and
/// use public stop/recovery semantics. Closing retains any primary failure.
fn settle(path: &Path, id: &str, r: &Registration, notice: Notice) -> Result<(), E> {
    let root = RootStore::open(path, true)?;
    let result = (|| {
        if root.instance()? != r.root {
            return Err(E::Refused("preparation-monitor-root-changed"));
        }
        root.preparation_monitor_settle(id, r, notice)
    })();
    finished(result, root.close())
}
/// Supervise one already registered owned child. Thread creation or channel
/// failure returns ownership for caller cleanup, never loses a Child on spawn
/// refusal. Successful detach reaps while this parent lives. A short-lived CLI
/// cannot promise a surviving monitor: explicit stop remains its recovery path.
/// Normal child exit also checks unresolved state, covering SIGKILL/crash before
/// terminal publication. Estimated budget is never refunded by this monitor.
pub(super) fn reap(
    child: Child,
    path: PathBuf,
    id: String,
    r: Registration,
) -> Result<(), (Child, E)> {
    let (tx, rx) = mpsc::channel::<Child>();
    let monitor_r = r.clone();
    let spawned =
        std::thread::Builder::new()
            .name("preparation-reaper".into())
            .spawn(move || {
                let notice = match rx.recv() {
                    Ok(mut child) => match child.wait() {
                        Ok(_) => Notice::new("preparation-worker-exited-without-final-state", None),
                        Err(error) => Notice::new("preparation-worker-reap-failed", Some(&error)),
                    },
                    Err(_) => Notice::new("preparation-worker-monitor-channel-closed", None),
                };
                let diagnostic = (notice.code.clone(), notice.io_kind.clone(), notice.errno);
                if let Err(error) = settle(&path, &id, &monitor_r, notice) {
                    // The owned local service's stderr is its supported diagnostic sink.
                    // Never print raw nested causes/paths or infer successful settlement.
                    eprintln!(
                    "preparation-worker-monitor-recovery-failed: {} phase:{} kind:{:?} errno:{:?}",
                    error.code(),diagnostic.0,diagnostic.1,diagnostic.2
                );
                }
            });
    if let Err(error) = spawned {
        return Err((child, cause("preparation-worker-monitor-spawn", error)));
    }
    match tx.send(child) {
        Ok(()) => Ok(()),
        Err(error) => Err((
            error.0,
            E::Refused("preparation-worker-monitor-channel-closed"),
        )),
    }
}
