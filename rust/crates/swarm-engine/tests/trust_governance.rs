//! Public RootStore governance checks on owned isolated roots; no provider.
use serde_json::{json, Value};
use swarm_engine::{RootStore, SystemEnvironment};
struct Fixture(std::path::PathBuf);
impl Fixture {
    /// Initialize one owned private root; retain it on a failed assertion.
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(swarm_engine::generate_identifier("governance-").unwrap());
        std::fs::create_dir(&p).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        RootStore::initialize(&p, &mut SystemEnvironment)
            .unwrap()
            .close()
            .unwrap();
        Self(p)
    }
    /// Invoke the shared public contract and close the opened root before decode.
    fn call(&self, op: &str, v: &Value, token: &str) -> Value {
        let s = RootStore::open(&self.0, true).unwrap();
        let r = s.trust(op, &serde_json::to_vec(v).unwrap(), token);
        s.close().unwrap();
        use std::error::Error;
        let reply = r.unwrap_or_else(|e| {
            panic!(
                "{op}: {e}; {}",
                e.source().map(|s| s.to_string()).unwrap_or_default()
            )
        });
        serde_json::from_slice(&reply.bytes).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn config(id: &str, revision: i64) -> Value {
    json!({"schema_version":1,"event_id":id,"expected_revision":revision,"reason":"Isolated governance regression","policy":{"mandate_ttl_seconds":3600,"unknown_reconcile_seconds":0,"evidence_retention_days":365,"require_separation":true},"identities":[{"subject":"requester","token":"fixture-requester-token","integrity":1,"roles":["requester"]},{"subject":"approver","token":"fixture-approver-token","integrity":2,"roles":["approver"]},{"subject":"executor","token":"fixture-executor-token","integrity":2,"roles":["effect"]},{"subject":"admin","token":"fixture-admin-token","integrity":3,"roles":["admin"]}]})
}
fn policy() -> Value {
    json!({"enabled":true,"incompatible_roles":[["requester","approver"],["requester","effect"],["approver","effect"],["admin","effect"]],"principals":{"requester":"customer-operator","approver":"finance-operator","executor":"execution-service","admin":"policy-operator"}})
}
/// First prove legacy admission, then demand the new policy through preview,
/// save and a fresh root open. Preview must leave the exact state bytes intact.
#[test]
fn governance_policy_persists_and_preview_is_read_only() {
    let f = Fixture::new();
    let c = config("legacy", 0);
    assert_eq!(f.call("config-apply", &c, "")["revision"], 1);
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let mut c = config("governance", 1);
    c["role_separation"] = policy();
    assert_eq!(
        f.call("config-preview", &c, "")["role_separation"],
        policy()
    );
    assert_eq!(
        before,
        std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap()
    );
    assert_eq!(f.call("config-apply", &c, "")["role_separation"], policy());
    assert_eq!(f.call("show", &json!({}), "")["role_separation"], policy());
}

fn cmd(id: &str, v: Value) -> Value {
    let mut v = v;
    v["schema_version"] = 1.into();
    v["event_id"] = id.into();
    v
}

fn signing_config() -> Value {
    let mut c = config("signing-config", 0);
    c["evidence_signing"] = json!({"enabled":true,"key_ttl_seconds":91});
    c["identities"].as_array_mut().unwrap().push(json!({"subject":"second-admin","token":"fixture-second-admin-token","integrity":3,"roles":["admin"]}));
    c
}
fn key_change(id: &str, revision: u64, key: &str) -> Value {
    json!({"schema_version":1,"event_id":id,"expected_registry_revision":revision,"reason":"Isolated public signature check","key_id":key})
}
#[derive(Clone, serde::Serialize)]
struct ProofFixture {
    export: Box<serde_json::value::RawValue>,
    anchor: Value,
    trusted_key: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    trusted_checkpoint: Option<Value>,
}
/// Preserve exact nested policy/receipt bytes instead of reordering them through
/// serde_json::Value. A reordered raw-context export is a different signed input.
fn proof_bundle(v: &Value, key: &Value) -> ProofFixture {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let export = String::from_utf8(
        URL_SAFE_NO_PAD
            .decode(v["export_base64"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let anchor = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(v["anchor_base64"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    ProofFixture {
        export: serde_json::value::RawValue::from_string(export).unwrap(),
        anchor,
        trusted_key: key.clone(),
        trusted_checkpoint: None,
    }
}
/// Independent pins, exact prefix witnesses and current revoked records must
/// discriminate valid signatures from altered, old and untrusted input.
#[test]
fn governance_public_signatures_checkpoints_and_revocation() {
    let f = Fixture::new();
    f.call("config-apply", &signing_config(), "");
    let prepared = f.call(
        "evidence-key-prepare",
        &key_change("prepare", 0, ""),
        "fixture-admin-token",
    );
    let id = prepared["key_id"].as_str().unwrap();
    assert_eq!(prepared["runtime"]["private_available"][id], true);
    assert!(!nix::sys::prctl::get_dumpable().unwrap());
    let activate = key_change("activate", 1, id);
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let s = RootStore::open(&f.0, true).unwrap();
    assert!(s
        .trust(
            "evidence-key-activate",
            &serde_json::to_vec(&activate).unwrap(),
            "fixture-admin-token"
        )
        .is_err());
    s.close().unwrap();
    assert_eq!(
        before,
        std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap()
    );
    let activated = f.call(
        "evidence-key-activate",
        &activate,
        "fixture-second-admin-token",
    );
    let pin = activated["registry"]["keys"][id].clone();
    // Persist the public pin outside the producer root, then acquire it again.
    let pin_path =
        std::env::temp_dir().join(swarm_engine::generate_identifier("independent-pin-").unwrap());
    std::fs::write(&pin_path, serde_json::to_vec(&pin).unwrap()).unwrap();
    let independent: Value = serde_json::from_slice(&std::fs::read(&pin_path).unwrap()).unwrap();
    let bundle = f.call("public-anchor", &json!({}), "fixture-admin-token");
    let valid = proof_bundle(&bundle, &independent);
    let verify =
        |v: &ProofFixture| swarm_engine::verify_public_trust(&serde_json::to_vec(v).unwrap());
    assert_eq!(
        serde_json::from_slice::<Value>(&verify(&valid).unwrap()).unwrap()["checkpoint_verified"],
        false
    );
    let checkpoint = json!({"anchor":valid.anchor,"trusted_key":independent});
    let mut equal = valid.clone();
    equal.trusted_checkpoint = Some(checkpoint.clone());
    assert!(verify(&equal).is_ok());
    let export: Value = serde_json::from_str(valid.export.get()).unwrap();
    let (original, value) = export["originals"]
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let raw = serde_json::to_string(&valid).unwrap();
    let duplicate = raw.replacen(
        "\"originals\":{",
        &format!(
            "\"originals\":{{{}:{},",
            serde_json::to_string(original).unwrap(),
            serde_json::to_string(value).unwrap()
        ),
        1,
    );
    assert_eq!(
        swarm_engine::verify_public_trust(duplicate.as_bytes())
            .unwrap_err()
            .code(),
        "format"
    );
    let mut invalid = raw.into_bytes();
    let index = invalid.iter().position(|b| *b == b's').unwrap();
    invalid[index] = 0xff;
    assert_eq!(
        swarm_engine::verify_public_trust(&invalid)
            .unwrap_err()
            .code(),
        "format"
    );
    let mut expired = valid.clone();
    expired.trusted_key["expires_at"] = valid.anchor["signed_at"].clone();
    assert!(verify(&expired).is_err());
    let other = Fixture::new();
    for name in ["trust-actions.json", "trust-actions-proof.key"] {
        std::fs::copy(
            f.0.join(".swarm").join(name),
            other.0.join(".swarm").join(name),
        )
        .unwrap();
    }
    assert_eq!(
        other.call("show", &json!({}), "")["public_evidence_runtime"]["private_available"][id],
        false
    );
    let s = RootStore::open(&other.0, true).unwrap();
    assert!(s
        .trust("public-anchor", b"{}", "fixture-admin-token")
        .is_err());
    s.close().unwrap();
    let mut bad = valid.clone();
    let mut altered: Value = serde_json::from_str(bad.export.get()).unwrap();
    altered["evidence"][0]["actor"] = "other".into();
    bad.export =
        serde_json::value::RawValue::from_string(serde_json::to_string(&altered).unwrap()).unwrap();
    assert!(verify(&bad).is_err());
    let mut bad = valid.clone();
    bad.trusted_key["public_key"] = "wrong".into();
    assert!(verify(&bad).is_err());
    let mut bad = valid.clone();
    bad.trusted_checkpoint = Some(Value::Null);
    assert!(verify(&bad).is_err());
    let mut bad = valid.clone();
    bad.trusted_checkpoint = Some(json!({}));
    assert!(verify(&bad).is_err());
    let revoked = f.call(
        "evidence-key-revoke",
        &key_change("revoke", 2, id),
        "fixture-admin-token",
    );
    assert_eq!(revoked["runtime"]["private_available"][id], false);
    let mut current = valid.clone();
    current.trusted_key = revoked["registry"]["keys"][id].clone();
    assert!(verify(&current).is_err());
    assert!(
        verify(&valid).is_ok(),
        "an offline old pin cannot learn revocation"
    );
    let replay = f.call(
        "evidence-key-activate",
        &activate,
        "fixture-second-admin-token",
    );
    assert_eq!(replay["registry"]["keys"][id]["status"], "revoked");
    let s = RootStore::open(&f.0, true).unwrap();
    assert!(s
        .trust("public-anchor", b"{}", "fixture-admin-token")
        .is_err());
    s.close().unwrap();
    std::fs::remove_file(pin_path).unwrap();
}
/// Admit and request a positive control, preserving the original pending object.
fn pending(f: &Fixture) -> Value {
    let m = cmd(
        "mandate",
        json!({"mandate_id":"invoice","subject":"requester","actions":["settle"],"resource":"invoice/one","max_amount_minor":1000,"currency":"EUR","beneficiary":"supplier"}),
    );
    assert_eq!(
        f.call("mandate-create", &m, "fixture-admin-token")["state"],
        "admitted"
    );
    let a=f.call("request",&cmd("request",json!({"mandate_id":"invoice","resource":"invoice/one","amount_minor":450,"currency":"EUR","beneficiary":"supplier","idempotency_key":"payment-one"})),"fixture-requester-token");
    assert_eq!(a["state"], "pending_approval");
    a
}
/// Approve the exact pending action as a valid control for later effect checks.
fn approved(f: &Fixture) -> Value {
    let a = pending(f);
    // Use only admitted command fields, never the action response as input.
    let v = cmd(
        "approve",
        json!({"action_id":a["id"],"mandate_id":"invoice","resource":"invoice/one","content_digest":a["content_digest"],"amount_minor":450,"currency":"EUR","beneficiary":"supplier"}),
    );
    assert_eq!(
        f.call("approve", &v, "fixture-approver-token")["state"],
        "approved"
    );
    a
}

/// Invalid proposals must not mutate any state; omitted policy must preserve
/// the explicit saved policy instead of silently disabling it.
#[test]
fn governance_invalid_policy_refuses_and_omission_preserves() {
    let f = Fixture::new();
    let mut c = config("valid", 0);
    c["role_separation"] = policy();
    f.call("config-apply", &c, "");
    let mut bad = Vec::new();
    bad.push(Value::Null);
    let mut v = policy();
    v["enabled"] = Value::Null;
    bad.push(v);
    let mut v = policy();
    v["incompatible_roles"] = json!([["requester", "requester"]]);
    bad.push(v);
    let mut v = policy();
    v["incompatible_roles"] = json!([["requester", "unknown"]]);
    bad.push(v);
    let mut v = policy();
    v["incompatible_roles"] = json!([["requester", "effect"], ["effect", "requester"]]);
    bad.push(v);
    let mut v = policy();
    v["principals"].as_object_mut().unwrap().remove("executor");
    bad.push(v);
    let mut v = policy();
    v["principals"]["unknown"] = "someone".into();
    bad.push(v);
    let mut v = policy();
    v["hidden_fallback"] = true.into();
    bad.push(v);
    for (n, v) in bad.into_iter().enumerate() {
        let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
        let mut c = config(&format!("bad-{n}"), 1);
        c["role_separation"] = v;
        let s = RootStore::open(&f.0, true).unwrap();
        let r = s.trust("config-apply", &serde_json::to_vec(&c).unwrap(), "");
        s.close().unwrap();
        assert!(r.is_err(), "{n}");
        assert_eq!(
            before,
            std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap()
        );
    }
    let c = config("omitted", 1);
    assert_eq!(f.call("config-apply", &c, "")["role_separation"], policy());
}

/// Audit verifies a transaction without mutating it, distinguishes a pending
/// prefix from completion and reports the exact single recorded native effect.
#[test]
fn governance_audit_links_pending_and_completed_transaction() {
    let f = Fixture::new();
    let mut c = config("configure", 0);
    c["role_separation"] = policy();
    f.call("config-apply", &c, "");
    let a = approved(&f);
    let input = json!({"action_id":a["id"]});
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let audit = f.call("audit-transaction", &input, "fixture-admin-token");
    assert_eq!(audit["transaction_verified"], true);
    assert_eq!(audit["complete"], false);
    assert_eq!(audit["events"].as_array().unwrap().len(), 3);
    let recorded: Value =
        serde_json::from_slice(&std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap())
            .unwrap();
    let before: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(before["actions"], recorded["actions"]);
    assert_eq!(
        recorded["evidence"].as_array().unwrap().last().unwrap()["kind"],
        "audit-access"
    );
    assert_eq!(audit["access_recorded"], true);
    f.call(
        "commit",
        &cmd("commit", json!({"action_id":a["id"]})),
        "fixture-executor-token",
    );
    let audit = f.call("audit-transaction", &input, "fixture-admin-token");
    assert_eq!(audit["transaction_verified"], true);
    assert_eq!(audit["complete"], true);
    assert_eq!(audit["events"].as_array().unwrap().len(), 4);
    assert_eq!(audit["external_checkpoint_verified"], false);
    assert!(!serde_json::to_string(&audit)
        .unwrap()
        .contains("fixture-executor-token"));
    let s = RootStore::open(&f.0, true).unwrap();
    assert!(s
        .trust_as_principal(
            "audit-transaction",
            &serde_json::to_vec(&input).unwrap(),
            "fixture-requester-token"
        )
        .is_err());
    assert!(s
        .trust_as_principal(
            "audit-transaction",
            &serde_json::to_vec(&input).unwrap(),
            "fixture-admin-token"
        )
        .is_ok());
    s.close().unwrap();
}
/// Same-owner requester/executor and approver/executor combinations must refuse
/// despite distinct subjects/tokens; a valid effect is the control for each pair.
#[test]
fn governance_effect_conflicts_preserve_business_state() {
    for role in ["requester", "approver", "admin"] {
        for conflict in [false, true] {
            let f = Fixture::new();
            let mut c = config("configure", 0);
            let mut p = policy();
            if conflict {
                p["principals"]["executor"] = p["principals"][role].clone();
            }
            c["role_separation"] = p;
            f.call("config-apply", &c, "");
            let a = approved(&f);
            let before = f.call("show", &json!({}), "");
            let commit = cmd("commit", json!({"action_id":a["id"]}));
            let r = f.call("commit", &commit, "fixture-executor-token");
            assert_eq!(
                r["state"],
                if conflict { "denied" } else { "committed" },
                "{role}"
            );
            let after = f.call("show", &json!({}), "");
            if conflict {
                assert_eq!(before["actions"], after["actions"]);
                assert_eq!(after["ledger"], json!([]));
                assert_eq!(
                    after["evidence_count"].as_u64().unwrap(),
                    before["evidence_count"].as_u64().unwrap() + 1
                );
            } else {
                assert_eq!(after["ledger"].as_array().unwrap().len(), 1);
                assert_eq!(
                    f.call("commit", &commit, "fixture-executor-token")["state"],
                    "committed"
                );
                assert_eq!(
                    f.call("show", &json!({}), "")["ledger"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
            }
        }
    }
}

/// An admitted legacy resource alias must cross the exact same role boundary
/// as action_id. First exercise the alias with genuinely distinct principals.
#[test]
fn governance_review_alias_cannot_bypass_separation() {
    for conflict in [false, true] {
        let f = Fixture::new();
        let mut c = config("configure", 0);
        let mut p = policy();
        if conflict {
            p["principals"]["approver"] = p["principals"]["requester"].clone();
        }
        c["role_separation"] = p;
        f.call("config-apply", &c, "");
        let a = pending(&f);
        let approval = cmd(
            "alias",
            json!({"resource":a["id"],"mandate_id":"invoice","content_digest":a["content_digest"],"amount_minor":450,"currency":"EUR","beneficiary":"supplier"}),
        );
        let r = f.call("approve", &approval, "fixture-approver-token");
        assert_eq!(r["state"], if conflict { "denied" } else { "approved" });
    }
}
/// Revoking a mandate after a valid effect must not erase its historical
/// admission or turn a complete transaction into a false audit failure.
#[test]
fn governance_review_revocation_preserves_historical_audit() {
    let f = Fixture::new();
    let mut c = config("configure", 0);
    c["role_separation"] = policy();
    f.call("config-apply", &c, "");
    let a = approved(&f);
    f.call(
        "commit",
        &cmd("commit", json!({"action_id":a["id"]})),
        "fixture-executor-token",
    );
    let query = json!({"action_id":a["id"]});
    assert_eq!(
        f.call("audit-transaction", &query, "fixture-admin-token")["transaction_verified"],
        true
    );
    assert_eq!(
        f.call(
            "mandate-revoke",
            &cmd("revoke", json!({"mandate_id":"invoice"})),
            "fixture-admin-token"
        )["state"],
        "revoked"
    );
    assert_eq!(
        f.call("audit-transaction", &query, "fixture-admin-token")["transaction_verified"],
        true
    );
}

/// Reconciliation reads a previously authorized effect. First prove an ordinary
/// unknown recovery, then change principal groups without allowing a fresh write.
#[test]
fn governance_reconcile_survives_later_group_conflict() {
    for changed in [false,true] {
        let f=Fixture::new();let mut c=config("configure",0);c["role_separation"]=policy();f.call("config-apply",&c,"");
        let a=approved(&f);assert_eq!(f.call("commit",&cmd("lost",json!({"action_id":a["id"],"lose_response":true})),"fixture-executor-token")["state"],"unknown");
        if changed {let mut c=config("changed",1);let mut p=policy();p["principals"]["executor"]=p["principals"]["requester"].clone();c["role_separation"]=p;f.call("config-apply",&c,"");}
        let v=f.call("reconcile",&cmd("readback",json!({"action_id":a["id"]})),"fixture-executor-token");assert_eq!(v["state"],"reconciled");assert_eq!(f.call("show",&json!({}),"")["ledger"].as_array().unwrap().len(),1);
    }
}

/// A valid approval precedes a missing-target command. Enabling the matrix
/// must preserve its ordinary attributed business denial and unchanged effects.
#[test]
fn governance_review_new_missing_target_is_recorded() {
    let f=Fixture::new();let mut c=config("configure",0);c["role_separation"]=policy();f.call("config-apply",&c,"");let _=approved(&f);
    let before=f.call("show",&json!({}),"");let v=f.call("approve",&cmd("absent",json!({"action_id":"missing-action","mandate_id":"invoice","content_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","amount_minor":450,"currency":"EUR","beneficiary":"supplier"})),"fixture-approver-token");
    assert_eq!(v["state"],"denied");assert_eq!(v["reason"],"action absente");let after=f.call("show",&json!({}),"");assert_eq!(after["evidence_count"].as_u64().unwrap(),before["evidence_count"].as_u64().unwrap()+1);assert_eq!(after["actions"],before["actions"]);assert_eq!(after["ledger"],before["ledger"]);
}

/// An inherited public registry may be revoked in its own Root without touching
/// private material owned by another Root in the same persistent process.
#[test]
fn governance_review_new_foreign_root_revoke_is_independent() {
    let a=Fixture::new();a.call("config-apply",&signing_config(),"");let p=a.call("evidence-key-prepare",&key_change("prepare",0,""),"fixture-admin-token");let id=p["key_id"].as_str().unwrap();a.call("evidence-key-activate",&key_change("activate",1,id),"fixture-second-admin-token");a.call("public-anchor",&json!({}),"fixture-admin-token");
    let b=Fixture::new();for name in ["trust-actions.json","trust-actions-proof.key"]{std::fs::copy(a.0.join(".swarm").join(name),b.0.join(".swarm").join(name)).unwrap();}
    assert_eq!(b.call("show",&json!({}),"")["public_evidence_runtime"]["private_available"][id],false);
    assert_eq!(b.call("evidence-key-revoke",&key_change("revoke-copy",2,id),"fixture-admin-token")["registry"]["keys"][id]["status"],"revoked");
    assert_eq!(a.call("show",&json!({}),"")["public_evidence_runtime"]["private_available"][id],true);a.call("public-anchor",&json!({}),"fixture-admin-token");
}

/// Enrollment mapping declares roles only. It must disable the old local bearer
/// and reject two aliases of the same issuer/subject without changing storage.
#[test]
fn governance_enrollment_mapping_has_no_local_token_fallback() {
    let f = Fixture::new();
    f.call("config-apply", &config("local-witness", 0), "");
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let principal = json!({"principal_id":"policy-human","principal_kind":"human","issuer":"urn:fixture:identity","subject":"policy-user"});
    let mut c = config("enrollment-mapping", 1);
    c["identities"][3].as_object_mut().unwrap().remove("token");
    c["identities"][3]["enrollment_principal"] = principal.clone();
    assert_eq!(f.call("config-preview", &c, "")["applied"], false);
    assert_eq!(before, std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap());
    f.call("config-apply", &c, "");
    assert_eq!(f.call("show", &json!({}), "")["identities"][3]["enrollment_principal"], principal);
    let mut c = config("alias-refusal", 2);
    for i in [1,3] {
        c["identities"][i].as_object_mut().unwrap().remove("token");
        c["identities"][i]["enrollment_principal"] = principal.clone();
    }
    c["identities"][1]["enrollment_principal"]["principal_id"] = "another-alias".into();
    let s = RootStore::open(&f.0, true).unwrap();
    let before = std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap();
    let raw = serde_json::to_vec(&c).unwrap();
    assert!(s.trust_as_principal("config-preview", &raw, "fixture-admin-token").is_err());
    assert!(s.trust("config-preview", &raw, "").is_err());
    assert_eq!(before, std::fs::read(f.0.join(".swarm/trust-actions.json")).unwrap());
    s.close().unwrap();
}
