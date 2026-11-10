//! Owned libtest reproducer: real reviewer claim/controller/grant, then DB-only swap.
//! No provider executes: the child calls the actual admission reader and drops
//! its material. This is not the normal CLI or proof of sealed provider exec.
use super::{claim, gate, process};
use crate::{root_store::{task_validation, RootStore}, RootStoreError};
use serde::Deserialize;
use std::{error::Error, fmt, fs, io::{self, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}, process::Command};

type Cause = Box<dyn Error + Send + Sync>;
type Result<T> = std::result::Result<T, Cause>;

/// Independent fixture/restore/close failures remain owned, not flattened.
#[derive(Debug)]
struct Failures(Vec<Cause>);
impl fmt::Display for Failures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "review root identity SDK failed in {} phase(s)", self.0.len())
    }
}
impl Error for Failures {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    control: Fixture,
    attack: Fixture,
    output: PathBuf,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    root: PathBuf,
    work: String,
    request: serde_json::Value,
    token: String,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct ChildInput {
    root: PathBuf,
    job: String,
    token: String,
    barrier: PathBuf,
    result: PathBuf,
}

/// Assert only owned fixture preconditions; tokens isolate tests, not authority.
fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok { Ok(()) } else { Err(io::Error::other(message).into()) }
}

/// Observe the real original grant without consuming or replacing it. The
/// independent barrier is reached before gate::admit performs its first Root
/// open. Peer/process/permission checks still run in the real admission reader.
#[test]
#[ignore = "private child; select only from the owned root identity SDK parent"]
fn reviewer_root_identity_child() -> Result<()> {
    let path = std::env::var_os("SWARM_REVIEW_ROOT_IDENTITY_CHILD")
        .ok_or_else(|| io::Error::other("owned child input required"))?;
    let input: ChildInput = serde_json::from_slice(&fs::read(path)?)?;
    require(fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture"))? == input.token,
        "owned reviewer marker mismatch")?;
    let descriptor = rustix::io::fcntl_dupfd_cloexec(std::io::stdin(), 3)?;
    let mut channel = UnixStream::from(descriptor);
    let mut byte = [0_u8; 1];
    let (_, length) = rustix::net::recv(&channel, &mut byte[..], rustix::net::RecvFlags::PEEK)?;
    require(length == 1 && byte == [1], "original committed grant required")?;
    let mut barrier = UnixStream::connect(&input.barrier)?;
    barrier.write_all(b"P")?;
    let mut resume = [0_u8; 1];
    barrier.read_exact(&mut resume)?;
    require(resume == *b"R", "explicit post-grant resume required")?;
    nix::unistd::close(barrier)?;
    let admission = gate::admit(&input.root, &input.job, &mut channel);
    let (admitted, diagnostic) = match admission {
        Ok(material) => { drop(material); (true, None) },
        Err(error) => (false, Some(crate::agent_execution_gate::diagnostic(&error))),
    };
    fs::write(&input.result, serde_json::to_vec(&serde_json::json!({
        "admitted":admitted, "actual_gate_admission":true, "provider_executed":false,
        "original_grant_peeked_not_consumed":true, "diagnostic":diagnostic
    }))?)?;
    nix::unistd::close(channel)?;
    Ok(())
}

/// A separate Python process owns SQLite backup/read descriptors: it cannot
/// release the parent's POSIX locks by closing a same-process SQLite data FD.
/// No authority rows are inserted/updated. On finish, restoration occurs even
/// when snapshot comparison fails; all independent failures remain grouped.
const DATABASE_HELPER: &str = r#"
import json,os,sqlite3,stat,sys
from pathlib import Path
root,out,action,job,child,attack=sys.argv[1:]
root=Path(root);out=Path(out);directory=root/'.swarm';db=directory/'state.db'
held=directory/'review-identity-original.db';clone=directory/'review-identity-clone.db'
substituted=directory/'review-identity-substituted.db';attack=attack=='true'
def checked(path):
 s=path.lstat()
 assert stat.S_ISREG(s.st_mode) and s.st_uid==os.getuid() and s.st_nlink==1 and s.st_mode&0o077==0
 return (s.st_dev,s.st_ino)
def volatile_absent():
 for name in ['state.db-wal','state.db-shm','state.db-journal']:
  assert not (directory/name).exists(), 'volatile sibling prevents this fixture'
