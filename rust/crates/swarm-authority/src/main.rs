//! `swarm-authority` — cockpit authority daemon (phase B).
//!
//! Every command first loads the protected installation declaration
//! (`--declaration PATH --pin SHA256`), which refuses unless the process runs
//! as the declared `service_uid`. Commands:
//!
//! - `prepare --dir DIR --controller-uid UID --admin SUBJECT --json` (no
//!   declaration yet; run as the service account): create `DIR/{bootstrap,
//!   configuration,witness}` (0700), the private signing seed and the
//!   installation declaration pinning a random installation digest, the
//!   daemon public key and the first admin subject; print the pin. Never
//!   replaces an existing seed or declaration.
//! - `check --json`: readiness only.
//! - `init --admin SUBJECT --display NAME --json`: initialize the authority:
//!   create the private signing seed (`authority-signing-seed`, 0600, never
//!   replaced) and the **first user** (roles `admin`, integrity 3) whose
//!   password is asked twice without echo on a terminal, or read from the first
//!   stdin line otherwise. SUBJECT must be the admin pinned by `prepare`
//!   (`authority-admin-not-pinned`). Refuses once any account exists
//!   (`authority-already-initialized`); rerunning after an interruption
//!   between seed and account completes it. Prints the public key to save in
//!   the engine setting `web_authority_public_key`.
//! - `add-account SUBJECT --roles R[,R] --integrity N --display NAME --json`:
//!   create a further account; password read like `init` (never argv).
//!   Meant to run as `sudo -u swarm-authority …`, outside any agent channel.
//! - `serve --listen HOST:PORT --engine HOST:PORT`: run the HTTP front.
//!
//! Output: JSON on stdout (exit 0) or `{"schema_version":1,"failure":{"code":…}}`
//! on stderr (exit 2); usage errors exit 64.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::{collections::BTreeMap, io::Write, path::Path, process::ExitCode, sync::Arc};

fn usage() -> ExitCode {
    eprintln!("usage: swarm-authority prepare --dir DIR --controller-uid UID --admin SUBJECT --json | check|init|add-account SUBJECT|serve --declaration PATH --pin SHA256 [options] [--json]");
    ExitCode::from(64)
}

fn fail(code: &str) -> ExitCode {
    eprintln!("{}", serde_json::json!({"schema_version": 1, "failure": {"code": code}}));
    ExitCode::from(2)
}

