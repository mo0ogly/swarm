#![cfg(target_os = "linux")]
//! Actual typed settings publication with owned observer output failure doubles.
//! This does not establish TCP mutation delivery or persistent incident routing.
use http::{Request, StatusCode};
use std::{
    fs,
    io::{self, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
};
use swarm_engine::{EngineSettingsChange, RootStore, SettingCap, SettingCount, SystemEnvironment};
use swarm_web::{LocalWebAuthority, NativeHttpService, NativeJobOutcome, NativeObservationCause};
/// Original output failure at write or flush, without arbitrary deadlines/retries.
struct BrokenSink {
    flush_failure: bool,
    bytes: Vec<u8>,
}
impl Write for BrokenSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.flush_failure {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "owned observer write",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "owned observer flush",
        ))
    }
}
/// Observer writes an explicit safe record then flushes. Acknowledgement means
/// this local callback completed, never durable storage or receipt by an HTTP peer.
fn write_record(sink: &mut dyn Write, outcome: &NativeJobOutcome) -> io::Result<()> {
    let raw = serde_json::to_vec(&outcome.record()).map_err(io::Error::other)?;
    sink.write_all(&raw)?;
    sink.flush()
}
/// A real private Root and actual apply/replay precede failing owner outputs.
/// Known publication and original IO cause must survive, with no automatic retry.
#[test]
fn native_observer_write_and_flush_failure_retain_actual_settings_publication() {
    for flush_failure in [false, true] {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("observer96-").unwrap());
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let r = RootStore::initialize(&p, &mut SystemEnvironment).unwrap();
        let document = r.settings().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let key = r.web_session(&listener).unwrap().secret().to_owned();
        r.close().unwrap();
        let a = LocalWebAuthority::from_listener(&listener, &key).unwrap();
        let s = NativeHttpService::open_local(&p, &listener).unwrap();
        let request = |path: &str, raw: Vec<u8>| {
            Request::builder()
                .method("POST")
                .uri(path)
                .header("Host", a.host())
                .header("Cookie", format!("{}={}", a.cookie_name(), key))
                .header("Origin", format!("http://{}", a.host()))
                .header("X-Swarm-CSRF", a.csrf_token())
                .body(raw)
                .unwrap()
        };
        let mut values = document.values;
        values.review_delivery_max_bytes = Some(SettingCap(Some(SettingCount(3))));
        let mut change = EngineSettingsChange {
            schema_version: document.schema_version,
            event_id: "observer-settings".into(),
            expected_revision: document.revision,
            values,
            reason: "Owned outcome observation fixture".into(),
            preview_token: String::new(),
        };
        let before = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
        let preview = s
            .handle(request(
                "/api/v1/native-settings/preview",
                serde_json::to_vec(&change).unwrap(),
            ))
            .unwrap();
        assert_eq!(preview.response.status(), StatusCode::OK);
        assert_eq!(
            fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
            before
        );
        let (preview, response) = NativeJobOutcome::from_reply(preview);
        let mut witness = Vec::new();
        let preview = preview
            .observe(&mut |o| write_record(&mut witness, o))
            .unwrap();
        assert!(!preview.configuration_may_be_recorded);
        change.preview_token = serde_json::from_slice::<serde_json::Value>(response.body())
            .unwrap()["preview_token"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(!String::from_utf8(witness).unwrap().contains(&key));
        let raw = serde_json::to_vec(&change).unwrap();
        let reply = s
            .handle(request("/api/v1/native-settings/apply", raw.clone()))
            .unwrap();
        assert_eq!(reply.response.status(), StatusCode::OK);
        let (outcome, _response) = NativeJobOutcome::from_reply(reply);
        assert!(outcome.configuration_may_be_recorded);
        let mut sink = BrokenSink {
            flush_failure,
            bytes: Vec::new(),
        };
        let failure = outcome
            .observe(&mut |o| write_record(&mut sink, o))
            .unwrap_err();
        assert!(failure.configuration_may_be_recorded());
        assert!(failure.recorded_work().is_none());
        assert_eq!(failure.pending.len(), 1);
        match &failure.cause {
            NativeObservationCause::Io(e) => assert_eq!(e.kind(), io::ErrorKind::BrokenPipe),
            _ => panic!("IO cause lost"),
        };
        assert!(failure.pending[0].configuration_may_be_recorded);
        assert_eq!(failure.pending[0].status, StatusCode::OK);
        assert!(!format!("{failure:?}").contains(&key));
        let saved = fs::read(p.join(".swarm/engine-settings.json")).unwrap();
        assert_ne!(saved, before);
        let r = RootStore::open(&p, false).unwrap();
        assert_eq!(r.settings().unwrap().revision, SettingCount(2));
        assert!(r.list_works().unwrap().is_empty());
        r.close().unwrap();
        let replay = s
            .handle(request("/api/v1/native-settings/apply", raw))
            .unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(replay.response.body()).unwrap()
                ["replayed"]
                .as_bool()
                .unwrap()
        );
        let (replay, _) = NativeJobOutcome::from_reply(replay);
        let mut recovered = Vec::new();
        let replay = replay
            .observe(&mut |o| write_record(&mut recovered, o))
            .unwrap();
        assert!(replay.configuration_may_be_recorded);
        assert_eq!(
            fs::read(p.join(".swarm/engine-settings.json")).unwrap(),
            saved
        );
        s.close().unwrap();
        drop(listener);
        fs::remove_dir_all(p).unwrap();
    }
}
