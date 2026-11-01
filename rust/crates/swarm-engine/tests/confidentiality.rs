//! Persistent BLP policy/public engine regressions on owned fixture roots.
//! No test invokes a provider or claims runtime information-flow enforcement.
#![cfg(target_os="linux")]
use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};

struct Fixture(std::path::PathBuf);
impl Fixture {
    /// Allocate below the configured test TMPDIR. The recipe supplies the project
    /// fixture area; no live database, service or operator credentials are used.
    fn new() -> Self {
        let area=std::env::var_os("TMPDIR").expect("project-local TMPDIR required");
        let area=std::path::PathBuf::from(area).canonicalize().unwrap();
        assert!(!area.starts_with("/tmp"));
        let path=area.join(swarm_engine::generate_identifier("blp-").unwrap());
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let store=RootStore::initialize(&path,&mut SystemEnvironment).unwrap(); store.close().unwrap();
        let fixture=Self(path);
        fixture.call("config-apply",config(0,"trust-init"),"").unwrap(); fixture
    }
    /// Exercise the public Root consumer and close it on both success/refusal.
    /// Refusals render the stable code plus every retained safe cause reason,
    /// since RootStoreError's own Display keeps only the top-level code.
    fn call(&self, op:&str, input:Value, token:&str)->Result<Value,String> {
        let store=RootStore::open(&self.0,true).unwrap();
        let result=store.trust_as_principal(op,&serde_json::to_vec(&input).unwrap(),token);
        store.close().unwrap();
        result.map(|r|serde_json::from_slice(&r.bytes).unwrap()).map_err(|e|{
            let mut text=e.to_string();
            let mut source=std::error::Error::source(&e);
            while let Some(cause)=source { text.push_str(": "); text.push_str(&cause.to_string()); source=cause.source(); }
            text
        })
    }
    fn state(&self)->Vec<u8>{std::fs::read(self.0.join(".swarm/trust-actions.json")).unwrap()}
    /// Publish new state bytes through the same trust-lock protocol the engine
    /// uses (exclusive reservation + atomic same-directory publish), only for
    /// a test-owned fixture root. This exercises the storage/reopen contract
    /// on histories no supported producer can produce; it is never used
    /// against a real project root and no evidence or other process is touched.
    /// The mapper receives the CURRENT bytes and returns the replacement bytes.
    fn rewrite_state(&self,map:impl FnOnce(Vec<u8>)->Vec<u8>){
        use std::os::unix::fs::OpenOptionsExt;
        let lock_path=self.0.join(".swarm/trust-actions.lock");
        let lock=std::fs::OpenOptions::new().create(true).write(true).mode(0o600)
            .open(&lock_path).unwrap();
        rustix::fs::flock(&lock,rustix::fs::FlockOperation::LockExclusive).unwrap();
        let next=map(self.state());
        crate_publish(&self.0,&next);
        rustix::fs::flock(&lock,rustix::fs::FlockOperation::Unlock).unwrap();
    }
    fn apply(&self)->Value{self.call("blp-policy-apply",change(0,"blp-init"),"fixture-admin-token").unwrap()}
}
impl Drop for Fixture {
    fn drop(&mut self){if !std::thread::panicking(){std::fs::remove_dir_all(&self.0).unwrap();}}
}
/// Atomic same-directory publish mirroring LocalDirectory::publish semantics
/// for the test-owned fixture: write a sibling temp file, fsync, rename over
/// the state file. A partial write never replaces the original.
fn crate_publish(root:&std::path::Path,bytes:&[u8]){
    use std::io::Write;
    let target=root.join(".swarm/trust-actions.json");
    let temp=root.join(".swarm/trust-actions.json.poison-tmp");
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file=std::fs::OpenOptions::new().create(true).write(true).truncate(true)
            .mode(0o600).open(&temp).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }
    std::fs::rename(&temp,&target).unwrap();
}
fn config(revision:u64,event:&str)->Value {
    json!({"schema_version":1,"event_id":event,"expected_revision":revision,"reason":"Owned isolated BLP fixture",
        "policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},
        "identities":[{"subject":"admin","integrity":3,"roles":["admin"],"token":"fixture-admin-token"},
        {"subject":"worker","integrity":1,"roles":["requester"],"token":"fixture-worker-token"},
        {"subject":"approver","integrity":2,"roles":["approver"],"token":"fixture-approver-token"}]})
}
fn change(revision:u64,event:&str)->Value {
    json!({"schema_version":1,"event_id":event,"expected_revision":revision,"expected_trust_revision":1,"reason":"Explicit confidentiality policy fixture",
        "policy":{"schema_version":1,"levels":[{"id":"public","name_fr":"Public","name_en":"Public"},{"id":"confidential","name_fr":"Confidentiel","name_en":"Confidential"}],
        "compartments":["client-a","client-b"],"clearances":{"worker":{"level":"confidential","compartments":["client-a"]}}}})
}
fn check(destination:Value)->Value {
    json!({"schema_version":1,"expected_revision":1,"sources":[{"level":"confidential","compartments":["client-a"]}],"destination":destination})
}

