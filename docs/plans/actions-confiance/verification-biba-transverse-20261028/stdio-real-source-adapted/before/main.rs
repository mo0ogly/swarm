pub use swarm_engine::{Provider,SettingCount};
#[path="agent_launch_error.rs"]mod agent_launch_error;
pub use agent_launch_error::AgentLaunchError;
// Unexercised sparse Root error adapter; no error-continuity qualification.
#[derive(Debug)]pub enum RootStoreError{Launch(Box<AgentLaunchError>),Refused(&'static str),Cleanup{primary:Option<Box<Self>>,cleanup:Box<Self>}}
mod root_store {use super::RootStoreError as E;pub fn finished<T>(v:Result<T,E>,c:Result<(),E>)->Result<T,E>{match(v,c){(Ok(v),Ok(()))=>Ok(v),(Err(e),Ok(()))=>Err(e),(Ok(_),Err(e))=>Err(E::Cleanup{primary:None,cleanup:Box::new(e)}),(Err(p),Err(c))=>Err(E::Cleanup{primary:Some(Box::new(p)),cleanup:Box::new(c)})}}}
// This empty-environment fixture never exercises environment validation.
mod agent_launch_policy {pub fn environment_name(_: &str)->bool{panic!("outside fixture scope")}}
#[path="transport.rs"]mod transport;
fn main(){transport::closed_stdin_probe()}
