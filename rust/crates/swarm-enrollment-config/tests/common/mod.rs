use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use swarm_enrollment_config::{Change, Entry, Head, Pins, Snapshot, Values};
use swarm_identity_contract::Principal;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
pub fn at(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &Rfc3339).unwrap()
}
pub fn pins() -> Pins {
    Pins {
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
pub fn change(event: &str, expected_head: Head) -> Change {
    Change {
        schema_version: 1,
        event_id: event.into(),
        expected_head,
        values: Values {
            mode: "local".into(),
            socket_path: "/run/swarm-fixture/authority.sock".into(),
        },
        reason: "explicit fixture choice".into(),
    }
}
pub fn snapshot() -> Snapshot {
    let pins = pins();
    Snapshot {
        schema_version: 1,
        history: vec![Entry {
            change: change("initial", Head::default()),
            actor: pins.principal.clone(),
            at: at("2026-10-17T12:00:00Z"),
        }],
        pins,
    }
}
