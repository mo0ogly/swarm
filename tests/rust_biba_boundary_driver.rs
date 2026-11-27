//! Owned-fixture-only syscall discriminator; no authority or publication.
use std::{fs::File,path::Path,os::unix::{fs::MetadataExt,process::CommandExt}};
fn main(){
 let a:Vec<_>=std::env::args().collect();assert!(a.len()>=4);
 let root=Path::new(&a[1]);
 let workspace=Path::new(&a[2]);
 let profile=if a.len()==4 {
    swarm_biba_linux::Sandbox::prepare(root,workspace)
 } else {
    let files:Vec<_>=a[4..].iter().map(|name| File::open(workspace.join(name)).unwrap()).collect();
    let metadata=workspace.metadata().unwrap();
    swarm_biba_linux::Sandbox::prepare_readonly_inputs(root,workspace,(metadata.dev(),metadata.ino()),&files)
 }.unwrap();
 profile.enforce().unwrap();
 let error=std::process::Command::new("/usr/bin/python3").arg(&a[3]).exec();
 panic!("owned fixture exec refused: {error}")
}