#[test]
fn policy_preview_apply_reopen_and_replay_preserve_history() {
    let f=Fixture::new(); let before=f.state();
    let preview=f.call("blp-policy-preview",change(0,"blp-init"),"fixture-admin-token").unwrap();
    assert_eq!(preview["applied"],false); assert_eq!(f.state(),before);
    let applied=f.apply(); assert_eq!(applied["proposed_revision"],"1"); assert_eq!(applied["runtime_enforced"],false);
    let after=f.state(); assert_eq!(f.apply(),applied); assert_eq!(f.state(),after);
    let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap();
    assert_eq!(view["record"]["history"].as_array().unwrap().len(),1);
    assert_eq!(view["record"]["history"][0]["actor"],"admin");
    assert_eq!(view["record"]["history"][0]["policy"],change(0,"blp-init")["policy"]);
    assert_eq!(view["capability"]["runtime_enforced"],false);
}

#[test]
fn session_local_operator_or_forged_level_cannot_configure_blp() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    for token in ["","fixture-worker-token","unknown-private-token"] {
        for op in ["blp-show","blp-policy-preview","blp-policy-apply"] {
            let input=if op=="blp-show"{json!({})}else{change(1,"unauthorized")};
            assert!(f.call(op,input,token).unwrap_err().contains("authorization_required"));
        }
    }
    let mut forged=change(1,"forged"); forged["actor"]="admin".into(); forged["integrity"]=3.into();
    assert!(f.call("blp-policy-apply",forged,"fixture-admin-token").is_err());
    assert_eq!(f.state(),before);
}

#[test]
fn supplied_label_diagnostics_are_readonly_and_never_runtime_authority() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    let allowed=f.call("blp-check",check(json!({"level":"confidential","compartments":["client-a"]})),"fixture-worker-token").unwrap();
    assert_eq!(allowed["allowed_by_supplied_labels"],true); assert_eq!(allowed["runtime_authorized"],false);
    for target in [json!({"level":"public","compartments":[]}),json!({"level":"confidential","compartments":["client-b"]}),Value::Null] {
        let denied=f.call("blp-check",check(target),"fixture-worker-token").unwrap();
        assert_eq!(denied["allowed_by_supplied_labels"],false); assert_eq!(denied["runtime_authorized"],false);
        assert!(denied.get("output_classification").is_none());
    }
    let mut unknown=check(json!({"level":"public","compartments":[]})); unknown["sources"]=json!([null]);
    assert_eq!(f.call("blp-check",unknown,"fixture-worker-token").unwrap()["reason"],"blp-classification-required");
    assert_eq!(f.state(),before);
}

