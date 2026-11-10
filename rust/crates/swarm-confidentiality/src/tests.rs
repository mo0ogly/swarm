//! Independent contract cases; these prove data-flow semantics, not a running
//! provider, authenticated actor, protected source or network enforcement.
use super::*;

fn label(level: &str, compartments: &[&str]) -> Classification {
    Classification { level: level.into(), compartments: compartments.iter().map(|s| (*s).into()).collect() }
}
fn policy() -> Policy {
    Policy { schema_version: 1, levels: ["public", "internal", "confidential", "restricted"].iter().map(|id|
        Level { id: (*id).into(), name_fr: (*id).into(), name_en: (*id).into() }).collect(),
        compartments: vec!["client-a".into(), "client-b".into()],
        clearances: BTreeMap::from([("worker".into(), label("restricted", &["client-a", "client-b"]))]),
        preparation_quarantine: false, provider_internet_denied: false }
}

#[test]
fn provider_internet_denial_is_explicit_and_preserves_legacy_encoding() {
    let policy = policy();
    let legacy = serde_json::to_value(&policy).unwrap();
    assert!(legacy.get("provider_internet_denied").is_none());
    let absent: Policy = serde_json::from_value(legacy.clone()).unwrap();
    assert!(!absent.provider_internet_denied);
    let mut explicit = legacy.clone(); explicit["provider_internet_denied"] = false.into();
    let off: Policy = serde_json::from_value(explicit).unwrap();
    assert_eq!(serde_json::to_vec(&off).unwrap(), serde_json::to_vec(&absent).unwrap());
    let mut on = legacy.clone(); on["provider_internet_denied"] = true.into();
    let on: Policy = serde_json::from_value(on).unwrap();
    assert!(on.provider_internet_denied); absent.validate_successor(&on).unwrap();
    on.validate_successor(&absent).unwrap();
    for value in [serde_json::Value::Null, "true".into(), 1.into()] {
        let mut invalid = legacy.clone(); invalid["provider_internet_denied"] = value;
        assert!(serde_json::from_value::<Policy>(invalid).is_err());
    }
}

#[test]
fn preparation_quarantine_is_explicit_and_preserves_legacy_encoding() {
    let policy = policy();
    let legacy = serde_json::to_value(&policy).unwrap();
    assert!(legacy.get("preparation_quarantine").is_none());
    let absent: Policy = serde_json::from_value(legacy.clone()).unwrap();
    assert!(!absent.preparation_quarantine);
    let mut explicit = legacy.clone(); explicit["preparation_quarantine"] = false.into();
    let off: Policy = serde_json::from_value(explicit).unwrap();
    assert_eq!(serde_json::to_vec(&off).unwrap(), serde_json::to_vec(&absent).unwrap());
    let mut on = legacy.clone(); on["preparation_quarantine"] = true.into();
    let on: Policy = serde_json::from_value(on).unwrap();
    assert!(on.preparation_quarantine); absent.validate_successor(&on).unwrap();
    on.validate_successor(&absent).unwrap();
    for value in [serde_json::Value::Null, "true".into(), 1.into()] {
        let mut invalid = legacy.clone(); invalid["preparation_quarantine"] = value;
        assert!(serde_json::from_value::<Policy>(invalid).is_err());
    }
}

#[test]
fn dominance_exhaustive_independent_matrix() {
    let p = policy(); p.validate().unwrap();
    let expected = [[true,false,false,false],[true,true,false,false],[true,true,true,false],[true,true,true,true]];
    let ids = ["public", "internal", "confidential", "restricted"];
    for (i,a) in ids.iter().enumerate() { for (j,b) in ids.iter().enumerate() {
        assert_eq!(p.dominates(&label(a,&[]), &label(b,&[])).unwrap(), expected[i][j]);
    }}
}

#[test]
fn same_level_distinct_compartments_are_incomparable() {
    let p = policy(); let a = label("confidential", &["client-a"]); let b = label("confidential", &["client-b"]);
    assert!(p.dominates(&a,&a).unwrap()); // valid control
    assert!(!p.dominates(&a,&b).unwrap()); assert!(!p.dominates(&b,&a).unwrap());
    assert!(p.dominates(&label("confidential", &["client-a","client-b"]),&a).unwrap());
}

#[test]
fn join_preserves_all_exposed_compartments_and_maximum_level() {
    let p = policy();
    let sources = vec![Some(label("internal", &["client-b"])), Some(label("restricted", &["client-a"]))];
    assert_eq!(p.join(&sources).unwrap(), label("restricted", &["client-a","client-b"]));
    let expected=p.join(&sources).unwrap();
    let mut reversed=sources; reversed.reverse(); assert_eq!(p.join(&reversed).unwrap(),expected);
}