def snapshot(path):
 checked(path);connection=sqlite3.connect('file:'+str(path)+'?mode=ro',uri=True)
 try:
  connection.execute('BEGIN');connection.execute('SELECT count(*) FROM sqlite_schema').fetchone()
  return '\n'.join(connection.iterdump()).encode()
 finally:connection.close()
def publish(name,data):
 with (out/name).open('xb') as f:f.write(data);f.flush();os.fsync(f.fileno())
def syncdir():
 fd=os.open(directory,os.O_RDONLY|os.O_DIRECTORY)
 try:os.fsync(fd)
 finally:os.close(fd)
if action=='prepare':
 volatile_absent();identity=checked(db)
 source=sqlite3.connect('file:'+str(db)+'?mode=ro',uri=True);target=None
 try:
  source.execute('BEGIN');source.execute('SELECT count(*) FROM sqlite_schema').fetchone()
  row=source.execute('SELECT phase,permission,child_pid,owner_pid FROM native_review_jobs WHERE id=?',(job,)).fetchone()
  assert row==('permitted',1,int(child),os.getppid()), 'actual controller permission/child required'
  original='\n'.join(source.iterdump()).encode();publish('before.sql',original)
  if attack:
   fd=os.open(clone,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600);os.close(fd)
   target=sqlite3.connect(clone);source.backup(target)
 finally:
  pending=sys.exception();failures=[]
  for connection in [target,source]:
   if connection is not None:
    try:connection.close()
    except BaseException as error:failures.append(error)
  if failures:raise BaseExceptionGroup('backup/close',([pending] if pending else [])+failures)
 if attack:
  assert snapshot(clone)==original
  assert checked(clone)!=identity and not held.exists() and not substituted.exists()
  os.rename(db,held);os.rename(clone,db);syncdir()
 publish('prepared.json',json.dumps({'attack':attack,'original_identity':identity,'connected_path_identity':checked(db),'directory_identity':(directory.stat().st_dev,directory.stat().st_ino),'actual_permission':1,'actual_child':int(child)}).encode())
elif action=='finish':
 failures=[]
 try:
  volatile_absent()
  if (out/'before.sql').exists():
   before=(out/'before.sql').read_bytes();current=snapshot(db);publish('after-path.sql',current)
   if current!=before:failures.append(AssertionError('path database SQL changed'))
   original=snapshot(held if held.exists() else db);publish('after-original.sql',original)
   if original!=before:failures.append(AssertionError('original database SQL changed'))
 except BaseException as error:failures.append(error)
 finally:
  if held.exists():
   try:
    if db.exists():os.rename(db,substituted)
    os.rename(held,db);syncdir()
   except BaseException as error:failures.append(error)
 if failures:raise BaseExceptionGroup('snapshot/restore',failures)
 restored=checked(db)
 if (out/'prepared.json').exists():
  assert list(restored)==json.loads((out/'prepared.json').read_bytes())['original_identity'], 'original inode not restored'
 publish('restored.json',json.dumps({'restored':True,'original_identity':restored,'provider_executed_by_sdk':False}).encode())
else:raise ValueError('unknown helper action')
"#;

/// Retain exact helper argv and both original channels before checking its exit.
/// This is an explicit fixture process, never an application retry or watchdog.
fn database_helper(root: &Path, out: &Path, action: &str, job: &str, child: u32, attack: bool) -> Result<()> {
    let script = out.join("database-helper.py");
    let argv = [script.as_os_str(), root.as_os_str(), out.as_os_str(), std::ffi::OsStr::new(action),
        std::ffi::OsStr::new(job)];
    let output = Command::new("python3").args(argv).arg(child.to_string()).arg(attack.to_string()).output()?;
    fs::write(out.join(format!("{action}.stdout")), &output.stdout)?;
    fs::write(out.join(format!("{action}.stderr")), &output.stderr)?;
    fs::write(out.join(format!("{action}.command.json")), serde_json::to_vec(&serde_json::json!({
        "program":"python3", "script":script, "root":root, "output":out, "action":action,
        "job":job, "child":child, "attack":attack, "exit_success":output.status.success(), "global_watchdog":null
    }))?)?;
    require(output.status.success(), "database fixture helper failed; original channels retained")
}