#[test]
fn conflicts_catalog_changes_and_unrecognized_grants_do_not_mutate() {
    let f=Fixture::new(); f.apply(); let before=f.state();
    let mut cases=vec![change(0,"stale"),change(1,"blp-init")];
    let mut changed=change(1,"bad-order");changed["policy"]["levels"].as_array_mut().unwrap().reverse();cases.push(changed);
    changed=change(1,"unknown-grant");changed["policy"]["clearances"]["intruder"]=json!({"level":"public","compartments":[]});cases.push(changed);
    changed=change(1,"bad-schema");changed["policy"]["schema_version"]=2.into();cases.push(changed);
    for proposal in cases {assert!(f.call("blp-policy-apply",proposal,"fixture-admin-token").is_err());assert_eq!(f.state(),before);}
}

#[test]
fn trust_revision_policy_revocation_and_old_replays_are_not_reactivated() {
    let f=Fixture::new(); f.apply();
    let mut revoked=change(1,"revoke-clearance");revoked["policy"]["clearances"]=json!({});
    f.call("blp-policy-apply",revoked,"fixture-admin-token").unwrap();
    let before=f.state(); assert!(f.call("blp-policy-apply",change(0,"blp-init"),"fixture-admin-token").unwrap_err().contains("blp-replay-stale"));
    assert_eq!(f.state(),before);
    f.call("config-apply",config(1,"trust-change"),"fixture-admin-token").unwrap();
    let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap(); assert_eq!(view["capability"]["authority_current"],false);
    let before=f.state(); let mut input=check(json!({"level":"confidential","compartments":["client-a"]}));input["expected_revision"]=2.into();
    assert!(f.call("blp-check",input,"fixture-worker-token").unwrap_err().contains("blp-policy-stale")); assert_eq!(f.state(),before);
    let mut recovered=change(2,"explicit-renewal");recovered["expected_trust_revision"]=2.into();
    assert_eq!(f.call("blp-policy-apply",recovered,"fixture-admin-token").unwrap()["proposed_revision"],"3");
}

/// BLP-RET-05 control attribution: a produced attribution is bound to the
/// exact canonical digest, project path and current policy revision, is
/// replayed identically and is refused for a stale policy or bad digest
/// shape without any mutation. The attributing admin must itself hold a
/// clearance dominating the attributed label (classification authority is
/// subordinated to habilitation); errors surface with their exact code.
#[test]
fn object_attribute_produces_binds_and_replays_under_current_policy() {
    let f=Fixture::new(); f.apply();
    // The default fixture policy grants no admin clearance, so attribution of
    // a confidential label must refuse (no dominating clearance) and mutate
    // nothing; the renewed policy grants admin the confidential clearance.
    let digest="a".repeat(64);
    let base=json!({"schema_version":1,"event_id":"blp-attribute-init","expected_revision":1,
        "expected_trust_revision":1,"canonical_digest":digest,
        "project_path":f.0.to_str().unwrap(),
        "classification":{"level":"confidential","compartments":[]},
        "reason":"Control attribution for the exact prepared action fixture"});
    let before=f.state();
    let refusal=f.call("blp-object-attribute",base.clone(),"fixture-admin-token").unwrap_err();
    assert!(refusal.contains("blp-access-denied"),"actual refusal: {refusal}");
    assert_eq!(f.state(),before);
    let mut renewed=change(1,"blp-renewal-admin-clearance");renewed["expected_trust_revision"]=1.into();
    renewed["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":[]});
    f.call("blp-policy-apply",renewed,"fixture-admin-token").unwrap();
    let input=json!({"schema_version":1,"event_id":"blp-attribute-init","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest,
        "project_path":f.0.to_str().unwrap(),
        "project_path":f.0.to_str().unwrap(),
        "classification":{"level":"confidential","compartments":[]},
        "reason":"Control attribution for the exact prepared action fixture"});
    let reply=f.call("blp-object-attribute",input.clone(),"fixture-admin-token").unwrap_or_else(|e|panic!("attribute refused: {e}"));
    assert_eq!(reply["attributed"],true); assert_eq!(reply["runtime_enforced"],false);
    assert_eq!(reply["policy_revision"],"2");
    assert_eq!(f.call("blp-object-attribute",input,"fixture-admin-token").unwrap()["attributed"],true);
}

