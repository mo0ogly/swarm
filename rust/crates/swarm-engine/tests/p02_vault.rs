//! P02 through the public Rust RootStore surface, with fictional invoices only.
#![cfg(target_os = "linux")]
use serde_json::json;
use std::{num::NonZeroUsize, path::Path, time::Duration};
use swarm_engine::{P02Context, P02Grant, RootStore, RootStoreError, SystemEnvironment};

const ISSUER: &str = "fixture-p02-issuer-token";
const RESOLVER: &str = "fixture-p02-resolver-token";
const OTHER: &str = "fixture-p02-other-token";
const ACCOUNT_A: &[u8] = b"fictitious-invoice-account-alpha";
const ACCOUNT_B: &[u8] = b"fictitious-invoice-account-beta";

/// Publish only synthetic local identities through the real Trust protocol.
fn configure(root: &Path, revision: i64) {
    let command = json!({
        "schema_version": 1,
        "event_id": format!("p02-config-{revision}"),
        "expected_revision": revision,
        "reason": "P02 fictional invoices",
        "policy": {
            "mandate_ttl_seconds": 3600, "unknown_reconcile_seconds": 0,
            "evidence_retention_days": 1, "require_separation": true
        },
        "identities": [
            {"subject": "issuer", "token": ISSUER, "integrity": 3, "roles": ["admin"]},
            {"subject": "resolver", "token": RESOLVER, "integrity": 2, "roles": ["effect"]},
            {"subject": "other", "token": OTHER, "integrity": 3, "roles": ["admin"]}
        ]
    });
    let writer = RootStore::open(root, true).unwrap();
    let reply = writer
        .trust("config-apply", &serde_json::to_vec(&command).unwrap(), "")
        .unwrap();
    assert_eq!(reply.publication, Some("recorded"));
    writer.close().unwrap();
}

/// Return explicit fixture terms. Production callers choose their own limits.
fn grant(batch: &str) -> P02Grant {
    P02Grant {
        context: P02Context {
            project: "fictional-invoices".into(),
            batch: batch.into(),
            audience: "local-receiver".into(),
        },
        resolver: "resolver".into(),
        lifetime: Duration::from_secs(3600),
        max_associations: NonZeroUsize::new(2).unwrap(),
        max_value_bytes: NonZeroUsize::new(64).unwrap(),
    }
}

/// Require a typed denial and check that the diagnostic omits fixture values.
fn denied<T>(result: Result<T, RootStoreError>) {
    let error = result.err().expect("P02 unexpectedly authorized");
    assert_eq!(
        error.trust_failure().unwrap().code(),
        "authorization_required"
    );
    assert!(!error.to_string().contains("fictitious-invoice-account"));
}

/// Exercise a positive/negative pair for every scope dimension and ensure
/// local revoke, mandate revoke, revision replacement and reopen fail closed.
#[test]
fn fictional_invoices_require_current_scoped_mandate_and_respect_revocation() {
    let work = tempfile::tempdir().unwrap();
    let root_path = work.path().to_path_buf();
    RootStore::initialize(&root_path, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    configure(&root_path, 0);
    let store = RootStore::open(&root_path, false).unwrap();
    denied(store.open_p02_vault(grant("batch-a"), RESOLVER));
    let mut invalid = grant("batch-a");
    invalid.resolver = "missing".into();
    denied(store.open_p02_vault(invalid, ISSUER));

    let a = grant("batch-a").context;
    let b = grant("batch-b").context;
    let mut first = store.open_p02_vault(grant("batch-a"), ISSUER).unwrap();
    let mut second = store.open_p02_vault(grant("batch-b"), ISSUER).unwrap();
    denied(first.tokenize(&a, ACCOUNT_A, RESOLVER));
    let bytes_before = std::fs::read(root_path.join(".swarm/trust-actions.json")).unwrap();
    let f001 = first.tokenize(&a, ACCOUNT_A, ISSUER).unwrap();
    let f002 = first.tokenize(&a, ACCOUNT_A, ISSUER).unwrap();
    let f003 = first.tokenize(&a, ACCOUNT_B, ISSUER).unwrap();
    let f004 = second.tokenize(&b, ACCOUNT_A, ISSUER).unwrap();
    let mut short = grant("expires");
    short.lifetime = Duration::from_nanos(1);
    let lease = short.lifetime;
    let expiry_context = short.context.clone();
    let mut expiring = store.open_p02_vault(short, ISSUER).unwrap();
    // The sleep begins after issuance and lasts a full synthetic lease.
    // Expiration must refuse before inserting any new correspondence.
    std::thread::sleep(lease);
    denied(expiring.tokenize(&expiry_context, ACCOUNT_A, ISSUER));
    drop(expiring);
    assert_eq!(f001, f002);
    assert_ne!(f001, f003);
    assert_ne!(f001, f004);
    assert_eq!(f001.len(), 43);
    assert!(!f001.contains("fictitious"));
    assert_eq!(&**first.resolve(&a, &f001, RESOLVER).unwrap(), ACCOUNT_A);
    assert_eq!(&**first.resolve(&a, &f003, RESOLVER).unwrap(), ACCOUNT_B);
    denied(first.resolve(&a, &f001, OTHER));
    denied(first.resolve(&a, &f001, ISSUER));
    denied(first.resolve(&b, &f001, RESOLVER));
    let mut wrong_project = a.clone();
    wrong_project.project = "other-project".into();
    denied(first.resolve(&wrong_project, &f001, RESOLVER));
    let mut wrong_audience = a.clone();
    wrong_audience.audience = "other-receiver".into();
    denied(first.resolve(&wrong_audience, &f001, RESOLVER));
    denied(second.resolve(&b, &f001, RESOLVER));
    denied(first.revoke(&f001, OTHER));
    assert_eq!(&**first.resolve(&a, &f001, RESOLVER).unwrap(), ACCOUNT_A);
    assert!(first.revoke(&f001, ISSUER).unwrap());
    assert!(!first.revoke(&f001, ISSUER).unwrap());
    denied(first.resolve(&a, &f002, RESOLVER));
    assert_eq!(first.tokenize(&a, ACCOUNT_B, ISSUER).unwrap(), f003);
    assert!(matches!(
        first.tokenize(&a, ACCOUNT_A, ISSUER),
        Err(RootStoreError::Refused("p02-vault-capacity"))
    ));
    assert_eq!(&**first.resolve(&a, &f003, RESOLVER).unwrap(), ACCOUNT_B);
    first.revoke_mandate(ISSUER).unwrap();
    denied(first.resolve(&a, &f003, RESOLVER));
    denied(first.tokenize(&a, ACCOUNT_B, ISSUER));
    assert_eq!(
        bytes_before,
        std::fs::read(root_path.join(".swarm/trust-actions.json")).unwrap()
    );
    assert_eq!(&**second.resolve(&b, &f004, RESOLVER).unwrap(), ACCOUNT_A);
    configure(&root_path, 1);
    denied(second.resolve(&b, &f004, RESOLVER));
    drop((first, second));
    store.close().unwrap();

    let reopened = RootStore::open(&root_path, false).unwrap();
    let mut fresh = reopened.open_p02_vault(grant("batch-b"), ISSUER).unwrap();
    denied(fresh.resolve(&b, &f004, RESOLVER));
    let new = fresh.tokenize(&b, ACCOUNT_A, ISSUER).unwrap();
    assert_eq!(&**fresh.resolve(&b, &new, RESOLVER).unwrap(), ACCOUNT_A);
    drop(fresh);
    reopened.close().unwrap();
}
