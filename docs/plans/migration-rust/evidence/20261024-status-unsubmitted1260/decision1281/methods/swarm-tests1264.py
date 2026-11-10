from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic,edit
p=Path('rust/crates/swarm-engine/src/root_store/mission_status_reports.rs');s=p.read_text();assert s.endswith('}\n')
extra='''    /// Real filesystem control precedes stale/empty/ambiguous/invalid variants.
    /// Boundary equality uses an explicit test margin, never a product default.
    #[test]
    fn mission_status_unsubmitted_report_selection_and_recovery() {
        use crate::mission_status_tasks::ReportAttribution::{Attributed, Unavailable};
        let fixture = Fixture::new();
        let mut agent = Agent { task_id: "task".into(), started: "1970-01-01T00:01:40Z".into(), ..Default::default() };
        let mut causes = Vec::new();
        assert!(matches!(proven_report(&fixture.0,&agent,2,&mut causes).unwrap(),Unavailable { .. }));
        disk::create_dir(fixture.0.join("docs")).unwrap();
        let path=fixture.0.join("docs/task.md");disk::write(&path,"actual report").unwrap();
        let set_time=|seconds| { disk::File::options().write(true).open(&path).unwrap().set_times(
            disk::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds))).unwrap(); };
        set_time(98);
        assert!(matches!(proven_report(&fixture.0,&agent,2,&mut causes).unwrap(),Attributed(name) if name=="docs/task.md"));
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason.contains("1 rapport(s) antérieur(s)")));
        set_time(100);
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        let second=fixture.0.join("docs/task-other-handoff.md");disk::write(&second,"second").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason.contains("plusieurs rapports") && reason.contains("docs/task.md") && reason.contains("docs/task-other-handoff.md")));
        disk::remove_file(second).unwrap();disk::write(&path,"").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason=="aucun rapport lisible et non vide sous docs/ pour cette tâche"));
        disk::write(&path,"restored").unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        assert!(causes.is_empty());
        assert!(proven_report(&fixture.0,&agent,-1,&mut causes).is_err());
        agent.started="not-a-date".into();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Unavailable{reason} if reason=="date de départ de la tentative illisible"));
        assert_eq!(causes.len(),1);assert!(std::error::Error::source(&causes[0]).is_some());
    }
    /// A contained genuine candidate precedes an outside-root link refusal;
    /// restoring its exact owned path reestablishes the read-only attribution.
    #[test]
    fn mission_status_unsubmitted_report_path_error_is_not_absence() {
        use crate::mission_status_tasks::ReportAttribution::Attributed;
        let fixture=Fixture::new();let foreign=Fixture::new();
        disk::create_dir(fixture.0.join("docs")).unwrap();
        let path=fixture.0.join("docs/task.md");disk::write(&path,"owned").unwrap();
        let agent=Agent { task_id:"task".into(), started:"1970-01-01T00:00:00Z".into(), ..Default::default() };
        let mut causes=Vec::new();assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
        let original=disk::read(&path).unwrap();disk::write(foreign.0.join("report"),"foreign").unwrap();
        disk::remove_file(&path).unwrap();std::os::unix::fs::symlink(foreign.0.join("report"),&path).unwrap();
        assert!(proven_report(&fixture.0,&agent,0,&mut causes).is_err());
        disk::remove_file(&path).unwrap();disk::write(&path,original).unwrap();
        assert!(matches!(proven_report(&fixture.0,&agent,0,&mut causes).unwrap(),Attributed(_)));
    }
'''
atomic(p,s[:-2]+extra+'}\n')
p='tests/rust_mission_worker_recovery.py'
edit(p,"t['task']['id']=='task'","t['id']=='task'")