/// Wait for either the actual child's barrier or explicit parent cancellation.
/// A failed controller that never releases grant cannot strand a helper thread.
/// No elapsed-time bound or repeated polling sleep is introduced.
/// Shared raw credential validation rejects PID0/invalid UID-GID before peer fields select the child.
fn barrier_worker(listener: UnixListener, cancel: UnixStream, root: PathBuf, out: PathBuf, job: String, attack: bool) -> Result<()> {
    let mut peer = None;
    let result = (|| -> Result<()> {
        {
            let mut fds = [rustix::event::PollFd::new(&listener, rustix::event::PollFlags::IN),
                rustix::event::PollFd::new(&cancel, rustix::event::PollFlags::IN)];
            loop {
                match rustix::event::poll(&mut fds, None) {
                    Err(rustix::io::Errno::INTR) => continue,
                    other => { other?; break; },
                }
            }
            require(fds[0].revents().contains(rustix::event::PollFlags::IN),
                "controller ended before post-grant barrier")?;
        }
        peer = Some(listener.accept()?.0);
        let connection = peer.as_mut().ok_or_else(|| io::Error::other("barrier connection missing"))?;
        let mut ready = [0_u8; 1];
        connection.read_exact(&mut ready)?;
        require(ready == *b"P", "post-grant barrier frame required")?;
        let credentials = crate::linux_peer_credentials::read(&*connection)?;
        require(credentials.uid() == rustix::process::geteuid().as_raw(), "barrier peer uid mismatch")?;
        let child = u32::try_from(credentials.pid())?;
        database_helper(&root, &out, "prepare", &job, child, attack)?;
        connection.write_all(b"R")?;
        Ok(())
    })();
    let mut causes: Vec<Cause> = Vec::new();
    if let Err(error) = result { causes.push(error); }
    if let Some(connection) = peer {
        if let Err(error) = nix::unistd::close(connection) { causes.push(Box::new(error)); }
    }
    if let Err(error) = nix::unistd::close(listener) { causes.push(Box::new(error)); }
    if let Err(error) = nix::unistd::close(cancel) { causes.push(Box::new(error)); }
    if causes.is_empty() { Ok(()) } else { Err(Box::new(Failures(causes))) }
}

