// Controlled actual artifact-reader regression, never a live project or proof.
use std::{fs, io::Read, os::unix::fs::symlink, path::PathBuf};
fn main() {
    let root=PathBuf::from(std::env::args_os().nth(1).expect("owned fixture root"));
    fs::create_dir_all(root.join("docs/parent")).unwrap();
    fs::create_dir_all(root.join(".swarm")).unwrap();
    fs::write(root.join("docs/parent/report.md"),b"allowed fixture").unwrap();
    fs::write(root.join(".swarm/report.md"),b"private fixture").unwrap();
    let read=|file:&mut fs::File| {
        let mut raw=Vec::new();file.read_to_end(&mut raw).expect("healthy controlled fixture read");Ok(raw)
    };
    let value=gate_evaluation::artifact_access_before_open(&root,"docs/parent/report.md", |_|{},read).unwrap().1;
    assert_eq!(value,b"allowed fixture");println!("PASS actual-reader-valid-control");
    let fault=gate_evaluation::artifact_access_before_open(&root,"docs/parent/report.md", |_|{
        fs::rename(root.join("docs/parent"),root.join("docs/original-parent")).unwrap();
        symlink(root.join(".swarm"),root.join("docs/parent")).unwrap();
    },read);
    let rejected=fault.is_err();
    if let Ok((_,bytes))=&fault {assert_eq!(bytes,b"private fixture");println!("OBSERVED private-fixture-read-after-parent-swap");}
    fs::remove_file(root.join("docs/parent")).unwrap();
    fs::rename(root.join("docs/original-parent"),root.join("docs/parent")).unwrap();
    let recovered=gate_evaluation::artifact_access_before_open(&root,"docs/parent/report.md", |_|{},read).unwrap().1;
    assert_eq!(recovered,b"allowed fixture");println!("PASS actual-reader-recovery");
    assert!(rejected,"actual artifact reader followed a replaced ancestor into private fixture data");
    println!("PASS actual-reader-parent-swap-refusal");
}
