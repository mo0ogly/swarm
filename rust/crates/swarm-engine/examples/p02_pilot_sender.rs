//! P02 through the public Rust RootStore surface, with fictional invoices only.
use serde_json::json;
use std::{num::NonZeroUsize, path::Path, time::Duration};
use swarm_engine::{P02Context, P02Grant, P02InvoiceInput, RootStore, SystemEnvironment};

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

/// Run the genuine Swarm sender against one host-verified inherited socket.
/// The root and entire data set belong to a new fictional qualification run.
fn main() {
    use std::{
        os::{fd::AsFd, unix::net::UnixStream},
        time::Instant,
    };
    use swarm_engine::{P02DraftDeliveryError, P02DraftDestination};
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3, "expected fictional invoice and fixture case");
    // The supervisor passes an already connected socket as stdin. Clone the
    // descriptor into owned Rust state and create a new disposable Root.
    let stream = UnixStream::from(std::io::stdin().as_fd().try_clone_to_owned().unwrap());
    let scratch = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let root = scratch.path();
    let deadline = Some(Duration::from_secs(5));
    stream.set_read_timeout(deadline).unwrap();
    stream.set_write_timeout(deadline).unwrap();
    RootStore::initialize(root, &mut SystemEnvironment)
        .unwrap()
        .close()
        .unwrap();
    configure(root, 0);
    let store = RootStore::open(root, false).unwrap();
    let mut mandate = grant("qualification");
    let context = mandate.context.clone();
    let mode = args[2].as_str();
    if mode == "expired" {
        mandate.lifetime = Duration::from_millis(250);
    }
    let subject = &args[1];
    let mut destination = P02DraftDestination {
        stream,
        context,
        reference_invoice_id: "ref".into(),
    };
    let mut batch = store
        .prepare_p02_invoice_batch(
            mandate,
            &[
                P02InvoiceInput {
                    invoice_id: "ref",
                    account_reference: ACCOUNT_A,
                },
                P02InvoiceInput {
                    invoice_id: "same",
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
    let projection = serde_json::to_value(
        batch
            .project_for_agent(&destination.context, ISSUER)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        projection[0]["account_token"],
        projection[1]["account_token"]
    );
    assert_ne!(
        projection[0]["account_token"],
        projection[2]["account_token"]
    );
    for item in projection.as_array().unwrap() {
        let fields = item.as_object().unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields["invoice_id"].is_string() && fields["account_token"].is_string());
    }
    assert!(!projection
        .to_string()
        .contains("fictitious-invoice-account"));
    match mode {
        "valid" => {}
        "wrong-project" => destination.context.project = "other".into(),
        "wrong-batch" => destination.context.batch = "other".into(),
        "wrong-audience" => destination.context.audience = "other".into(),
        "wrong-reference" => destination.reference_invoice_id = "outside".into(),
        "revoked" => {
            assert!(batch.revoke_account("ref", ISSUER).unwrap());
        }
        "expired" => {
            let start = Instant::now();
            std::thread::sleep(Duration::from_millis(250));
            assert!(start.elapsed() >= Duration::from_millis(250));
        }
        _ => panic!("unknown fixture case"),
    }
    let observation = match batch.deliver_draft(destination, subject, RESOLVER) {
        Ok(receipt) => json!({"state":"confirmed","digest": receipt.sha256.iter()
            .map(|b| format!("{b:02x}")).collect::<String>()}),
        Err(P02DraftDeliveryError::Preparation(error)) => json!({
            "state":"refused_before_send",
            "cause": if let Some(failure) = error.trust_failure() { failure.code() } else { "comparison" }
        }),
        Err(P02DraftDeliveryError::InvalidReceipt | P02DraftDeliveryError::Unconfirmed(_)) => {
            json!({"state":"unconfirmed"})
        }
        Err(_) => panic!("unexpected transport configuration failure"),
    };
    println!("{observation}");
    drop(batch);
    store.close().unwrap();
}
