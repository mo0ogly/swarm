//! P02 through the public Rust RootStore surface, with fictional invoices only.
#![cfg(target_os = "linux")]
use serde_json::json;
use std::{num::NonZeroUsize, path::Path, time::Duration};
use swarm_engine::{
    P02Context, P02Grant, P02InvoiceInput, RootStore, RootStoreError, SystemEnvironment,
};

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
    let resolved = first.resolve(&a, &f001, RESOLVER);
    assert_eq!(resolved.as_ref().unwrap().as_bytes(), ACCOUNT_A);
    assert!(
        format!("{resolved:?}") == "Ok(P02ResolvedValue([redacted]))",
        "successful P02 resolution must redact Debug"
    );
    assert_eq!(
        first.resolve(&a, &f003, RESOLVER).unwrap().as_bytes(),
        ACCOUNT_B
    );
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
    assert_eq!(
        first.resolve(&a, &f001, RESOLVER).unwrap().as_bytes(),
        ACCOUNT_A
    );
    assert!(first.revoke(&f001, ISSUER).unwrap());
    assert!(!first.revoke(&f001, ISSUER).unwrap());
    denied(first.resolve(&a, &f002, RESOLVER));
    assert_eq!(first.tokenize(&a, ACCOUNT_B, ISSUER).unwrap(), f003);
    assert!(matches!(
        first.tokenize(&a, ACCOUNT_A, ISSUER),
        Err(RootStoreError::Refused("p02-vault-capacity"))
    ));
    assert_eq!(
        first.resolve(&a, &f003, RESOLVER).unwrap().as_bytes(),
        ACCOUNT_B
    );
    first.revoke_mandate(ISSUER).unwrap();
    denied(first.resolve(&a, &f003, RESOLVER));
    denied(first.tokenize(&a, ACCOUNT_B, ISSUER));
    assert_eq!(
        bytes_before,
        std::fs::read(root_path.join(".swarm/trust-actions.json")).unwrap()
    );
    assert_eq!(
        second.resolve(&b, &f004, RESOLVER).unwrap().as_bytes(),
        ACCOUNT_A
    );
    configure(&root_path, 1);
    denied(second.resolve(&b, &f004, RESOLVER));
    drop((first, second));
    store.close().unwrap();

    let reopened = RootStore::open(&root_path, false).unwrap();
    let mut fresh = reopened.open_p02_vault(grant("batch-b"), ISSUER).unwrap();
    denied(fresh.resolve(&b, &f004, RESOLVER));
    let new = fresh.tokenize(&b, ACCOUNT_A, ISSUER).unwrap();
    assert_eq!(
        fresh.resolve(&b, &new, RESOLVER).unwrap().as_bytes(),
        ACCOUNT_A
    );
    drop(fresh);
    reopened.close().unwrap();
}

