//! Owned-fixture-only syscall discriminator; no authority or publication.
use std::{path::Path,os::unix::process::CommandExt};
fn main(){
 let a:Vec<_>=std::env::args().collect();assert_eq!(a.len(),4);
 let profile=swarm_biba_linux::Sandbox::prepare(Path::new(&a[1]),Path::new(&a[2])).unwrap();
 profile.enforce().unwrap();
 let error=std::process::Command::new("/usr/bin/python3").arg(&a[3]).exec();
 panic!("owned fixture exec refused: {error}")
}