/// BLP-RET-05 attribution conflict: a second accepted classification of the
/// SAME canonical digest, project path and policy revision that contradicts
/// the recorded one must never silently ride beside it (the reader takes the
/// first) nor implicitly declassify it. The smallest public-path witness:
/// attribute the object confidential, then attribute the same object public
/// through the same supported producer. Either the producer refuses the
/// contradictory change without mutation, or the record refuses such a
/// history — the one outcome it must never have is a stored contradicting
/// pair where the first classification stays authoritative by accident.
#[test]
fn contradictory_re_attribution_of_same_object_is_refused_without_mutation() {
    let f=Fixture::new(); f.apply();
    let mut renewed=change(1,"blp-renewal-admin-clearance");renewed["expected_trust_revision"]=1.into();
    renewed["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":[]});
    f.call("blp-policy-apply",renewed,"fixture-admin-token").unwrap();
    let digest="b".repeat(64);
    let path=f.0.to_str().unwrap();
    let first=json!({"schema_version":1,"event_id":"blp-attribute-first","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest,"project_path":path,
        "classification":{"level":"confidential","compartments":[]},
        "reason":"Initial control attribution of the exact prepared action fixture"});
    let reply=f.call("blp-object-attribute",first,"fixture-admin-token").unwrap_or_else(|e|panic!("first attribute refused: {e}"));
    assert_eq!(reply["attributed"],true);
    let before=f.state();
    // Contradictory re-attribution of the exact same object under the same
    // current policy revision: a DIFFERENT event with a LOWER label.
    let second=json!({"schema_version":1,"event_id":"blp-attribute-second","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest,"project_path":path,
        "classification":{"level":"public","compartments":[]},
        "reason":"Contradictory lower re-attribution of the same object fixture"});
    let result=f.call("blp-object-attribute",second,"fixture-admin-token");
    // The record must not end with both classifications stored for one object.
    match result {
        Err(refusal)=>assert!(!refusal.contains("blp-replay-stale"),
            "identical-object re-attribution must refuse on the contradiction, not on stale replay: {refusal}"),
        Ok(reply)=>{
            let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap();
            let entries=view["record"]["history"].as_array().unwrap();
            let stored:Vec<&Value>=entries.iter()
                .flat_map(|e|e["attributions"].as_array())
                .flatten()
                .filter(|a|a["canonical_digest"]==digest.as_str()
                    &&a["project_path"].as_array().is_some_and(|b|
                        b.iter().map(|v|v.as_u64().unwrap_or(256) as u8).collect::<Vec<u8>>()
                            .as_slice()==path.as_bytes()))
                .collect();
            assert!(stored.len()==1,"at most one classification per object may persist; got {stored:?}");
            let _=reply;
        }
    }
    assert_eq!(f.state(),before,"a contradictory refusal must not mutate");
}

