//! Owned-fixture-only Internet-refusal discriminator; no authority or publication.
//! Poses the mandatory profile THEN the additive engine policy, mirroring the
//! production exec point's order, and executes the supplied python fixture.
use std::{path::Path,os::unix::process::CommandExt};
fn main(){
 let a:Vec<_>=std::env::args().collect();assert_eq!(a.len(),4);
 let profile=swarm_biba_linux::Sandbox::prepare(Path::new(&a[1]),Path::new(&a[2])).unwrap();
 profile.enforce().unwrap();
 swarm_biba_linux::deny_internet().unwrap();
 let error=std::process::Command::new("/usr/bin/python3").arg(&a[3]).exec();
 panic!("owned fixture exec refused: {error}")
}
