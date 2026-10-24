//! Pure fixture SQL oracle. Neither main engine storage nor providers are opened.
use rusqlite::{types::Value, Connection};
use serde::Deserialize;
use std::{
    error::Error,
    fmt,
    io::{self, BufRead, Write},
};
type Cause = Box<dyn Error>;
#[derive(Deserialize)]
struct Input {
    name: String,
    raw: String,
    #[serde(default)]
    kind: String,
    #[serde(default = "legacy_projection")]
    projection: i64,
}
fn legacy_projection() -> i64 {
    1
}
struct Causes(Vec<Cause>);
impl fmt::Display for Causes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "identity plan fixture failed ({} causes)", self.0.len())
    }
}
impl fmt::Debug for Causes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for Causes {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.first().map(|v| v.as_ref())
    }
}
fn run(connection: &Connection) -> Result<(), Cause> {
    connection.busy_timeout(std::time::Duration::ZERO)?;
    swarm_engine::register_identity_plan_projection(connection)?;
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let v: Input = serde_json::from_str(&line?)?;
        let projection = match v.projection {
            1 => swarm_engine::IdentityPlanProjection::V1,
            2 => swarm_engine::IdentityPlanProjection::V2,
            _ => return Err("unknown fixture projection".into()),
        };
        if !v.raw.len().is_multiple_of(2) || !v.raw.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid fixture hex".into());
        }
        let raw = (0..v.raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&v.raw[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()?;
        let parameter = match v.kind.as_str() {
            "" | "blob" => Value::Blob(raw.clone()),
            "text" => {
                // SQLite text may contain invalid UTF-8 in Go. Bind those test bytes
                // as BLOB then CAST through SQLite, rather than silently repairing.
                Value::Blob(raw.clone())
            }
            "null" => Value::Null,
            "integer" => Value::Integer(1),
            "real" => Value::Real(1.0),
            _ => return Err("unknown fixture parameter kind".into()),
        };
        let query = match (projection, v.kind == "text") {
            (swarm_engine::IdentityPlanProjection::V1, true) => {
                "SELECT swarm_identity_plan_digest_v1(CAST(? AS TEXT))"
            }
            (swarm_engine::IdentityPlanProjection::V1, false) => {
                "SELECT swarm_identity_plan_digest_v1(?)"
            }
            (swarm_engine::IdentityPlanProjection::V2, true) => {
                "SELECT swarm_identity_plan_digest_v2(CAST(? AS TEXT))"
            }
            (swarm_engine::IdentityPlanProjection::V2, false) => {
                "SELECT swarm_identity_plan_digest_v2(?)"
            }
        };
        let result = connection.query_row(query, [parameter], |r| r.get::<_, String>(0));
        if matches!(v.kind.as_str(), "" | "blob" | "text") {
            let direct = swarm_engine::identity_plan_digest_for(&raw, projection);
            match (&direct, &result) {
                (Ok(a), Ok(b)) if a == b => {}
                (Err(_), Err(_)) => {}
                _ => return Err("native digest and SQL function disagree".into()),
            }
        }
        let (code, digest) = match result {
            Ok(d) => ("ok", d),
            Err(_) => ("format", String::new()),
        };
        writeln!(
            out,
            "{}",
            serde_json::json!({"name":v.name,"code":code,"digest":digest})
        )?;
    }
    Ok(())
}
fn main() -> Result<(), Cause> {
    let connection = Connection::open_in_memory()?;
    let mut causes = Vec::<Cause>::new();
    if let Err(e) = run(&connection) {
        causes.push(e);
    }
    if let Err((_, e)) = connection.close() {
        causes.push(Box::new(e));
    }
    if causes.is_empty() {
        Ok(())
    } else {
        Err(Box::new(Causes(causes)))
    }
}
