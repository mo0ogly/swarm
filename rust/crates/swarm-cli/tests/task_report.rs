#![cfg(target_os = "linux")]
#[path = "../../swarm-engine/tests/support/task_report.rs"]
mod support;
use std::io::{self, Write};
use swarm_engine::{RootStore, SystemEnvironment};
/// Output stage double only; private causes must survive internally and stay out of stderr.
struct Denied(bool);
impl Write for Denied {
    /// Either refuse write or accept bytes to reach the explicit flush fault.
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.0 {
            Ok(b.len())
        } else {
            Err(io::Error::other("PRIVATE sink"))
        }
    }
    /// Fail only the flush case after a committed submission.
    fn flush(&mut self) -> io::Result<()> {
        if self.0 {
            Err(io::Error::other("PRIVATE sink"))
        } else {
            Ok(())
        }
    }
}
/// The real CLI commits once before output; write/flush refusal reports preserved
/// state. Identical public replay succeeds without another attempt/event/receipt.
#[test]
fn task_report_output_faults_preserve_submission_and_replay() {
    let control = support::Fixture::new();
    let input = control.root.join("request.json");
    std::fs::write(&input, control.request("submit")).unwrap();
    let args = |f: &support::Fixture| {
        vec![
            "task".into(),
            "submit".into(),
            f.work.clone(),
            "--root".into(),
            f.root.to_str().unwrap().into(),
            "--input".into(),
            f.root.join("request.json").to_str().unwrap().into(),
            "--json".into(),
        ]
    };
    assert_eq!(
        swarm_cli::run_detailed(&args(&control), "en", &mut vec![], &mut vec![]).code,
        0
    );
    for flush in [false, true] {
        let f = support::Fixture::new();
        std::fs::write(f.root.join("request.json"), f.request("submit")).unwrap();
        let s = RootStore::open(&f.root, false).unwrap();
        let before = s.read_work(&f.work).unwrap();
        s.close().unwrap();
        let mut stderr = vec![];
        let outcome = swarm_cli::run_detailed(&args(&f), "en", &mut Denied(flush), &mut stderr);
        assert_eq!(outcome.code, 2);
        assert!(matches!(
            outcome.error,
            Some(swarm_cli::CliError::StatePreserved { .. })
        ));
        assert!(!String::from_utf8_lossy(&stderr).contains("PRIVATE"));
        let mut s = RootStore::open(&f.root, true).unwrap();
        let work = s.read_work(&f.work).unwrap();
        assert_eq!(work.revision, 3);
        assert_eq!(work.tasks.as_ref().unwrap()[0].status, "submitted");
        assert_eq!(
            work.tasks.as_ref().unwrap()[0].attempts,
            before.tasks.as_ref().unwrap()[0].attempts
        );
        let events = s.events(&f.work).unwrap().len();
        assert!(
            s.execute_external(
                &f.work,
                "task.submit",
                &f.request("submit"),
                &mut SystemEnvironment
            )
            .unwrap()
            .result
            .replayed
        );
        assert_eq!(s.events(&f.work).unwrap().len(), events);
        s.close().unwrap();
    }
}