/// Observe the serialized input and the inert draft receiver on fictional
/// invoices. Only an authorized engine comparison reaches that receiver.
#[test]
fn fictional_invoice_projection_and_draft_are_closed_and_mandate_bound() {
    let work = tempfile::tempdir().unwrap();
    let root = work.path();
    RootStore::initialize(root, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    configure(root, 0);
    let store = RootStore::open(root, false).unwrap();
    let a = grant("batch-a").context;
    let b = grant("batch-b").context;
    let inputs = [
        P02InvoiceInput {
            invoice_id: "f001",
            account_reference: ACCOUNT_A,
        },
        P02InvoiceInput {
            invoice_id: "f002",
            account_reference: ACCOUNT_A,
        },
        P02InvoiceInput {
            invoice_id: "f003",
            account_reference: ACCOUNT_B,
        },
    ];
    let bad_tail = [
        P02InvoiceInput {
            invoice_id: "f001",
            account_reference: ACCOUNT_A,
        },
        P02InvoiceInput {
            invoice_id: "f002",
            account_reference: b"",
        },
    ];
    assert!(store
        .prepare_p02_invoice_batch(grant("batch-a"), &bad_tail, ISSUER)
        .is_err());
    assert!(store
        .prepare_p02_invoice_batch(grant("batch-a"), &[], ISSUER)
        .is_err());
    denied(store.prepare_p02_invoice_batch(grant("batch-a"), &inputs, RESOLVER));

    let duplicate = [
        P02InvoiceInput {
            invoice_id: "f001",
            account_reference: ACCOUNT_A,
        },
        P02InvoiceInput {
            invoice_id: "f001",
            account_reference: ACCOUNT_B,
        },
    ];
    assert!(matches!(
        store.prepare_p02_invoice_batch(grant("batch-a"), &duplicate, ISSUER),
        Err(RootStoreError::Refused("p02-invoice-identifiers"))
    ));
    let trust_before = std::fs::read(root.join(".swarm/trust-actions.json")).unwrap();
    let mut batch = store
        .prepare_p02_invoice_batch(grant("batch-a"), &inputs, ISSUER)
        .unwrap();
    let other = store
        .prepare_p02_invoice_batch(grant("batch-b"), &inputs[..1], ISSUER)
        .unwrap();
    denied(batch.project_for_agent(&a, RESOLVER));
    denied(batch.project_for_agent(&b, ISSUER));
    let agent_json = serde_json::to_value(batch.project_for_agent(&a, ISSUER).unwrap()).unwrap();
    let cross = serde_json::to_value(other.project_for_agent(&b, ISSUER).unwrap()).unwrap();
    let shared = &agent_json[0]["account_token"];
    assert_eq!(
        agent_json[0],
        json!({"invoice_id":"f001","account_token":shared})
    );
    assert_eq!(agent_json[1]["account_token"], *shared);
    assert_ne!(agent_json[2]["account_token"], *shared);
    assert_ne!(cross[0]["account_token"], *shared);
    assert!(!agent_json
        .to_string()
        .contains("fictitious-invoice-account"));

    let mut wrong_audience = a.clone();
    wrong_audience.audience = "foreign-receiver".into();
    assert_eq!(
        batch
            .resolve_account(&a, "f001", RESOLVER)
            .unwrap()
            .as_bytes(),
        ACCOUNT_A
    );
    denied(batch.resolve_account(&a, "f001", ISSUER));
    let mut received = Vec::new();
    {
        let mut deliver = |context, invoice, reference, credential| {
            let draft = batch.prepare_draft(context, invoice, reference, credential)?;
            received.push(serde_json::to_value(draft).unwrap());
            Ok::<(), RootStoreError>(())
        };
        deliver(&a, "f002", "f001", RESOLVER).unwrap();
        denied(deliver(&wrong_audience, "f002", "f001", RESOLVER));
        denied(deliver(&a, "f002", "f001", ISSUER));
        assert!(deliver(&a, "f002", "missing-invoice", RESOLVER).is_err());
        assert!(deliver(&a, "f002", "f002", RESOLVER).is_err());
    }
    assert_eq!(
        received,
        vec![json!({"invoice_id":"f002","account_matches":true})]
    );
    let different = batch.prepare_draft(&a, "f003", "f001", RESOLVER).unwrap();
    assert_eq!(
        serde_json::to_value(different).unwrap(),
        json!({"invoice_id":"f003","account_matches":false})
    );
    assert!(batch.revoke_account("f001", ISSUER).unwrap());
    denied(batch.project_for_agent(&a, ISSUER));
    denied(batch.prepare_draft(&a, "f003", "f002", RESOLVER));
    denied(batch.resolve_account(&a, "f002", RESOLVER));
    assert_eq!(
        batch
            .resolve_account(&a, "f003", RESOLVER)
            .unwrap()
            .as_bytes(),
        ACCOUNT_B
    );
    batch.revoke_mandate(ISSUER).unwrap();
    denied(batch.resolve_account(&a, "f003", RESOLVER));
    assert_eq!(
        trust_before,
        std::fs::read(root.join(".swarm/trust-actions.json")).unwrap()
    );
    assert_eq!(
        other
            .resolve_account(&b, "f001", RESOLVER)
            .unwrap()
            .as_bytes(),
        ACCOUNT_A
    );
    configure(root, 1);
    denied(other.project_for_agent(&b, ISSUER));
    denied(other.resolve_account(&b, "f001", RESOLVER));
    drop((batch, other));
    store.close().unwrap();
}

/// Connect to a distinct fixture subprocess over a private kernel socket.
/// The deadline bounds this socket operation if the test receiver stalls.
/// The returned child owns the other socket end and its observed draft.
fn draft_receiver(
    context: &P02Context,
    mode: &str,
    expected_invoice: &str,
) -> (swarm_engine::P02DraftDestination, std::process::Child) {
    use std::{
        os::{fd::OwnedFd, unix::net::UnixStream},
        process::{Command, Stdio},
    };

    let (sender, receiver) = UnixStream::pair().unwrap();
    sender
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    sender
        .set_write_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let child = Command::new("python3")
        .args([
            "-c",
            include_str!("fixtures/p02_draft_receiver.py"),
            mode,
            expected_invoice,
        ])
        .stdin(Stdio::from(OwnedFd::from(receiver.try_clone().unwrap())))
        .stdout(Stdio::from(OwnedFd::from(receiver)))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    (
        swarm_engine::P02DraftDestination {
            stream: sender,
            context: context.clone(),
            reference_invoice_id: "reference".into(),
        },
        child,
    )
}

/// Exercise the engine's sender and a separate receiver on fictional data.
/// The receiver readback distinguishes actual receipt from acknowledgement.
#[test]
fn fictional_draft_crosses_socket_and_requires_an_exact_receipt() {
    use sha2::{Digest, Sha256};
    use swarm_engine::P02DraftDeliveryError as DeliveryError;

    let work = tempfile::tempdir().unwrap();
    RootStore::initialize(work.path(), &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    configure(work.path(), 0);
    let store = RootStore::open(work.path(), false).unwrap();
    let mandate = grant("transport-fixture");
    let context = mandate.context.clone();
    let mut batch = store
        .prepare_p02_invoice_batch(
            mandate,
            &[
                P02InvoiceInput {
                    invoice_id: "reference",
                    account_reference: ACCOUNT_A,
                },
                P02InvoiceInput {
                    invoice_id: "equal",
                    account_reference: ACCOUNT_A,
                },
                P02InvoiceInput {
                    invoice_id: "different",
                    account_reference: ACCOUNT_B,
                },
            ],
            ISSUER,
        )
        .unwrap();

    for (mode, subject, expected) in [
        ("accept", "equal", true),
        ("empty", "equal", true),
        ("truncated", "equal", true),
        ("trailing", "equal", true),
        ("mismatch", "equal", true),
        ("accept", "different", false),
    ] {
        let (destination, peer) = draft_receiver(&context, mode, subject);
        let result = batch.deliver_draft(destination, subject, RESOLVER);
        let output = peer.wait_with_output().unwrap();
        assert!(output.status.success(), "fixture receiver failed: {mode}");
        let expected_draft = json!({"invoice_id": subject, "account_matches": expected});
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap(),
            expected_draft
        );
        if mode == "accept" {
            let expected_bytes =
                format!(r#"{{"invoice_id":"{subject}","account_matches":{expected}}}"#);
            let expected_digest: [u8; 32] = Sha256::digest(expected_bytes).into();
            assert_eq!(result.unwrap().sha256, expected_digest);
        } else {
            assert!(
                matches!(result, Err(DeliveryError::InvalidReceipt)),
                "{mode}: {result:?}"
            );
        }
    }

    let mut wrong_audience = context.clone();
    wrong_audience.audience = "foreign-receiver".into();
    let (destination, peer) = draft_receiver(&wrong_audience, "no-draft", "");
    let Err(DeliveryError::Preparation(error)) =
        batch.deliver_draft(destination, "equal", RESOLVER)
    else {
        panic!("wrong audience reached transport");
    };
    denied::<()>(Err(error));
    let output = peer.wait_with_output().unwrap();
    assert!(output.status.success() && output.stderr.is_empty());

    use std::io::Read;
    for read_deadline in [true, false] {
        let (stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let deadline = Some(Duration::from_secs(30));
        if read_deadline {
            stream.set_read_timeout(deadline).unwrap();
        } else {
            stream.set_write_timeout(deadline).unwrap();
        }
        let destination = swarm_engine::P02DraftDestination {
            stream,
            context: context.clone(),
            reference_invoice_id: "reference".into(),
        };
        assert!(matches!(
            batch.deliver_draft(destination, "equal", RESOLVER),
            Err(DeliveryError::MissingDeadline)
        ));
        let mut ignored = [0; 1];
        assert_eq!(peer.read(&mut ignored).unwrap(), 0);
    }

    let (stream, closed_peer) = std::os::unix::net::UnixStream::pair().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    drop(closed_peer);
    let destination = swarm_engine::P02DraftDestination {
        stream,
        context: context.clone(),
        reference_invoice_id: "reference".into(),
    };
    assert!(matches!(
        batch.deliver_draft(destination, "equal", RESOLVER),
        Err(DeliveryError::Unconfirmed(error)) if error.kind() == std::io::ErrorKind::BrokenPipe
    ));

    assert!(batch.revoke_account("reference", ISSUER).unwrap());
    let (destination, peer) = draft_receiver(&context, "no-draft", "");
    let Err(DeliveryError::Preparation(error)) =
        batch.deliver_draft(destination, "different", RESOLVER)
    else {
        panic!("revoked reference reached transport");
    };
    denied::<()>(Err(error));
    let output = peer.wait_with_output().unwrap();
    assert!(output.status.success() && output.stderr.is_empty());
    drop(batch);
    store.close().unwrap();
}

/// Confirm that malformed wire inputs cannot yield a fixture acknowledgement.
#[test]
fn fictional_receiver_rejects_nonobject_and_oversized_drafts() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let witness = br#"{"invoice_id":"equal","account_matches":true}"#;
    let mut oversized = witness.to_vec();
    oversized.resize(257, b' ');
    let array = br#"[["invoice_id","equal"],["account_matches",true]]"#;
    for (data, expected) in [
        (witness.as_slice(), None),
        (array.as_slice(), Some("draft must be a JSON object")),
        (oversized.as_slice(), Some("oversized draft")),
    ] {
        let mut child = Command::new("python3")
            .args([
                "-c",
                include_str!("fixtures/p02_draft_receiver.py"),
                "accept",
                "equal",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(data).unwrap();
        let output = child.wait_with_output().unwrap();
        if let Some(expected) = expected {
            assert!(!output.status.success() && output.stdout.is_empty());
            assert_eq!(
                std::str::from_utf8(&output.stderr).unwrap().trim(),
                expected
            );
        } else {
            assert!(output.status.success());
            assert_eq!(output.stdout.len(), 32);
        }
    }
}
