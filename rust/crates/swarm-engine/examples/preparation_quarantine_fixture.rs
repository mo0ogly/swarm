//! Test-only pending-turn producer/dispatcher. Never a product command or an
//! authenticated actor. Only marked direct children of the explicit fixture
//! area are admitted; no arbitrary project path or private SQL mutation.
use sha2::{Digest, Sha256};
use std::{io::Read, path::{Path, PathBuf}};
use swarm_engine::{RootStore, RootStoreError as E, SystemEnvironment};

/// Pin the production worker image before forwarding its anonymous stdin gate.
/// Environment names contain fixture paths/hashes, never account credentials.
/// Hashes establish bytes, not qualification; Python checks the normal profile.
fn production_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path=PathBuf::from(std::env::var_os("SWARM_BLP_BINARY").ok_or("fixture binary missing")?).canonicalize()?;
    let expected=std::env::var("SWARM_BLP_BINARY_SHA256")?;
    let mut file=std::fs::File::open(&path)?;let mut hash=Sha256::new();let mut buffer=[0u8;65536];
    loop { let n=file.read(&mut buffer)?;if n==0 {break;}hash.update(&buffer[..n]); }
    if format!("{:x}",hash.finalize())!=expected { return Err("fixture production image changed".into()); }
    Ok(path)
}

/// Require an owned-fixture locator before any Root opening. The marker does
/// not authenticate an actor or provide confinement against a malicious owner.
fn fixture_root(path:&Path)->Result<PathBuf,Box<dyn std::error::Error>> {
    let area=PathBuf::from(std::env::var_os("SWARM_BLP_FIXTURE_AREA").ok_or("fixture area missing")?).canonicalize()?;
    let root=path.canonicalize()?;
    if area.starts_with("/tmp") || root.parent()!=Some(area.as_path())
        || !root.file_name().and_then(|s|s.to_str()).is_some_and(|s|s.starts_with("swarm-model83-"))
        || std::fs::read(root.join(".blp-quarantine-fixture"))?!=b"owned inert quarantine fixture\n"
    {return Err("fixture root refused".into());}
    Ok(root)
}

/// Preserve primary/close causes using the engine wrapper; success alone never
/// establishes that a provider ran or an operation was independently accepted.
fn closed<T>(result:Result<T,E>,close:Result<(),E>)->Result<T,E> {
    match (result,close) {
        (value,Ok(()))=>value,
        (Ok(_),Err(cleanup))=>Err(E::Cleanup{primary:Some(Box::new(E::Refused("fixture-operation-completed"))),cleanup:Box::new(cleanup)}),
        (Err(primary),Err(cleanup))=>Err(E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cleanup)}),
    }
}

/// Refuse sidecar diagnostics instead of dropping unavailable-method causes.
fn observed<T>(read:swarm_engine::PreparationReadObservation<T>)->Result<T,E> {
    let mut causes=read.causes.into_iter();
    if let Some(mut primary)=causes.next() {
        for cleanup in causes {primary=E::Cleanup{primary:Some(Box::new(primary)),cleanup:Box::new(cleanup)};}
        Err(primary)
    } else {Ok(read.value)}
}

/// Stage through the actual public engine producer, or dispatch an already
/// pending turn and observe its terminal state. Child _prepare_turn execs the
/// qualified production image, retaining PID/starttime/stdin; it mints no gate.
/// Polling cadence is test observation, without a whole-journey watchdog.
fn main()->Result<(),Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s|s=="_prepare_turn") {
        use std::os::unix::process::CommandExt;
        let index=args.iter().position(|s|s=="--root").ok_or("fixture child root missing")?;
        fixture_root(Path::new(args.get(index+1).ok_or("fixture child root missing")?))?;
        return Err(std::process::Command::new(production_binary()?).args(&args).exec().into());
    }
    if args.len()!=3 {return Err("fixture requires stage|dispatch root input|turn".into());}
    let root=fixture_root(Path::new(&args[1]))?;production_binary()?;
    let store=RootStore::open(&root,true)?;
    let result=(||->Result<serde_json::Value,E>{
        if args[0]=="stage" {
            let raw=swarm_engine::read_command_file(Path::new(&args[2]))
                .map_err(|e|swarm_engine::preparation_worker_error("fixture-stage-input-invalid",e))?;
            let turn=store.preparation_send(&raw,&mut SystemEnvironment,None)?;
            Ok(serde_json::json!({"id":turn.id,"preparation_id":turn.preparation_id,"status":turn.status}))
        } else if args[0]=="dispatch" {
            let id=args[2].to_str().ok_or(E::Refused("fixture-turn-invalid"))?;
            // Read the parent from the public list, never private SQLite.
            let parents=observed(store.preparation_list(None)?)?.preparations;
            let mut parent=None;
            for p in parents {if observed(store.preparation_dialogue(&p.id,None)?)?.iter().any(|t|t.id==id){parent=Some(p.id);break;}}
            let parent=parent.ok_or(E::Refused("fixture-turn-not-found"))?;
            store.preparation_dispatch(id)?;
            loop {
                let turns=observed(store.preparation_dialogue(&parent,None)?)?;
                let turn=turns.into_iter().find(|t|t.id==id).ok_or(E::Refused("fixture-turn-not-found"))?;
                if !turn.active() {break Ok(serde_json::json!({"id":turn.id,"status":turn.status,"error":turn.error}));}
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        } else {Err(E::Refused("fixture-operation-invalid"))}
    })();
    let value=closed(result,store.close())?;println!("{}",serde_json::to_string(&value)?);Ok(())
}
