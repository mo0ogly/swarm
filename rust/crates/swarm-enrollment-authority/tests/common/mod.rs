use swarm_enrollment_authority::Document;
use swarm_identity_contract::{Permission, Principal, Scope};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
fn at(raw: &str) -> OffsetDateTime {
    OffsetDateTime::parse(raw, &Rfc3339).unwrap()
}
pub fn document() -> Document {
    Document {
        schema_version: 1,
        enrollment_id: "enrollment-1".into(),
        generation: 1,
        project_digest: "0b".repeat(32),
        principal: Principal {
            id: "alice".into(),
            kind: "human".into(),
            issuer: "https://[::1]/issuer".into(),
            subject: "employee[eu]".into(),
        },
        scope: Scope {
            work_id: "work-1".into(),
            plan_version: 3,
            plan_digest: "b".repeat(64),
        },
        trust_revision: 2,
        permissions: vec![
            Permission {
                operation: "agent.create".into(),
                resource: "task/child".into(),
            },
            Permission {
                operation: "memory.read".into(),
                resource: "memory/team".into(),
            },
        ],
        delegable: vec![Permission {
            operation: "memory.read".into(),
            resource: "memory/team".into(),
        }],
        not_before: at("2026-10-17T08:00:00Z"),
        expires_at: at("2026-10-17T09:00:00Z"),
    }
}
