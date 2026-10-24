#![cfg(target_os = "linux")]
//! Actual TCP commit/disconnect and observer IO doubles; never a live mission Root.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
use swarm_web::{
    NativeHttpServer, NativeJobOutcome, NativeObservationCause, NativeServerError,
    NativeServerReport,
};
/// Owned server handles; callers stop/join before removing their successful Root.
type Running = (
    tokio::sync::oneshot::Sender<()>,
    std::thread::JoinHandle<Result<NativeServerReport, NativeServerError>>,
);
struct Fixture {
    root: PathBuf,
    server: Option<NativeHttpServer>,
    address: SocketAddr,
    key: String,
    cookie: String,
}
impl Fixture {
    /// Initialize a private Root and verify the real report consumer before any fault.
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(swarm_engine::generate_identifier("tcp-observer96-").unwrap());
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        RootStore::initialize(&root, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        fs::write(root.join("report.md"), b"report").unwrap();
        let r = RootStore::open(&root, false).unwrap();
        assert_eq!(r.review_delivery("report.md").unwrap(), "report");
        r.close().unwrap();
        let server = NativeHttpServer::bind(&root, None).unwrap();
        let address = server.local_addr().unwrap();
        let mut fixture = Self {
            root,
            server: Some(server),
            address,
            key: String::new(),
            cookie: String::new(),
        };
        fixture.load_session();
        fixture
    }
    /// Read only this fixture's session for the actual bound address. Filename and
    /// cookie suffixes are the existing Go-compatible protocol, not guessed keys.
    fn load_session(&mut self) {
        let hash = format!("{:x}", Sha256::digest(self.address.to_string().as_bytes()));
        self.key = fs::read_to_string(
            self.root
                .join(".swarm")
                .join(format!("web-session-{}", &hash[..16])),
        )
        .unwrap();
        self.cookie = format!("swarm_session_{}={}", &hash[..12], self.key);
    }
    /// Launch one owned server with the explicit observer; no production fallback.
    fn start<F>(&mut self, mut observer: F) -> Running
    where
        F: FnMut(&NativeJobOutcome) -> io::Result<()> + Send + 'static,
    {
        let server = self.server.take().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        (
            tx,
            std::thread::spawn(move || {
                server.serve_until(
                    async {
                        let _ = rx.await;
                    },
                    &mut observer,
                )
            }),
        )
    }
    /// Restart only the owned fixture listener after its predecessor was joined.
    /// This preserves Root/history; it is not a restart of the serving application.
    fn restart(&mut self) {
        let server = NativeHttpServer::bind(&self.root, None).unwrap();
        self.address = server.local_addr().unwrap();
        self.server = Some(server);
        self.load_session();
    }
    /// Complete owned HTTP frame. Origin/CSRF are actual observed-address scoped.
    fn frame(&self, path: &str, body: Option<&str>, csrf: Option<&str>) -> String {
        let method = if body.is_some() { "POST" } else { "GET" };
        let raw = body.unwrap_or("");
        let token = csrf
            .map(|s| format!("X-Swarm-CSRF: {s}\r\n"))
            .unwrap_or_default();
        format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nOrigin: http://{}\r\n{token}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}",self.address,self.cookie,self.address,raw.len())
    }
    /// Exchange real bytes. Failed observer shutdown may interrupt response IO;
    /// that does not turn a known Root effect into rollback or successful delivery.
    fn exchange(&self, path: &str, body: Option<&str>, csrf: Option<&str>) -> io::Result<String> {
        let mut stream = TcpStream::connect(self.address)?;
        stream.write_all(self.frame(path, body, csrf).as_bytes())?;
        stream.flush()?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw)?;
        Ok(raw)
    }
    /// Require meaningful200JSON on normal witnesses before injecting output failure.
    fn json(&self, path: &str, body: Option<&str>, csrf: Option<&str>) -> serde_json::Value {
        let raw = self.exchange(path, body, csrf).unwrap();
        assert!(raw.starts_with("HTTP/1.1 200"), "{raw}");
        serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
    /// Prepare through actual session/preview routes and verify preview is read-only.
    fn prepare(&self) -> (String, String) {
        let session = self.json("/api/v1/native-session", None, None);
        let csrf = session["csrf_token"].as_str().unwrap().to_owned();
        assert_ne!(csrf, self.key);
        let r = RootStore::open(&self.root, false).unwrap();
        let d = r.settings().unwrap();
        r.close().unwrap();
        let before = self.saved();
        let mut values = d.values;
        values.review_delivery_max_bytes = Some(SettingCap(Some(SettingCount(3))));
        let mut change = EngineSettingsChange {
            schema_version: d.schema_version,
            event_id: "observer-tcp96".into(),
            expected_revision: d.revision,
            values,
            reason: "Owned network observer failure witness".into(),
            preview_token: String::new(),
        };
        let preview = self.json(
            "/api/v1/native-settings/preview",
            Some(&serde_json::to_string(&change).unwrap()),
            Some(&csrf),
        );
        assert_eq!(self.saved(), before);
        change.preview_token = preview["preview_token"].as_str().unwrap().into();
        (csrf, serde_json::to_string(&change).unwrap())
    }
    /// Exact saved settings bytes, preserving original history for replay assertions.
    fn saved(&self) -> Vec<u8> {
        fs::read(self.root.join(".swarm/engine-settings.json")).unwrap()
    }
    /// Public Root readback proves the actual effect and consumer, not just HTTP200.
    fn verify_effect(&self) {
        let r = RootStore::open(&self.root, false).unwrap();
        assert_eq!(r.settings().unwrap().revision, SettingCount(2));
        assert!(r.review_delivery("report.md").is_err());
        assert!(r.list_works().unwrap().is_empty());
        r.close().unwrap();
    }
}
impl Drop for Fixture {
    /// Remove only owned successful fixtures after stop/join. Failed Roots survive.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(self.server.is_none());
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
/// Original IO failure at either write or flush; no OS fault injection claimed.
struct BrokenSink {
    flush_failure: bool,
    bytes: Vec<u8>,
}
impl Write for BrokenSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.flush_failure {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "owned TCP observer write",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "owned TCP observer flush",
        ))
    }
}
/// Safe explicit record/write/flush, retaining original IO errors at each boundary.
fn observe(sink: &mut dyn Write, outcome: &NativeJobOutcome) -> io::Result<()> {
    let raw = serde_json::to_vec(&outcome.record()).map_err(io::Error::other)?;
    sink.write_all(&raw)?;
    sink.flush()
}
/// Session/preview are valid before each failing output. Actual apply is preserved
/// in the error returned to owner, then explicit replay after readback stays exact.
#[test]
fn native_tcp_observer_write_and_flush_failure_preserve_committed_settings_and_replay() {
    for flush_failure in [false, true] {
        let mut f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let mut witness = Vec::new();
        let mut failed = BrokenSink {
            flush_failure,
            bytes: Vec::new(),
        };
        let (_stop, handle) = f.start(move |outcome| {
            counted.fetch_add(1, Ordering::SeqCst);
            if outcome.configuration_may_be_recorded {
                observe(&mut failed, outcome)
            } else {
                observe(&mut witness, outcome)
            }
        });
        let (csrf, raw) = f.prepare();
        let response = f.exchange("/api/v1/native-settings/apply", Some(&raw), Some(&csrf));
        if let Ok(raw) = response {
            assert!(!raw.starts_with("HTTP/1.1 200"));
        }
        let error = handle
            .join()
            .unwrap()
            .err()
            .expect("observer failure must stop server");
        assert_eq!(error.code(), "web-outcome-observation-failed");
        assert!(error.session_state_may_be_recorded());
        assert!(error.configuration_may_be_recorded());
        let receipt = error
            .recorded_configuration()
            .expect("actual apply receipt lost");
        assert_eq!(receipt.event_id, "observer-tcp96");
        assert_eq!(receipt.event_revision, SettingCount(2));
        assert_eq!(receipt.current_revision, SettingCount(2));
        assert!(!receipt.replayed);

        assert!(error.recorded_work().is_none());
        let pending = error.observation_failure().unwrap();
        assert_eq!(pending.pending.len(), 1);
        assert!(pending.pending[0].configuration_may_be_recorded);
        assert_eq!(pending.pending[0].status, http::StatusCode::OK);
        assert!(
            matches!(&pending.cause,NativeObservationCause::Io(e) if e.kind()==io::ErrorKind::BrokenPipe)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(!format!("{error:?}").contains(&f.key));
        f.verify_effect();
        let saved = f.saved();
        assert!(TcpStream::connect(f.address).is_err());
        f.restart();
        let mut records = Vec::new();
        let (stop, handle) = f.start(move |outcome| observe(&mut records, outcome));
        let session = f.json("/api/v1/native-session", None, None);
        let csrf = session["csrf_token"].as_str().unwrap();
        let replay = f.json("/api/v1/native-settings/apply", Some(&raw), Some(csrf));
        assert_eq!(replay["replayed"], true);
        assert_eq!(f.saved(), saved);
        stop.send(()).unwrap();
        let report = handle.join().unwrap().unwrap();
        assert_eq!(report.configuration_outcomes, 1);
        assert_eq!(report.observed_outcomes, 2);
        f.verify_effect();
    }
}
/// A real client is closed after Root publication, before observer acknowledgement
/// and therefore before any final HTTP reply. The owner already holds the safe
/// record; shutdown drains, preserves effect and never retries the Root operation.
#[test]
fn native_tcp_client_disconnect_after_commit_keeps_owner_outcome_and_saved_effect() {
    let mut f = Fixture::new();
    let (signal, received) = mpsc::channel();
    let (release, resume) = mpsc::channel();
    let (stop, handle) = f.start(move |outcome| {
        if outcome.configuration_may_be_recorded {
            signal
                .send(serde_json::to_value(outcome.record()).map_err(io::Error::other)?)
                .map_err(io::Error::other)?;
            resume.recv().map_err(io::Error::other)?;
        }
        Ok(())
    });
    let (csrf, raw) = f.prepare();
    let mut client = TcpStream::connect(f.address).unwrap();
    client
        .write_all(
            f.frame("/api/v1/native-settings/apply", Some(&raw), Some(&csrf))
                .as_bytes(),
        )
        .unwrap();
    client.flush().unwrap();
    eprintln!("OWNED96 waiting for post-commit observation barrier");
    let event = received.recv().unwrap();
    assert_eq!(event["configuration_may_be_recorded"], true);
    assert_eq!(event["status"], 200);
    assert_eq!(
        event["recorded_configuration"]["event_id"],
        "observer-tcp96"
    );
    assert_eq!(event["recorded_configuration"]["event_revision"], "2");

    f.verify_effect();
    let saved = f.saved();
    client.shutdown(Shutdown::Both).unwrap();
    drop(client);
    release.send(()).unwrap();
    stop.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.completed_jobs, 3);
    assert_eq!(report.observed_outcomes, 3);
    assert_eq!(report.configuration_outcomes, 1);
    assert_eq!(f.saved(), saved);
    f.verify_effect();
    eprintln!("OWNED96 post-commit disconnect observed and owner drained");
}

