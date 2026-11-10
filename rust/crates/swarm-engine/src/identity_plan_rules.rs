//! Generated from actual Go projection sources. No adoption or creation authority.
// internal/engine/agent_identity_plan_linux.go SHA-256 d4c19975145926a24b7e60df731034f92d52474fd7a8aac7baaba8730b916381
// internal/engine/agent_identity_plan_schema_linux.go SHA-256 1e25bd7910c2a5c037caa2eb63248a1ce7045aab2f4531af287d11b39cc1f4b0
pub const PROJECTION_VERSION: i64 = 1;
pub const UDF_NAME: &str = "swarm_identity_plan_digest_v1";
pub const IDENTITY_PLAN_HEADS_SQL: &str = include_str!("identity_plan_heads.sql");
pub(crate) const OMITS: &[(&str, &[&str])] = &[
    (
        "object",
        &[
            "revision",
            "updated",
            "git",
            "retex",
            "summary",
            "project_memory",
        ],
    ),
    (
        "planning",
        &["activations", "decisions", "failure", "inbox"],
    ),
    (
        "scope",
        &[
            "revision",
            "generation",
            "holder",
            "lease_until",
            "state",
            "activations",
            "last_model",
            "delivery",
            "workflow",
            "rejection_wait",
        ],
    ),
    (
        "task",
        &[
            "status",
            "blocker",
            "attempts",
            "gate",
            "auto_validation",
            "override",
            "revalidation",
            "evidence_stale_reason",
            "launch_held",
            "response",
            "response_error",
            "answers",
            "contexts",
            "planning_retry",
            "corrective_recovery",
            "recovered_result",
            "independent_review",
            "previous_reviews",
            "batch_review_resume",
            "review_coordination",
            "legacy_report_submissions",
            "restarts",
            "integration_retries",
            "requalifications",
        ],
    ),
];
pub(crate) const NESTED: &[(&str, &str, &[&str])] = &[
    ("object", "launch_profile", &["updated", "actor"]),
    ("task", "launch_profile", &["updated", "actor"]),
    ("planning", "repository", &["candidate_commit"]),
    (
        "planning",
        "judge",
        &[
            "verdict",
            "reason",
            "evidence_ref",
            "at",
            "cycle",
            "closed",
            "history",
        ],
    ),
];