#[test]
fn unknown_or_empty_context_never_becomes_public() {
    let p = policy(); assert!(p.join(&[Some(label("public", &[]))]).is_ok());
    assert_eq!(p.join(&[]), Err(ConfidentialityError::EmptyContext));
    assert_eq!(p.join(&[None]), Err(ConfidentialityError::Unclassified));
    assert_eq!(p.check_read("worker",None),Err(ConfidentialityError::Unclassified));
}

#[test]
fn catalog_and_labels_reject_unknowns_duplicates_and_malformed_input() {
    let p=policy(); p.validate().unwrap();
    for invalid in [label("invented", &[]),label("public", &["missing"]),label("public", &["client-a","client-a"])] {
        assert_eq!(p.dominates(&invalid,&label("public",&[])),Err(ConfidentialityError::InvalidClassification));
    }
    let mut invalid=p.clone(); invalid.levels.push(invalid.levels[0].clone()); assert!(invalid.validate().is_err());
    invalid=p.clone(); invalid.compartments.push("client-a".into()); assert!(invalid.validate().is_err());
    invalid=p.clone(); invalid.levels.clear(); assert!(invalid.validate().is_err());
    invalid=p; invalid.schema_version=2; assert!(invalid.validate().is_err());
    assert!(serde_json::from_str::<Classification>(r#"{"level":"public","compartments":[],"actor":"admin"}"#).is_err());
    assert!(serde_json::from_str::<Classification>(r#"{"level":"public","level":"restricted","compartments":[]}"#).is_err());
}

#[test]
fn no_read_up_no_write_down_and_no_compartment_loss() {
    let mut p=policy(); p.clearances.insert("low".into(),label("internal",&["client-a"]));
    assert!(p.check_read("low",Some(&label("public",&[]))).is_ok());
    assert_eq!(p.check_read("low",Some(&label("confidential",&[]))),Err(ConfidentialityError::AccessDenied));
    let d=ExecutionDomain::prepare(&p,1,"worker",&[Some(label("confidential",&["client-a"]))]).unwrap();
    assert!(d.check_output(&p,1,Some(&label("restricted",&["client-a","client-b"]))).is_ok());
    assert_eq!(d.check_output(&p,1,Some(&label("public",&[]))),Err(ConfidentialityError::AccessDenied));
    assert_eq!(d.check_output(&p,1,Some(&label("restricted",&["client-b"]))),Err(ConfidentialityError::AccessDenied));
}

#[test]
fn fixed_domain_does_not_use_maximum_clearance_for_later_reads() {
    let p=policy(); let d=ExecutionDomain::prepare(&p,1,"worker",&[Some(label("public",&[]))]).unwrap();
    assert!(d.check_input(Some(&label("public",&[]))).is_ok());
    assert_eq!(d.check_input(Some(&label("restricted",&[]))),Err(ConfidentialityError::AccessDenied));
    assert_eq!(d.output_classification(),&label("public",&[]));
}

#[test]
fn policy_change_and_revocation_require_new_execution() {
    let p=policy(); let d=ExecutionDomain::prepare(&p,1,"worker",&[Some(label("confidential",&[]))]).unwrap();
    assert!(d.check_output(&p,1,Some(&label("confidential",&[]))).is_ok());
    let mut changed=p.clone(); changed.clearances.remove("worker");
    assert_eq!(d.check_output(&changed,2,Some(&label("confidential",&[]))),Err(ConfidentialityError::PolicyChanged));
    assert!(ExecutionDomain::prepare(&changed,2,"worker",&[Some(label("confidential",&[]))]).is_err());
    assert!(d.check_output(&p,1,None).is_err());
}

#[test]
fn unrecognized_principal_learns_no_object_status_and_errors_are_content_free() {
    let p=policy();
    for source in [None,Some(label("confidential",&["client-a"]))] {
        let err=p.check_read("private-identity",source.as_ref()).unwrap_err();
        assert_eq!(err,ConfidentialityError::AccessDenied); assert_eq!(err.to_string(),"blp-access-denied");
    }
}

#[test]
fn successor_preserves_historical_catalog_semantics() {
    let p=policy(); let mut next=p.clone(); next.levels[0].name_fr="Public renommé".into();
    p.validate_successor(&next).unwrap();
    next.levels.swap(0,1); assert!(p.validate_successor(&next).is_err());
    next=p.clone(); next.compartments.remove(0); assert!(p.validate_successor(&next).is_err());
    next=p.clone(); next.levels.push(Level{id:"extra".into(),name_fr:"Extra".into(),name_en:"Extra".into()});
    p.validate_successor(&next).unwrap();
}