/// Actual TCP task update uses the prepared Work, overwrites forged inner fields,
/// preserves revision/event history on replay and hands the real Work effect to owner.
/// Manual update remains distinct from worker completion or task acceptance.
#[test]
fn native_tcp_task_action_observes_recorded_work_and_replays_without_forged_origin() {
    let mut f = Fixture::new();
    let mut r = RootStore::open(&f.root, true).unwrap();
    let work=r.execute_external("","work.create",br#"{"schema_version":1,"event_id":"create","expected_revision":0,"title":"Owned TCP Work","objective":"Verify observer effects","scope":"Fixture","criteria":["Saved state"]}"#,&mut SystemEnvironment).unwrap().result.work.id;
    let add = serde_json::json!({"schema_version":1,"event_id":"add","expected_revision":1,"id":"task","title":"Original","deliverable":"Report","criteria":["Proof"]});
    r.execute_external(
        &work,
        "task.add",
        &serde_json::to_vec(&add).unwrap(),
        &mut SystemEnvironment,
    )
    .unwrap();
    r.close().unwrap();
    let (events, received) = mpsc::channel();
    let (stop, handle) = f.start(move |outcome| {
        events
            .send(serde_json::to_value(outcome.record()).map_err(io::Error::other)?)
            .map_err(io::Error::other)
    });
    let session = f.json("/api/v1/native-session", None, None);
    let csrf = session["csrf_token"].as_str().unwrap();
    let action = serde_json::json!({"kind":"task","work":work,"task":"task","event_id":"tcp-task-change","expected_revision":2,"request":{"title":"Changed over TCP","id":"forged","event_id":"forged","expected_revision":999,"schema_version":99,"origin":"engine"}});
    let raw = serde_json::to_string(&action).unwrap();
    let result = f.json("/api/v1/action", Some(&raw), Some(csrf));
    assert_eq!(result["revision"], 3);
    assert_eq!(result["tasks"][0]["id"], "task");
    assert_eq!(result["tasks"][0]["title"], "Changed over TCP");
    assert_eq!(result["tasks"][0]["status"], "todo");
    let r = RootStore::open(&f.root, false).unwrap();
    let saved = (r.read_work(&work).unwrap(), r.events(&work).unwrap());
    assert_eq!(saved.1.len(), 3);
    r.close().unwrap();
    let replay = f.json("/api/v1/action", Some(&raw), Some(csrf));
    assert_eq!(replay, result);
    stop.send(()).unwrap();
    let report = handle.join().unwrap().unwrap();
    assert_eq!(report.work_outcomes, 2);
    assert_eq!(report.configuration_outcomes, 0);
    let records: Vec<serde_json::Value> = received.into_iter().collect();
    assert_eq!(records.len(), 3);
    for item in &records[1..] {
        assert_eq!(item["recorded_work"]["work"], work);
        assert_eq!(item["recorded_work"]["revision"], 3);
        assert_eq!(item["configuration_may_be_recorded"], false);
    }
    let r = RootStore::open(&f.root, false).unwrap();
    assert_eq!(
        (r.read_work(&work).unwrap(), r.events(&work).unwrap()),
        saved
    );
    assert_eq!(r.settings().unwrap().revision, SettingCount(1));
    r.close().unwrap();
}