/// Produce a real review claim and use the exact production controller. Restore
/// DB and close Root only after the common runner's actual wait has succeeded.
/// Setup/transport errors keep the owned fixture for diagnosis, never fake reap.
fn run_case(input: &Fixture, out: &Path, attack: bool) -> Result<bool> {
    fs::create_dir(out)?;
    require(input.root.is_absolute(), "absolute owned fixture required")?;
    require(fs::read_to_string(input.root.join(".swarm/owned-review-claim-fixture"))? == input.token,
        "owned reviewer marker mismatch")?;
    let root = RootStore::open(&input.root, true)?;
    let mut failures: Vec<Cause> = Vec::new();
    let result = (|| -> Result<bool> {
        let request = task_validation::decode(&crate::encode_model(&input.request)?)?;
        let staged = match claim::record(&root, &input.work, &request)? {
            claim::Recorded::New(staged) => *staged,
            claim::Recorded::Replay(_) => return Err(io::Error::other("fresh review claim required").into()),
        };
        fs::write(out.join("database-helper.py"), DATABASE_HELPER)?;
        let barrier = input.root.join(".swarm/review-identity-barrier.sock");
        let listener = UnixListener::bind(&barrier)?;
        let (cancel, stop) = UnixStream::pair()?;
        let child_input = out.join("child-input.json");
        let child_result = out.join("admission.json");
        fs::write(&child_input, serde_json::to_vec(&ChildInput { root:input.root.clone(), job:staged.job.id.clone(),
            token:input.token.clone(), barrier, result:child_result.clone() })?)?;
        let mut command = Command::new("/proc/self/exe");
        let module = module_path!().split_once("::").ok_or_else(|| io::Error::other("libtest module path required"))?.1;
        let name = format!("{module}::reviewer_root_identity_child");
        command.args(["--exact", &name, "--ignored", "--nocapture"])
            .env("SWARM_REVIEW_ROOT_IDENTITY_CHILD", &child_input);
        fs::write(out.join("child-command.json"), serde_json::to_vec(&serde_json::json!({
            "program":"/proc/self/exe", "test":name, "input":child_input, "profile":"libtest", "production_cli":false
        }))?)?;
        let thread_root = input.root.clone(); let thread_out = out.to_owned(); let job = staged.job.id.clone();
        let worker = std::thread::spawn(move || barrier_worker(listener, cancel, thread_root, thread_out, job, attack));
        let mut stdout = Vec::new(); let mut stderr = Vec::new();
        let observation = process::run(&root, &staged.job, command, &mut stdout, &mut stderr);
        let mut causes: Vec<Cause> = Vec::new();
        if let Err(error) = nix::unistd::close(stop) { causes.push(Box::new(error)); }
        match worker.join() {
            Ok(Ok(())) => {}, Ok(Err(error)) => causes.push(error),
            Err(_) => causes.push(io::Error::other("barrier worker panic").into()),
        }
        // Preserve both streams before any expectation is evaluated.
        for (name, bytes) in [("child.stdout", stdout), ("child.stderr", stderr)] {
            if let Err(error) = fs::write(out.join(name), bytes) { causes.push(Box::new(error)); }
        }
        let observed = match observation { Ok(observed) => observed, Err(error) => {
            causes.push(Box::new(error)); return Err(Box::new(Failures(causes)));
        }};
        let waited = match &observed.process {
            Ok(actual) => !actual.timed_out && !actual.output_drain_timed_out,
            Err(error) => error.reaped_exit_status().is_some()
                && error.unresolved_process_id().is_none() && error.ownership_lost_process_id().is_none(),
        };
        // Never restore under a possibly live child, even to make the fixture green.
        if waited {
            if let Err(error) = database_helper(&input.root, out, "finish", &observed.job.id, 0, attack) { causes.push(error); }
        } else { causes.push(io::Error::other("actual child reap missing; DB retained without restoration").into()); }
        fs::write(out.join("observation.json"), serde_json::to_vec(&serde_json::json!({
            "actual_controller":true,"grant_attempted":observed.grant_attempted,"channel_closed":observed.channel_closed,
            "actual_wait":waited,"permission":observed.job.permission,"child_pid":observed.job.child_pid,
            "duration_expired":observed.duration_expired,"drain_expired":observed.drain_expired,
            "gate_bytes":observed.gate_bytes,"transport_error_count":observed.transport_errors.len(),
            "interruption_code":observed.interruption.as_ref().map(RootStoreError::code),"provider_executed":false
        }))?)?;
        if let Err(error) = observed.process { causes.push(Box::new(error)); }
        causes.extend(observed.transport_errors.into_iter().map(|e| Box::new(e) as Cause));
        if let Some(error) = observed.interruption { causes.push(Box::new(error)); }
        if !observed.grant_attempted || !observed.channel_closed || observed.job.permission != 1 {
            causes.push(io::Error::other("actual granted controller required").into());
        }
        if !causes.is_empty() { return Err(Box::new(Failures(causes))); }
        let admitted: serde_json::Value = serde_json::from_slice(&fs::read(child_result)?)?;
        admitted["admitted"].as_bool().ok_or_else(|| io::Error::other("strict child admission bool required").into())
    })();
    let value = match result { Ok(value) => Some(value), Err(error) => {failures.push(error);None} };
    if let Err(error) = root.close() { failures.push(Box::new(error)); }
    if failures.is_empty() { Ok(value.expect("success value retained")) } else { Err(Box::new(Failures(failures))) }
}

/// External harness supplies two fresh genuine public reviewer fixtures. Control
/// admission must pass first. Attack must refuse after a coherent DB-only swap;
/// before-fix admission intentionally fails this final security assertion AFTER
/// real wait/restoration/Root closure and persisted original observations.
#[test]
#[ignore = "owned public fixtures and explicit input required; no normal CLI coverage"]
fn reviewer_root_identity_database_only_requires_original_instance() -> Result<()> {
    let path = std::env::var_os("SWARM_REVIEW_ROOT_IDENTITY_INPUT")
        .ok_or_else(|| io::Error::other("explicit owned root identity input required"))?;
    let input: Input = serde_json::from_slice(&fs::read(path)?)?;
    require(input.schema_version == 1 && input.output.is_absolute()
        && input.control.root != input.attack.root, "distinct owned roots/schema required")?;
    fs::create_dir(&input.output)?;
    let control = run_case(&input.control, &input.output.join("control"), false)?;
    require(control, "valid original admission control failed; do not run fault")?;
    let attack = run_case(&input.attack, &input.output.join("database-only"), true)?;
    fs::write(input.output.join("result.json"), serde_json::to_vec(&serde_json::json!({
        "status":if attack {"FAIL_SECURITY_GATE_ADMITTED_COPY"} else {"PASS_EXPECTED_REFUSAL"},
        "control_admitted":control,"copy_admitted":attack,"profile":"libtest",
        "normal_cli":false,"provider_executed":false,"task_accepted":false
    }))?)?;
    require(!attack, "security regression: real reviewer gate admitted substituted DB")
}
