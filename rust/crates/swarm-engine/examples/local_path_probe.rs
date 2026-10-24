//! Gated filesystem fixture; observes paths but grants no access authority.
#[cfg(unix)]
mod fixture {
    use serde::Deserialize;
    use std::{
        error::Error,
        io::{self, BufRead, Write},
        os::unix::ffi::{OsStrExt, OsStringExt},
        path::PathBuf,
    };
    use swarm_engine::{
        apply_checkpoint, canonical_owned_project_path, canonical_project_root, decode_request,
        decode_work, encode_model, local_file, GitState,
    };
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Frame {
        name: String,
        mode: String,
        root_hex: String,
        path_hex: String,
        file: String,
        body: String,
        request: String,
        git: FixtureGit,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureGit {
        head: String,
        branch: String,
        changes: String,
    }
    type Cause = Box<dyn Error>;
    fn unhex(s: &str) -> Result<Vec<u8>, Cause> {
        if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("invalid fixture hex".into());
        };
        Ok((0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<_, _>>()?)
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    fn one(f: Frame) -> Result<serde_json::Value, Cause> {
        let root = PathBuf::from(std::ffi::OsString::from_vec(unhex(&f.root_hex)?));
        let path = PathBuf::from(std::ffi::OsString::from_vec(unhex(&f.path_hex)?));
        let mut output = String::new();
        let mut body = String::new();
        let mut code = "ok";
        if f.mode == "checkpoint" {
            let mut w = decode_work(&unhex(&f.body)?)?;
            let r = decode_request(&unhex(&f.request)?)?;
            if let Err(e) = apply_checkpoint(
                &mut w,
                &r,
                &root,
                GitState {
                    head: f.git.head,
                    branch: f.git.branch,
                    changes: f.git.changes,
                },
            ) {
                code = e.code()
            };
            body = hex(&encode_model(&w)?);
        } else {
            let result = match f.mode.as_str() {
                "root" => canonical_project_root(&root),
                "owned" => canonical_owned_project_path(&root, &path),
                "local" => local_file(&root, &f.file),
                _ => return Err("unknown fixture mode".into()),
            };
            match result {
                Ok(p) => output = hex(p.as_os_str().as_bytes()),
                Err(_) => code = "format",
            }
        }
        Ok(serde_json::json!({"name":f.name,"code":code,"path":output,"body":body}))
    }
    fn run() -> Result<(), Cause> {
        let mut out = io::BufWriter::new(io::stdout().lock());
        for line in io::stdin().lock().lines() {
            serde_json::to_writer(&mut out, &one(serde_json::from_str(&line?)?)?)?;
            out.write_all(b"\n")?
        }
        out.flush()?;
        Ok(())
    }
    pub fn entry() {
        if let Err(e) = run() {
            eprintln!("{e}");
            std::process::exit(2)
        }
    }
}
#[cfg(unix)]
fn main() {
    fixture::entry()
}
#[cfg(not(unix))]
fn main() {
    eprintln!("Unix path fixtures required");
    std::process::exit(2)
}
