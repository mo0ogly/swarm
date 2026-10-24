//! Isolated fixture CLI; excluded from builds without test-fixtures.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{self, BufRead},
    path::Path,
    sync::Arc,
};
use swarm_enrollment_config::{
    self as cfg,
    store::{open_private_directory, Authorizer, Context, Coordinator, Store},
};
use swarm_identity_contract::Principal;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
#[derive(Deserialize)]
struct Input {
    op: String,
    #[serde(default)]
    coordinated: bool,
    #[serde(default)]
    witness_dir: String,
    #[serde(default)]
    dir: String,
    #[serde(default)]
    handle: String,
    #[serde(default)]
    authority: String,
    #[serde(default)]
    owner_delta: u32,
    #[serde(default)]
    different_pins: bool,
    change: Option<cfg::Change>,
    head: Option<cfg::Head>,
    at: Option<String>,
    #[serde(default)]
    year: i32,
    #[serde(default)]
    canceled: bool,
    #[serde(default)]
    deadline: bool,
    raw: Option<String>,
    umask: Option<u32>,
}
fn pins() -> cfg::Pins {
    cfg::Pins {
        installation_digest: "a".repeat(64),
        public_key: URL_SAFE_NO_PAD.encode([7; 32]),
        principal: Principal {
            id: "operator".into(),
            kind: "human".into(),
            issuer: "https://fixture.invalid".into(),
            subject: "subject-exact".into(),
        },
    }
}
fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(raw: &str) -> Vec<u8> {
    (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect()
}
enum Handle {
    Plain(Store),
    Coordinated(Coordinator),
}
impl Handle {
    fn close(&self) -> Result<(), cfg::Error> {
        match self {
            Self::Plain(s) => s.close(),
            Self::Coordinated(s) => s.close(),
        }
    }
    fn read(&self, ctx: &Context, head: Option<&cfg::Head>) -> Result<cfg::View, cfg::Error> {
        match self {
            Self::Plain(s) => s.read(ctx, head.unwrap()),
            Self::Coordinated(s) => s.read(ctx),
        }
    }
    fn preview(
        &self,
        ctx: &Context,
        c: &cfg::Change,
        t: OffsetDateTime,
    ) -> Result<cfg::Projection, cfg::Error> {
        match self {
            Self::Plain(s) => s.preview(ctx, c, t),
            Self::Coordinated(s) => s.preview(ctx, c, t),
        }
    }
    fn create(
        &self,
        ctx: &Context,
        c: &cfg::Change,
        t: OffsetDateTime,
    ) -> Result<cfg::View, cfg::Error> {
        match self {
            Self::Plain(s) => s.create(ctx, c, t),
            Self::Coordinated(s) => s.create(ctx, c, t),
        }
    }
    fn apply(
        &self,
        ctx: &Context,
        c: &cfg::Change,
        t: OffsetDateTime,
    ) -> Result<cfg::View, cfg::Error> {
        match self {
            Self::Plain(s) => s.apply(ctx, c, t),
            Self::Coordinated(s) => s.apply(ctx, c, t),
        }
    }
}
fn evaluate(root: &Path, stores: &mut HashMap<String, Handle>, v: Input) -> Value {
    let mut out = json!({"code":"ok","zero":true,"view":null,"projection":null,"raw":""});
    let ctx = Context::new();
    if v.deadline {
        ctx.cancel_for_deadline();
    }
    if v.canceled {
        ctx.cancel()
    }
    let old_mask = v
        .umask
        .map(|m| rustix::process::umask(rustix::fs::Mode::from_raw_mode(m)));
    let result = (|| -> Result<(), cfg::Error> {
        match v.op.as_str() {
            "mkdir" => {
                assert!(swarm_identity_contract::name(v.dir.as_bytes()));
                let path = root.join(&v.dir);
                std::fs::DirBuilder::new().mode(0o700).create(path).unwrap();
            }
            "decode" => {
                out["raw"] = json!(hex(&cfg::encode(&cfg::decode(&unhex(
                    v.raw.as_ref().unwrap()
                ))?)?));
            }
            "open" => {
                assert!(
                    swarm_identity_contract::name(v.dir.as_bytes())
                        && swarm_identity_contract::name(v.handle.as_bytes())
                );
                assert!(!stores.contains_key(&v.handle));
                let mut p = pins();
                if v.different_pins {
                    p.installation_digest = "b".repeat(64)
                }
                let actor = p.principal.clone();
                let no_authority = v.authority == "none";
                let mode = v.authority;
                let auth: Authorizer = Arc::new(move |ctx, mut current, mut c| {
                    match mode.as_str() {
                        "deny" => return Err(cfg::Error::Format),
                        "wrong" => {
                            let mut actor = actor.clone();
                            actor.subject = "different".into();
                            return Ok(actor);
                        }
                        "cancel" => ctx.cancel(),
                        "owned-copy" => {
                            if let Some(s) = current.as_mut() {
                                s.history[0].change.reason = "own copy".into()
                            }
                            c.reason = "own copy".into()
                        }
                        _ => (),
                    }
                    Ok(actor.clone())
                });
                let auth = if no_authority { None } else { Some(auth) };
                let path = root.join(&v.dir);
                let owner = rustix::process::geteuid().as_raw() + v.owner_delta;
                let s = if v.coordinated {
                    assert!(swarm_identity_contract::name(v.witness_dir.as_bytes()));
                    Handle::Coordinated(Coordinator::open(
                        &path,
                        &root.join(&v.witness_dir),
                        owner,
                        p,
                        auth,
                    )?)
                } else {
                    Handle::Plain(Store::open(&path, owner, p, auth)?)
                };
                stores.insert(v.handle, s);
            }
            "close" => stores.get(&v.handle).unwrap().close()?,
            "read" => {
                out["view"] = serde_json::to_value(
                    stores.get(&v.handle).unwrap().read(&ctx, v.head.as_ref())?,
                )
                .unwrap();
            }
            "recover" => {
                let Handle::Coordinated(c) = stores.get(&v.handle).unwrap() else {
                    panic!("coordinator required")
                };
                out["view"] = serde_json::to_value(c.recover(&ctx)?).unwrap();
            }
            "interrupt-intent" => {
                let Handle::Coordinated(c) = stores.get(&v.handle).unwrap() else {
                    panic!("coordinator required")
                };
                c.fixture_hook(Some(Arc::new(|phase, _, _| {
                    if phase == cfg::store::FixturePhase::Prepared {
                        std::process::exit(0);
                    }
                })));
            }
            "preview" | "create" | "apply" => {
                let at = if v.year != 0 {
                    time::Date::from_calendar_date(v.year, time::Month::January, 1)
                        .unwrap()
                        .midnight()
                        .assume_utc()
                } else {
                    OffsetDateTime::parse(v.at.as_ref().unwrap(), &Rfc3339).unwrap()
                };
                let s = stores.get(&v.handle).unwrap();
                let c = v.change.as_ref().unwrap();
                if v.op == "preview" {
                    out["projection"] = serde_json::to_value(s.preview(&ctx, c, at)?).unwrap()
                } else {
                    out["view"] = serde_json::to_value(if v.op == "create" {
                        s.create(&ctx, c, at)?
                    } else {
                        s.apply(&ctx, c, at)?
                    })
                    .unwrap()
                }
            }
            _ => panic!("unknown fixture operation"),
        }
        Ok(())
    })();
    if let Some(old) = old_mask {
        rustix::process::umask(old);
    }
    if let Err(e) = result {
        out["code"] = json!(e.code())
    }
    out
}
use std::os::unix::fs::DirBuilderExt;
fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("explicit isolated fixture root required");
    let root = Path::new(&root);
    nix::unistd::close(open_private_directory(root, rustix::process::geteuid().as_raw()).unwrap())
        .unwrap();
    let mut stores = HashMap::new();
    for line in io::stdin().lock().lines() {
        let v: Input = serde_json::from_str(&line.unwrap()).unwrap();
        println!("{}", evaluate(root, &mut stores, v));
    }
    for (_, s) in stores {
        s.close().unwrap();
    }
}