fn print(v: serde_json::Value) -> ExitCode {
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{v}").is_err() {
        return ExitCode::from(74);
    }
    ExitCode::SUCCESS
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    use swarm_authority::{accounts::{Accounts, Profile}, config::AuthorityConfig, server, sessions::Sessions};
    use swarm_enrollment_state::Context;

    pub const SEED: &str = "authority-signing-seed";

    /// Read the 32-byte private seed: regular file, owned by euid, mode 0600.
    pub fn read_seed(dir: &Path) -> Result<ed25519_dalek::SigningKey, &'static str> {
        let path = dir.join(SEED);
        let meta = std::fs::symlink_metadata(&path).map_err(|_| "authority-seed-missing")?;
        if !meta.is_file() || meta.uid() != rustix::process::geteuid().as_raw() || meta.permissions().mode() & 0o077 != 0 {
            return Err("authority-seed-unsafe");
        }
        let raw: [u8; 32] = std::fs::read(&path).map_err(|_| "authority-seed-io")?.try_into().map_err(|_| "authority-seed-invalid")?;
        Ok(ed25519_dalek::SigningKey::from_bytes(&raw))
    }

    /// Create the seed with O_EXCL (never replaces an existing key).
    pub fn create_seed(dir: &Path) -> Result<ed25519_dalek::SigningKey, &'static str> {
        let mut raw = [0u8; 32];
        getrandom::fill(&mut raw).map_err(|_| "authority-entropy")?;
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(dir.join(SEED)).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists { "authority-seed-exists" } else { "authority-seed-io" }
        })?;
        f.write_all(&raw).and_then(|()| f.sync_all()).map_err(|_| "authority-seed-io")?;
        Ok(ed25519_dalek::SigningKey::from_bytes(&raw))
    }

    /// Read a password: on a terminal, prompt twice on stderr with echo off
    /// and require both entries to match; otherwise the first stdin line.
    /// Empty passwords refuse. The password never reaches argv or logs.
    pub fn read_password() -> Result<String, &'static str> {
        use rustix::termios::{tcgetattr, tcsetattr, LocalModes, OptionalActions};
        let stdin = std::io::stdin();
        let line = |prompt: &str| -> Result<String, &'static str> {
            if !prompt.is_empty() {
                eprint!("{prompt}");
            }
            let mut s = String::new();
            stdin.read_line(&mut s).map_err(|_| "authority-password-input")?;
            Ok(s.trim_end_matches(['\n', '\r']).to_owned())
        };
        let password = match tcgetattr(&stdin) {
            Ok(saved) => {
                let mut quiet = saved.clone();
                quiet.local_modes.remove(LocalModes::ECHO);
                tcsetattr(&stdin, OptionalActions::Now, &quiet).map_err(|_| "authority-password-input")?;
                let first = line("Mot de passe / Password: ");
                eprintln!();
                let second = line("Confirmation: ");
                eprintln!();
                let _ = tcsetattr(&stdin, OptionalActions::Now, &saved);
                let (first, second) = (first?, second?);
                if first != second {
                    return Err("authority-password-mismatch");
                }
                first
            }
            Err(_) => line("")?,
        };
        if password.is_empty() {
            return Err("authority-password-empty");
        }
        Ok(password)
    }

    pub struct Loaded {
        pub dir: std::path::PathBuf,
        pub installation: String,
        pub project: [u8; 32],
        /// Daemon public key pinned in the declaration (base64url).
        pub pinned_key: String,
        /// First admin subject pinned in the declaration.
        pub pinned_admin: String,
    }

    /// Read the seed and require it to match the pinned public key, so a
    /// replaced seed file can never sign for this installation.
    pub fn pinned_seed(l: &Loaded) -> Result<ed25519_dalek::SigningKey, &'static str> {
        let key = read_seed(&l.dir)?;
        if URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes()) != l.pinned_key {
            return Err("authority-key-not-pinned");
        }
        Ok(key)
    }

    /// `prepare`: see module docs. Runs before any declaration exists, as the
    /// service account owning `dir` (0700). Prints `{pin, declaration,
    /// public_key}`; the pin must be stored root-owned (systemd unit).
    pub fn prepare(opt: &BTreeMap<String, String>) -> ExitCode {
        let (Some(dir), Some(controller), Some(admin)) = (
            opt.get("--dir").map(std::path::PathBuf::from),
            opt.get("--controller-uid").and_then(|u| u.parse::<u32>().ok()),
            opt.get("--admin"),
        ) else { return usage() };
        if !swarm_identity_contract::name(admin.as_bytes()) {
            return fail("authority-admin-invalid");
        }
        let euid = rustix::process::geteuid().as_raw();
        match std::fs::metadata(&dir) {
            Ok(m) if m.is_dir() && m.uid() == euid && m.permissions().mode() & 0o077 == 0 => {}
            Ok(_) => return fail("authority-dir-unsafe"),
            Err(_) => return fail("authority-dir-missing"),
        }
        for sub in ["bootstrap", "configuration", "witness"] {
            let p = dir.join(sub);
            if let Err(e) = std::fs::create_dir(&p) {
                if e.kind() != std::io::ErrorKind::AlreadyExists { return fail("authority-dir-io"); }
            }
            if std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).is_err() { return fail("authority-dir-io"); }
        }
        let declaration = dir.join("bootstrap/declaration.json");
        if declaration.exists() {
            return fail("authority-declaration-exists");
        }
        let key = match create_seed(&dir.join("configuration")) { Ok(k) => k, Err(c) => return fail(c) };
        let mut digest = [0u8; 32];
        if getrandom::fill(&mut digest).is_err() { return fail("authority-entropy"); }
        let public_key = URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes());
        let doc = swarm_enrollment_bootstrap::Document {
            schema_version: 1,
            pins: swarm_enrollment_config::Pins {
                installation_digest: digest.iter().map(|b| format!("{b:02x}")).collect(),
                public_key: public_key.clone(),
                principal: swarm_identity_contract::Principal {
                    id: admin.clone(),
                    kind: "human".into(),
                    issuer: swarm_authority::accounts::ISSUER.into(),
                    subject: admin.clone(),
                },
            },
            service_uid: euid,
            controller_uid: controller,
            configuration_directory: dir.join("configuration").to_string_lossy().into_owned(),
            witness_directory: dir.join("witness").to_string_lossy().into_owned(),
        };
        let raw = match swarm_enrollment_bootstrap::encode(&doc) { Ok(r) => r, Err(e) => return fail(e.code()) };
        let written = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&declaration)
            .and_then(|mut f| f.write_all(&raw).and_then(|()| f.sync_all()));
        if written.is_err() { return fail("authority-declaration-io"); }
        use sha2::Digest as _;
        let pin = format!("{:x}", sha2::Sha256::digest(&raw));
        print(serde_json::json!({"schema_version": 1, "declaration": declaration, "pin": pin, "public_key": public_key, "admin": admin}))
    }

    pub fn load(declaration: &str, pin: &str) -> Result<Loaded, &'static str> {
        let d = swarm_enrollment_bootstrap::load(declaration, pin).map_err(|e| e.code())?;
        let installation = d.pins.installation_digest.clone();
        let (pinned_key, pinned_admin) = (d.pins.public_key.clone(), d.pins.principal.subject.clone());
        let mut project = [0u8; 32];
        for (i, b) in project.iter_mut().enumerate() {
            *b = u8::from_str_radix(&installation[2 * i..2 * i + 2], 16).map_err(|_| "authority-installation-invalid")?;
        }
        Ok(Loaded { dir: d.configuration_directory.into(), installation, project, pinned_key, pinned_admin })
    }

    pub fn accounts(l: &Loaded, key: ed25519_dalek::SigningKey) -> Result<Accounts, &'static str> {
        Accounts::open(&Context::new(), &l.dir.join("accounts.sqlite"), key, l.project, &l.installation).map_err(|e| e.code())
    }

    pub fn run(cmd: &str, positional: &[String], opt: &BTreeMap<String, String>) -> ExitCode {
        if cmd == "prepare" {
            return prepare(opt);
        }
        let (Some(declaration), Some(pin)) = (opt.get("--declaration"), opt.get("--pin")) else { return usage() };
        let loaded = match load(declaration, pin) { Ok(l) => l, Err(c) => return fail(c) };
        match cmd {
            "check" => match swarm_authority::check(declaration, pin) {
                Ok(r) => print(serde_json::to_value(r).unwrap_or_default()),
                Err(e) => fail(e.code()),
            },
            "init" => {
                let (Some(subject), Some(display)) = (opt.get("--admin"), opt.get("--display")) else { return usage() };
                if *subject != loaded.pinned_admin {
                    return fail("authority-admin-not-pinned");
                }
                let config = match AuthorityConfig::load(&loaded.dir) { Ok(c) => c, Err(c) => return fail(c) };
                // The seed was created by `prepare` and is pinned in the declaration.
                let key = match pinned_seed(&loaded) { Ok(k) => k, Err(c) => return fail(c) };
                let a = match accounts(&loaded, key.clone()) { Ok(a) => a, Err(c) => return fail(c) };
                match a.list(&Context::new()) {
                    Ok(list) if !list.is_empty() => return fail("authority-already-initialized"),
                    Ok(_) => {}
                    Err(e) => return fail(e.code()),
                }
                let password = match read_password() { Ok(p) => p, Err(c) => return fail(c) };
                let profile = Profile { display_name: display.clone(), roles: vec!["admin".into()], integrity: 3, attributes: BTreeMap::new() };
                match a.create(&Context::new(), subject, &profile, password.as_bytes(), config.argon2) {
                    Ok(_) => print(serde_json::json!({"schema_version": 1, "admin": subject, "public_key": URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes())})),
                    Err(e) => fail(e.code()),
                }
            }
            "add-account" => {
                let (Some(subject), Some(roles), Some(integrity), Some(display)) =
                    (positional.first(), opt.get("--roles"), opt.get("--integrity").and_then(|i| i.parse().ok()), opt.get("--display"))
                else { return usage() };
                let password = match read_password() { Ok(p) => p, Err(c) => return fail(c) };
                let config = match AuthorityConfig::load(&loaded.dir) { Ok(c) => c, Err(c) => return fail(c) };
                let key = match pinned_seed(&loaded) { Ok(k) => k, Err(c) => return fail(c) };
                let a = match accounts(&loaded, key) { Ok(a) => a, Err(c) => return fail(c) };
                let profile = Profile { display_name: display.clone(), roles: roles.split(',').map(str::to_owned).collect(), integrity, attributes: BTreeMap::new() };
                match a.create(&Context::new(), subject, &profile, password.as_bytes(), config.argon2) {
                    Ok(_) => print(serde_json::json!({"schema_version": 1, "subject": subject, "created": true})),
                    Err(e) => fail(e.code()),
                }
            }
            "serve" => {
                let (Some(listen), Some(engine)) = (opt.get("--listen"), opt.get("--engine").and_then(|e| e.parse().ok())) else { return usage() };
                let config = match AuthorityConfig::load(&loaded.dir) { Ok(c) => c, Err(c) => return fail(c) };
                let key = match pinned_seed(&loaded) { Ok(k) => k, Err(c) => return fail(c) };
                let a = match accounts(&loaded, key.clone()) { Ok(a) => a, Err(c) => return fail(c) };
                let rt = match tokio::runtime::Runtime::new() { Ok(r) => r, Err(_) => return fail("authority-runtime") };
                rt.block_on(async move {
                    let listener = match tokio::net::TcpListener::bind(listen).await { Ok(l) => l, Err(_) => return fail("authority-listen") };
                    let own_host = match listener.local_addr() { Ok(a) => a.to_string(), Err(_) => return fail("authority-listen") };
                    eprintln!("{}", serde_json::json!({"schema_version": 1, "listening": own_host, "engine": engine}));
                    let front = Arc::new(server::Front { accounts: a, sessions: Sessions::new(config), signer: key, config, own_host, engine });
                    tokio::select! {
                        r = server::serve(listener, front) => if r.is_err() { return fail("authority-serve") },
                        _ = tokio::signal::ctrl_c() => {}
                    }
                    ExitCode::SUCCESS
                })
            }
            _ => usage(),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else { return usage() };
    let mut opt = BTreeMap::new();
    let mut positional = Vec::new();
    let mut iter = args.into_iter().skip(1);
    while let Some(a) = iter.next() {
        if a == "--json" {
            opt.insert(a, String::new());
        } else if a.starts_with("--") {
            let Some(v) = iter.next() else { return usage() };
            if opt.insert(a, v).is_some() { return usage(); }
        } else {
            positional.push(a);
        }
    }
    #[cfg(target_os = "linux")]
    {
        linux::run(&cmd, &positional, &opt)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (cmd, positional, opt);
        fail("authority-linux-required")
    }
}
