//! Two-directory intent protocol. Independence is logical: this does not defend
//! against restoring both stores, root, or agents sharing the service UID.
use super::*;
use crate::{entry_head, wire_input, Operation};
use serde::{Deserialize, Serialize};
use swarm_wire_json as wire;

#[derive(Clone, Serialize, Deserialize)]
struct Intent {
    entry: Entry,
    target: Head,
}
#[derive(Clone, Serialize, Deserialize)]
struct Witness {
    schema_version: i64,
    pins: Pins,
    committed: Head,
    pending: Option<Intent>,
}
fn encode_witness(w: &Witness) -> Result<Vec<u8>, Error> {
    if w.schema_version != 1 {
        return Err(Error::Version);
    }
    if w.pins.validate().is_err() || !w.committed.valid() {
        return Err(Error::Witness);
    }
    let mut w = w.clone();
    if let Some(intent) = w.pending.as_mut() {
        intent.entry.change.validate()?;
        if w.committed.revision == u64::MAX
            || intent.entry.change.expected_head != w.committed
            || intent.entry.actor != w.pins.principal
            || timestamp::zero(intent.entry.at)
        {
            return Err(Error::Witness);
        }
        intent.entry.at = intent
            .entry
            .at
            .checked_to_offset(UtcOffset::UTC)
            .ok_or(Error::Format)?;
        if entry_head(&w.pins, &intent.entry)? != intent.target {
            return Err(Error::Witness);
        }
    } else if w.committed.revision == 0 {
        return Err(Error::Witness);
    }
    wire::encode(&w).map_err(Error::Frame)
}
fn decode_witness(raw: &[u8]) -> Result<Witness, Error> {
    let w: Witness = wire_input::witness(raw)?;
    if encode_witness(&w)? != raw {
        return Err(Error::Witness);
    }
    Ok(w)
}
fn stage<T>(operation: Operation, result: Result<T, Error>) -> Result<T, Error> {
    result.map_err(|cause| Error::At {
        operation,
        cause: Box::new(cause),
    })
}
#[cfg(feature = "test-fixtures")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixturePhase {
    Prepared,
    Confirming,
}
/// Trusted fixture-only hook, absent from a default dependency. Runs while both
/// reservations are held: it must not reenter or close either store.
#[cfg(feature = "test-fixtures")]
pub type FixtureHook = Arc<dyn Fn(FixturePhase, &Store, &Store) + Send + Sync>;
#[derive(Clone)]
pub struct Coordinator {
    config: Store,
    witness: Store,
    #[cfg(feature = "test-fixtures")]
    hook: Arc<Mutex<Option<FixtureHook>>>,
}
impl fmt::Debug for Coordinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Coordinator").finish_non_exhaustive()
    }
}
impl Coordinator {
    pub fn open(
        config_path: &Path,
        witness_path: &Path,
        owner: u32,
        pins: Pins,
        authorize: Option<Authorizer>,
    ) -> Result<Self, Error> {
        // Component containment, not string prefixes ("/a" and "/ab" are separate).
        fn clean(path: &Path) -> bool {
            let raw = path.as_os_str().as_bytes();
            raw.starts_with(b"/")
                && (raw.len() == 1
                    || raw[1..]
                        .split(|b| *b == b'/')
                        .all(|p| !p.is_empty() && p != b"." && p != b".."))
        }
        if !clean(config_path)
            || !clean(witness_path)
            || !config_path.is_absolute()
            || !witness_path.is_absolute()
            || config_path.starts_with(witness_path)
            || witness_path.starts_with(config_path)
        {
            return Err(Error::Path);
        }
        let config = Store::open(config_path, owner, pins.clone(), authorize.clone())?;
        let witness = match Store::open(witness_path, owner, pins, authorize) {
            Ok(s) => s,
            Err(e) => return combined(Err(e), config.close()),
        };
        let distinct = (|| {
            let a = config.inner.handles.lock().map_err(|_| Error::State)?;
            let b = witness.inner.handles.lock().map_err(|_| Error::State)?;
            let a = unix::fstat(&a.as_ref().ok_or(Error::Closed)?.dir)
                .map_err(|e| Error::io(IoClass::Path, e))?;
            let b = unix::fstat(&b.as_ref().ok_or(Error::Closed)?.dir)
                .map_err(|e| Error::io(IoClass::Path, e))?;
            if a.st_dev == b.st_dev && a.st_ino == b.st_ino {
                return Err(Error::Path);
            }
            Ok(())
        })();
        if let Err(e) = distinct {
            return combined(combined(Err(e), config.close()), witness.close());
        }
        Ok(Self {
            config,
            witness,
            #[cfg(feature = "test-fixtures")]
            hook: Arc::new(Mutex::new(None)),
        })
    }
    pub fn close(&self) -> Result<(), Error> {
        combined(self.config.close(), self.witness.close())
    }
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_hook(&self, hook: Option<FixtureHook>) {
        *self.hook.lock().unwrap() = hook;
    }
    #[cfg(feature = "test-fixtures")]
    pub fn fixture_store_faults(&self, config: &[FixtureFault], witness: &[FixtureFault]) {
        self.config.fixture_faults(config);
        self.witness.fixture_faults(witness);
    }
    fn reserve<'a>(&'a self, ctx: &Context) -> Result<Pair<'a>, Error> {
        let config = self.config.reserve(ctx)?;
        match self.witness.reserve(ctx) {
            Ok(witness) => Ok(Pair { config, witness }),
            Err(e) => config.finish(Err(e)),
        }
    }
    pub fn read(&self, ctx: &Context) -> Result<View, Error> {
        let pair = self.reserve(ctx)?;
        let result = (|| {
            let (current, w) = pair.load()?;
            let v = committed_view(current.as_ref(), w.as_ref())?;
            pair.barriers()?;
            ctx.check()?;
            Ok(v)
        })();
        pair.finish(result)
    }
    pub fn preview(
        &self,
        ctx: &Context,
        c: &Change,
        at: OffsetDateTime,
    ) -> Result<Projection, Error> {
        validate(c, at)?;
        let pair = self.reserve(ctx)?;
        let result = (|| {
            let (current, w) = pair.load()?;
            let base = if w.is_some() {
                committed_view(current.as_ref(), w.as_ref())?.head
            } else {
                Head::default()
            };
            let (proposed, replay) = self.config.prepare(ctx, current.as_ref(), c, at)?;
            Ok(Projection {
                base,
                proposed,
                current_event_replay: replay,
            })
        })();
        pair.finish(result)
    }
    pub fn create(&self, ctx: &Context, c: &Change, at: OffsetDateTime) -> Result<View, Error> {
        if c.expected_head != Head::default() {
            return Err(Error::Conflict);
        }
        self.change(ctx, c, at, true)
    }
    pub fn apply(&self, ctx: &Context, c: &Change, at: OffsetDateTime) -> Result<View, Error> {
        self.change(ctx, c, at, false)
    }
    fn change(
        &self,
        ctx: &Context,
        c: &Change,
        at: OffsetDateTime,
        create: bool,
    ) -> Result<View, Error> {
        validate(c, at)?;
        let pair = self.reserve(ctx)?;
        let result = (|| {
            let (current, w) = pair.load()?;
            if let Some(w) = w.as_ref().filter(|w| w.pending.is_some()) {
                if w.pending.as_ref().ok_or(Error::State)?.entry.change != *c {
                    return Err(Error::Pending);
                }
                return self.resume(ctx, &pair, current.as_ref(), w);
            }
            if w.is_none() {
                if !create {
                    return Err(Error::NotConfigured);
                }
            } else {
                committed_view(current.as_ref(), w.as_ref())?;
            }
            let (result, replay) = self.config.prepare(ctx, current.as_ref(), c, at)?;
            if replay {
                pair.barriers()?;
                return Ok(result);
            }
            let w = Witness {
                schema_version: 1,
                pins: self.config.inner.pins.clone(),
                committed: w
                    .as_ref()
                    .map_or_else(Head::default, |w| w.committed.clone()),
                pending: Some(Intent {
                    entry: result.snapshot.history.last().ok_or(Error::State)?.clone(),
                    target: result.head.clone(),
                }),
            };
            stage(
                Operation::PrepareWitness,
                pair.save(&w, w.committed.revision == 0),
            )?;
            #[cfg(feature = "test-fixtures")]
            self.call_hook(FixturePhase::Prepared);
            self.commit(ctx, &pair, current.as_ref(), result, w, false)
        })();
        pair.finish(result)
    }
    pub fn recover(&self, ctx: &Context) -> Result<View, Error> {
        let pair = self.reserve(ctx)?;
        let result = (|| {
            let (current, w) = pair.load()?;
            let w = w.as_ref().ok_or(Error::NotConfigured)?;
            if w.pending.is_none() {
                let result = committed_view(current.as_ref(), Some(w))?;
                pair.barriers()?;
                return Ok(result);
            }
            self.resume(ctx, &pair, current.as_ref(), w)
        })();
        pair.finish(result)
    }
    fn resume(
        &self,
        ctx: &Context,
        pair: &Pair<'_>,
        current: Option<&Snapshot>,
        w: &Witness,
    ) -> Result<View, Error> {
        let intent = w.pending.as_ref().ok_or(Error::State)?;
        if let Some(current) = current {
            let head = snapshot_view(current)?.head;
            if head != w.committed && head != intent.target {
                return Err(Error::Head);
            }
        } else if w.committed != Head::default() {
            return Err(Error::NotConfigured);
        }
        let (result, _) =
            self.config
                .prepare(ctx, current, &intent.entry.change, intent.entry.at)?;
        let last = result.snapshot.history.last().ok_or(Error::State)?;
        if result.head != intent.target || last != &intent.entry {
            return Err(Error::Head);
        }
        self.commit(ctx, pair, current, result, w.clone(), true)
    }
    fn commit(
        &self,
        ctx: &Context,
        pair: &Pair<'_>,
        current: Option<&Snapshot>,
        result: View,
        mut w: Witness,
        resuming: bool,
    ) -> Result<View, Error> {
        stage(Operation::CheckConfiguration, pair.config.check_directory())?;
        if resuming {
            stage(Operation::CheckWitness, pair.witness.check_directory())?;
            stage(
                Operation::SyncPreparedWitness,
                pair.witness.sync_directory(),
            )?;
        }
        if let Err(e) = ctx.check() {
            return Err(Error::PendingCause(Box::new(e)));
        }
        let target = &w.pending.as_ref().ok_or(Error::State)?.target;
        let already = current
            .map(snapshot_view)
            .transpose()?
            .is_some_and(|v| v.head == *target);
        if !already {
            stage(
                Operation::WriteConfiguration,
                pair.config.persist_document(
                    "config.json",
                    &encode(&result.snapshot)?,
                    current.is_none(),
                ),
            )?;
        } else {
            stage(
                Operation::SyncRecoveredConfiguration,
                pair.config.sync_directory(),
            )?;
        }
        w.committed = result.head.clone();
        w.pending = None;
        #[cfg(feature = "test-fixtures")]
        self.call_hook(FixturePhase::Confirming);
        stage(Operation::ConfirmWitness, pair.save(&w, false))?;
        Ok(result)
    }
    #[cfg(feature = "test-fixtures")]
    fn call_hook(&self, phase: FixturePhase) {
        // Clone before calling to allow fixture hook replacement without holding
        // its mutex; the callback still runs under both Store reservations.
        let hook = self.hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(phase, &self.config, &self.witness);
        }
    }
}
fn validate(c: &Change, at: OffsetDateTime) -> Result<(), Error> {
    c.validate()?;
    if timestamp::zero(at) {
        return Err(Error::Format);
    }
    Ok(())
}
fn committed_view(current: Option<&Snapshot>, w: Option<&Witness>) -> Result<View, Error> {
    let w = w.ok_or(Error::NotConfigured)?;
    if w.pending.is_some() {
        return Err(Error::Pending);
    }
    let v = snapshot_view(current.ok_or(Error::NotConfigured)?)?;
    if v.head != w.committed {
        return Err(Error::Head);
    }
    Ok(v)
}
struct Pair<'a> {
    config: Reservation<'a>,
    witness: Reservation<'a>,
}
impl Pair<'_> {
    fn finish<T>(self, result: Result<T, Error>) -> Result<T, Error> {
        // Both releases execute even when the first fails. No usable value on
        // cleanup failure; all original causes remain in the error tree.
        let result = self.witness.finish(result);
        self.config.finish(result)
    }
    fn load(&self) -> Result<(Option<Snapshot>, Option<Witness>), Error> {
        let current = match self.config.load() {
            Ok(s) => Some(s),
            Err(Error::NotConfigured) => None,
            Err(e) => return Err(e),
        };
        let raw = match self.witness.read_document("witness.json") {
            Ok(raw) => raw,
            Err(Error::NotConfigured) if current.is_none() => return Ok((None, None)),
            Err(Error::NotConfigured) => return Err(Error::WitnessMissing),
            Err(e) => return Err(e),
        };
        let w = decode_witness(&raw)?;
        if w.pins != self.config.store.inner.pins {
            return Err(Error::Pins);
        }
        Ok((current, Some(w)))
    }
    fn save(&self, w: &Witness, create: bool) -> Result<(), Error> {
        self.witness
            .persist_document("witness.json", &encode_witness(w)?, create)
    }
    fn barriers(&self) -> Result<(), Error> {
        for (r, check, sync) in [
            (
                &self.config,
                Operation::CheckConfiguration,
                Operation::SyncConfiguration,
            ),
            (
                &self.witness,
                Operation::CheckWitness,
                Operation::SyncWitness,
            ),
        ] {
            stage(check, r.check_directory())?;
            stage(sync, r.sync_directory())?;
        }
        Ok(())
    }
}
impl Reservation<'_> {
    fn check_directory(&self) -> Result<(), Error> {
        safe_directory(&self.handles()?.dir, self.store.inner.owner)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn prepared() -> Witness {
        let pins = Pins {
            installation_digest: "a".repeat(64),
            public_key: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc".into(),
            principal: Principal {
                id: "operator".into(),
                kind: "human".into(),
                issuer: "https://fixture.invalid".into(),
                subject: "subject-exact".into(),
            },
        };
        let entry = Entry {
            change: Change {
                schema_version: 1,
                event_id: "initial".into(),
                expected_head: Head::default(),
                values: crate::Values {
                    mode: "local".into(),
                    socket_path: "/run/swarm-fixture/authority.sock".into(),
                },
                reason: "explicit fixture choice".into(),
            },
            actor: pins.principal.clone(),
            at: OffsetDateTime::from_unix_timestamp(1792238400).unwrap(),
        };
        let target = entry_head(&pins, &entry).unwrap();
        Witness {
            schema_version: 1,
            pins,
            committed: Head::default(),
            pending: Some(Intent { entry, target }),
        }
    }
    #[test]
    fn witness_codec_binds_intent_base_actor_target_and_original_bytes() {
        let w = prepared();
        let raw = encode_witness(&w).unwrap();
        let decoded = decode_witness(&raw).unwrap();
        assert_eq!(encode_witness(&decoded).unwrap(), raw);
        for variant in [
            "target",
            "base",
            "actor",
            "empty",
            "zero-time",
            "overflow",
            "pins",
            "mode",
        ] {
            let mut x = w.clone();
            match variant {
                "target" => x.pending.as_mut().unwrap().target.revision += 1,
                "base" => x.committed = x.pending.as_ref().unwrap().target.clone(),
                "actor" => x.pending.as_mut().unwrap().entry.actor.subject = "other".into(),
                "empty" => x.pending = None,
                "zero-time" => {
                    x.pending.as_mut().unwrap().entry.at =
                        crate::go_clock::parse("0001-01-01T00:00:00Z").unwrap()
                }
                "overflow" => {
                    x.committed.revision = u64::MAX;
                    x.committed.digest = "a".repeat(64);
                }
                "pins" => x.pins.public_key = "bad".into(),
                "mode" => x.pending.as_mut().unwrap().entry.change.values.mode = "remote".into(),
                _ => unreachable!(),
            }
            let code = encode_witness(&x).err().unwrap().code();
            assert_eq!(code, if variant == "mode" { "mode" } else { "witness" });
        }
        let mut committed = w.clone();
        committed.committed = w.pending.unwrap().target;
        committed.pending = None;
        assert!(decode_witness(&encode_witness(&committed).unwrap()).is_ok());
        let s = String::from_utf8(raw.clone()).unwrap();
        for input in [
            format!(" {s}"),
            format!("{s}{{}}"),
            s.replacen("schema_version", "Schema_Version", 1),
            s.replacen("\"revision\":\"0\"", "\"revision\":0", 1),
            s.replacen("{", "{\"unknown\":true,", 1),
            s.replacen("{", "{\"schema_version\":1,", 1),
        ] {
            assert!(decode_witness(input.as_bytes()).is_err());
        }
    }
    #[test]
    fn witness_projection_retains_go_error_order_pointer_null_and_depth() {
        let w = prepared();
        let raw = String::from_utf8(encode_witness(&w).unwrap()).unwrap();
        assert_eq!(decode_witness(b"{}").err().unwrap().code(), "version");
        assert_eq!(
            decode_witness(br#"{"schema_version":2,"pins":false}"#)
                .err()
                .unwrap()
                .code(),
            "witness"
        );
        assert_eq!(
            decode_witness(
                raw.replacen("schema_version\":1", "schema_version\":2", 1)
                    .as_bytes()
            )
            .err()
            .unwrap()
            .code(),
            "version"
        );
        let body = &raw[..raw.len() - 1];
        assert_eq!(
            decode_witness(format!("{body},\"pending\":null}}").as_bytes())
                .err()
                .unwrap()
                .code(),
            "witness"
        );
        // Sequential duplicate object merge must retain the original fields;
        // strict canonical comparison then rejects the duplicate representation.
        assert_eq!(
            decode_witness(format!("{body},\"pending\":{{}}}}").as_bytes())
                .err()
                .unwrap()
                .code(),
            "witness"
        );
        for depth in [9999, 10000] {
            let input = format!(
                "{{\"schema_version\":2,\"unknown\":{}0{}}}",
                "[".repeat(depth),
                "]".repeat(depth)
            );
            assert_eq!(
                decode_witness(input.as_bytes()).err().unwrap().code(),
                if depth == 9999 { "version" } else { "witness" }
            );
        }
    }
}