/// Admit a real fixture policy and a confidential object under revision one.
/// The admin can read both compartments; the approver has only public clearance.
/// No human identity, declassification grant or provider is involved.
fn classified_object_fixture() -> (Fixture, Value, Value) {
    let f=Fixture::new();
    let mut policy=change(0,"object-policy-one");
    policy["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":["client-a","client-b"]});
    policy["policy"]["clearances"]["approver"]=json!({"level":"public","compartments":[]});
    f.call("blp-policy-apply",policy.clone(),"fixture-admin-token").unwrap();
    let attribution=json!({"schema_version":1,"event_id":"object-first","expected_revision":1,
        "expected_trust_revision":1,"canonical_digest":"c".repeat(64),"project_path":f.0.to_str().unwrap(),
        "classification":{"level":"confidential","compartments":["client-a"]},"reason":"Exact confidential fixture object"});
    f.call("blp-object-attribute",attribution.clone(),"fixture-admin-token").unwrap();
    assert_eq!(object_reader(&f,"fixture-admin-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    assert_ne!(object_reader(&f,"fixture-approver-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    (f,policy,attribution)
}

/// Reopen the actual fixture Root and use the public reader guard on the exact
/// digest/path. Close on success/refusal, preserve safe nested cause diagnostics
/// like Fixture::call; no bytes are projected to a provider.
fn object_reader(f:&Fixture,token:&str)->Result<swarm_engine::ReaderVerdict,String> {
    let store=RootStore::open(&f.0,false).unwrap();
    let result=store.paea_reader_verdict(Some(token),&"c".repeat(64),&f.0);
    store.close().unwrap();result.map_err(|e|{
        let mut text=e.to_string();let mut source=std::error::Error::source(&e);
        while let Some(cause)=source {text.push_str(": ");text.push_str(&cause.to_string());source=cause.source();}
        text
    })
}

/// A new policy revision is not human declassification. Reproduce level loss,
/// compartment removal and incompatible compartments; preserve exact state on
/// refusal, then demonstrate stricter and equal renewal, replay and real reading.
#[test]
fn policy_revision_cannot_lower_or_remove_object_classification() {
    for denied in [json!({"level":"public","compartments":[]}),
        json!({"level":"confidential","compartments":[]}),
        json!({"level":"confidential","compartments":["client-b"]})] {
        let (f,mut policy,mut attribution)=classified_object_fixture();
        policy["expected_revision"]=1.into();policy["event_id"]="object-policy-two".into();
        f.call("blp-policy-apply",policy.clone(),"fixture-admin-token").unwrap();
        let before=f.state();
        attribution["expected_revision"]=2.into();attribution["event_id"]="object-lower".into();
        attribution["classification"]=denied;
        let result=f.call("blp-object-attribute",attribution.clone(),"fixture-admin-token");
        assert!(result.is_err(),"policy renewal admitted implicit declassification: {result:?}; low-reader={:?}",object_reader(&f,"fixture-approver-token"));
        assert!(result.unwrap_err().contains("blp-declassification-required"));
        assert_eq!(f.state(),before,"refused downgrade must preserve all state bytes");
        attribution["event_id"]="object-stricter".into();
        attribution["classification"]=json!({"level":"confidential","compartments":["client-a","client-b"]});
        let applied=f.call("blp-object-attribute",attribution.clone(),"fixture-admin-token").unwrap();
        let after=f.state();assert_eq!(f.call("blp-object-attribute",attribution.clone(),"fixture-admin-token").unwrap(),applied);
        assert_eq!(f.state(),after);
        assert_eq!(object_reader(&f,"fixture-admin-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
        assert_ne!(object_reader(&f,"fixture-approver-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
        policy["expected_revision"]=2.into();policy["event_id"]="object-policy-three".into();
        f.call("blp-policy-apply",policy,"fixture-admin-token").unwrap();
        attribution["expected_revision"]=3.into();attribution["event_id"]="object-equal".into();
        f.call("blp-object-attribute",attribution,"fixture-admin-token").unwrap();
        assert_eq!(object_reader(&f,"fixture-admin-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    }
}

/// Reject a retained cross-revision downgrade at both administration and reader
/// boundaries. Only this owned inert fixture is poisoned; originals are restored
/// exactly to demonstrate recovery without rewriting any production history.
#[test]
fn stored_cross_revision_downgrade_refuses_without_disclosure() {
    let (f,mut policy,mut attribution)=classified_object_fixture();
    policy["expected_revision"]=1.into();policy["event_id"]="object-policy-two".into();
    f.call("blp-policy-apply",policy,"fixture-admin-token").unwrap();
    attribution["expected_revision"]=2.into();attribution["event_id"]="stored-equal-control".into();
    f.call("blp-object-attribute",attribution,"fixture-admin-token").unwrap();
    assert_eq!(object_reader(&f,"fixture-admin-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    assert_ne!(object_reader(&f,"fixture-approver-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    let before=f.state();
    let mut stored:Value=serde_json::from_slice(&before).unwrap();
    let history=stored["confidentiality"]["history"].as_array_mut().unwrap();
    let mut lower=history[0]["attributions"][0].clone();
    lower["policy_revision"]=2.into();lower["level"]="public".into();lower["compartments"]=json!([]);
    history[1]["attributions"]=json!([lower]);
    let corrupted=serde_json::to_vec_pretty(&stored).unwrap();
    f.rewrite_state(|_|corrupted.clone());
    assert!(f.call("blp-show",json!({}),"fixture-admin-token").unwrap_err().contains("blp-attribution-history-downgrade"));
    assert!(object_reader(&f,"fixture-approver-token").unwrap_err().contains("blp-attribution-history-downgrade"));
    assert_eq!(f.state(),corrupted,"validation must not rewrite historical evidence");
    f.rewrite_state(|_|before.clone());
    assert!(f.call("blp-show",json!({}),"fixture-admin-token").is_ok());
    assert_eq!(object_reader(&f,"fixture-admin-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
    assert_ne!(object_reader(&f,"fixture-approver-token").unwrap(),swarm_engine::ReaderVerdict::Allowed);
}

/// Defect 3, sub-case (c): the producer contract announces `classification:
/// null` as "the object is unclassified", but the stored attribution gets an
/// EMPTY level string, and a reader presenting `Some(empty)` to
/// `check_read` meets `blp-classification-invalid` — an incoherent success
/// at production turning into an always-refused attribution at consumption.
/// The smallest public-path witness: an unclassified attribution either
/// carries a real catalog level (explicit decision) or the producer refuses
/// the null classification typed; it must never store an empty level that
/// can never be read back coherently. This test FAILS before the fix.
#[test]
fn unclassified_attribution_never_stores_an_unreadable_empty_level() {
    let f=Fixture::new(); f.apply();
    let mut renewed=change(1,"blp-renewal-admin-clearance");renewed["expected_trust_revision"]=1.into();
    renewed["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":[]});
    f.call("blp-policy-apply",renewed,"fixture-admin-token").unwrap();
    let digest="c".repeat(64);
    let before=f.state();
    let input=json!({"schema_version":1,"event_id":"blp-attribute-null","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest,
        "project_path":f.0.to_str().unwrap(),
        "classification":Value::Null,
        "reason":"Unclassified control attribution fixture"});
    let result=f.call("blp-object-attribute",input,"fixture-admin-token");
    match result {
        Err(refusal)=>assert!(refusal.contains("blp-attribute-classification-required"),
            "a null classification refuses typed, got: {refusal}"),
        Ok(_reply)=>{
            // If admitted, the stored level MUST be a real catalog level that
            // the reader guard can evaluate coherently.
            let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap();
            let stored:Vec<&Value>=view["record"]["history"].as_array().unwrap().iter()
                .flat_map(|e|e["attributions"].as_array()).flatten()
                .filter(|a|a["canonical_digest"]==digest.as_str()).collect();
            assert_eq!(stored.len(),1,"the null classification admits exactly one attribution");
            let level=stored[0]["level"].as_str().unwrap_or("");
            assert!(!level.is_empty(),
                "stored level must never be the unreadable empty string; got {level:?}");
        }
    }
    let _=before;
}

/// Defect 3, sub-case (b): replay freshness of an attribution event must be
/// bound to the exact recorded event identity — the replayed event's OWN
/// attribution, bound to its digest, path, revision and label — never to any
/// attribution that merely shares the free-text `reason`. Public-path note:
/// for a WELL-FORMED state the entry guards plus the event fingerprint
/// (which pins `expected_revision`) make the event's own effect always be on
/// the current entry, so the reason-match line is only reachable on a
/// HISTORY that no supported producer can produce today. The honest
/// discriminator is therefore the storage contract itself: a test-owned
/// poisoned history (written only inside this test's fixture root, by direct
/// file publication under the trust lock protocol) where the current entry
/// holds another object's attribution with the same reason while event A's
/// own attribution is ABSENT. Reopening and replaying A must refuse
/// `blp-replay-stale` — the poisoned state is rejected at open, and no
/// replay revives an event whose recorded effect is not stored. Before the
/// fix the free-text reason match accepts the replay despite the absent own
/// effect: the FAILING witness.
#[test]
fn attribution_replay_is_not_freshened_by_another_objects_reason() {
    let f=Fixture::new(); f.apply();
    let mut renewed=change(1,"blp-renewal-admin-clearance");renewed["expected_trust_revision"]=1.into();
    renewed["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":[]});
    f.call("blp-policy-apply",renewed,"fixture-admin-token").unwrap();
    let digest_a="d".repeat(64);
    let digest_b="e".repeat(64);
    let shared_reason="Shared free-text reason of two different object attributions";
    let path=f.0.to_str().unwrap();
    let first=json!({"schema_version":1,"event_id":"blp-attribute-object-a","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest_a,
        "project_path":path,
        "classification":{"level":"confidential","compartments":[]},
        "reason":shared_reason});
    f.call("blp-object-attribute",first,"fixture-admin-token")
        .unwrap_or_else(|e|panic!("first attribute refused: {e}"));
    let second=json!({"schema_version":1,"event_id":"blp-attribute-object-b","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest_b,
        "project_path":path,
        "classification":{"level":"confidential","compartments":[]},
        "reason":shared_reason});
    f.call("blp-object-attribute",second,"fixture-admin-token")
        .unwrap_or_else(|e|panic!("second attribute refused: {e}"));
    // Poison the TEST-OWNED fixture history only: drop event A's own
    // attribution from the current entry, keep object B's attribution (same
    // free-text reason) and keep A's retained reply/digest intact. This is
    // exactly the history shape the reason-match freshness check wrongly
    // accepts, and no supported producer can produce it.
    let poison=|bytes:Vec<u8>|{
        let mut st:Value=serde_json::from_slice(&bytes).unwrap();
        let history=st["confidentiality"]["history"].as_array_mut().unwrap();
        let current=history.last_mut().unwrap();
        let attributions=current["attributions"].as_array_mut().unwrap();
        attributions.retain(|a|a["canonical_digest"]!=digest_a.as_str());
        assert_eq!(attributions.len(),1,"poisoned history keeps only object B's attribution");
        serde_json::to_vec_pretty(&st).unwrap()
    };
    f.rewrite_state(poison);
    // Reopen (RootStore::open decodes and validates) then replay A exactly.
    let replay=json!({"schema_version":1,"event_id":"blp-attribute-object-a","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest_a,
        "project_path":path,
        "classification":{"level":"confidential","compartments":[]},
        "reason":shared_reason});
    let result=f.call("blp-object-attribute",replay,"fixture-admin-token");
    match result {
        Err(refusal)=>assert!(refusal.contains("blp-replay-stale")||refusal.contains("trust-proof-integrity"),
            "a replay whose own effect is not the recorded one refuses stale or at storage validation, got: {refusal}"),
        Ok(reply)=>panic!("replay accepted through another object's free-text reason: {reply}"),
    }
    // The poisoned history must not have been rewritten into a fabricated
    // success: A's own attribution is still absent after the refusal.
    let view=f.call("blp-show",json!({}),"fixture-admin-token").unwrap();
    let stored:Vec<&Value>=view["record"]["history"].as_array().unwrap().last().unwrap()
        ["attributions"].as_array().unwrap().iter()
        .filter(|a|a["canonical_digest"]==digest_a.as_str()).collect();
    assert!(stored.is_empty(),"a refused replay must not restore its own attribution");
}

/// Defect 3, sub-case (a): the storage/reopen contract. `Record::validate`
/// previously checked NO attribution field, so a persisted attribution with
/// an unknown/empty level, a mismatched policy revision or a malformed
/// digest/path/actor decoded fine and only surfaced (if ever) as an
/// incoherent refusal at the reader (`blp-classification-invalid`). Since
/// the supported producer can no longer produce invalid attributions, the
/// honest red witness is the storage contract itself: a test-owned poisoned
/// history, published through the same lock/atomic-rename protocol, whose
/// current entry carries attributions that violate the exact-once binding
/// contract. Reopening must refuse typed (integrity) at decode/validate —
/// never silently accept incoherent stored labels. Variants: unknown level,
/// empty level, wrong policy_revision, bad digest shape, non-absolute path,
/// duplicated same-triple attribution (contradicting pair).
#[test]
fn poisoned_stored_attribution_refuses_at_reopen() {
    let f=Fixture::new(); f.apply();
    let mut renewed=change(1,"blp-renewal-admin-clearance");renewed["expected_trust_revision"]=1.into();
    renewed["policy"]["clearances"]["admin"]=json!({"level":"confidential","compartments":[]});
    f.call("blp-policy-apply",renewed,"fixture-admin-token").unwrap();
    let digest="a".repeat(64);
    let path=f.0.to_str().unwrap();
    let first=json!({"schema_version":1,"event_id":"blp-attribute-poison-baseline","expected_revision":2,
        "expected_trust_revision":1,"canonical_digest":digest,
        "project_path":path,
        "classification":{"level":"confidential","compartments":[]},
        "reason":"Baseline attribution before poisoning variants fixture"});
    f.call("blp-object-attribute",first,"fixture-admin-token")
        .unwrap_or_else(|e|panic!("baseline attribute refused: {e}"));
    // Each variant poisons ONE field of the stored attribution in the
    // test-owned history; after each rewrite the whole root must refuse.
    let variants:Vec<(&str,Box<dyn Fn(&mut Value)>)>=vec![
        ("unknown-level",Box::new(|a:&mut Value|{a["level"]="top-secret-unknown".into();})),
        ("empty-level",Box::new(|a:&mut Value|{a["level"]="".into();})),
        ("wrong-policy-revision",Box::new(|a:&mut Value|{a["policy_revision"]=999.into();})),
        ("bad-digest-shape",Box::new(|a:&mut Value|{a["canonical_digest"]="ZZ".repeat(32).into();})),
        ("relative-path",Box::new(|a:&mut Value|{a["project_path"]=serde_json::to_value(
            "relative/path".as_bytes().to_vec()).unwrap();})),
        ("empty-actor",Box::new(|a:&mut Value|{a["actor"]="".into();})),
        ("contradicting-second",Box::new(|_a:&mut Value|{})),
    ];
    // The clean baseline captured BEFORE the loop. A refusal never mutates,
    // so the poisoned bytes stay published after each variant: restore the
    // clean baseline explicitly before each next poisoning.
    let baseline=f.state();
    for (name,poison_field) in variants {
        let digest=digest.clone();
        let clean=baseline.clone();
        let expected=baseline.clone();
        let restore=move |bytes:Vec<u8>|{
            let _=&bytes;
            clean.clone()
        };
        f.rewrite_state(restore);
        let poisoned=move |bytes:Vec<u8>|{
            assert_eq!(bytes,expected,"each variant starts from the clean baseline");
            let mut st:Value=serde_json::from_slice(&bytes).unwrap();
            let history=st["confidentiality"]["history"].as_array_mut().unwrap();
            let current=history.last_mut().unwrap();
            let attributions=current["attributions"].as_array_mut().unwrap();
            if name=="contradicting-second" {
                let mut second=attributions[0].clone();
                second["level"]="public".into();
                second["reason"]="Contradicting pair injected in the poisoned history fixture".into();
                attributions.push(second);
            } else {
                let target=attributions.iter_mut()
                    .find(|a|a["canonical_digest"]==digest.as_str())
                    .unwrap_or_else(||panic!("variant {name}: baseline attribution not found"));
                poison_field(target);
            }
            serde_json::to_vec_pretty(&st).unwrap()
        };
        f.rewrite_state(poisoned);
        // Reopen + any read: decode/validate must refuse the poisoned history.
        let result=f.call("blp-show",json!({}),"fixture-admin-token");
        assert!(matches!(&result,Err(refusal) if refusal.contains("trust-proof-integrity")),
            "poisoned variant {name} must refuse at reopen with an integrity cause, got: {result:?}");
    }
}
