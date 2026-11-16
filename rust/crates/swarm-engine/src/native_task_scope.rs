//! Native task binding data. Serialization never authenticates an execution.
use serde::{Deserialize, Serialize};

/// Exact physical Root, Work, task, attempt and agent binding supplied by the
/// trusted host. An operator may declare this mapping, but a consumer must
/// independently observe the current owned execution before granting authority.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTaskScope {
    pub root_digest: String,
    pub work_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub agent_id: String,
}

impl NativeTaskScope {
    /// Validate representation only against the shared identity contract.
    /// Returns a content-free static cause; performs no IO, authentication,
    /// policy lookup, task state check or mutation.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !swarm_identity_contract::digest(self.root_digest.as_bytes())
            || [
                &self.work_id,
                &self.task_id,
                &self.attempt_id,
                &self.agent_id,
            ]
            .iter()
            .any(|s| !swarm_identity_contract::name(s.as_bytes()))
        {
            return Err("native-task-scope-invalid");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercise valid closed-scope JSON and its round trip, then reject empty
    /// coordinates, an unknown field and a missing attempt. Assertions panic on
    /// mismatch; representation checks alone authenticate no execution or role.
    #[test]
    fn native_scope_closed_shape_and_identity_contract() {
        let good = serde_json::json!({"root_digest":"a".repeat(64),"work_id":"work",
            "task_id":"task","attempt_id":"attempt","agent_id":"agent"});
        let scope: NativeTaskScope = serde_json::from_value(good.clone()).unwrap();
        scope.validate().unwrap();
        assert!(serde_json::to_value(&scope).unwrap() == good);
        for key in [
            "root_digest",
            "work_id",
            "task_id",
            "attempt_id",
            "agent_id",
        ] {
            let mut bad = good.clone();
            bad[key] = "".into();
            assert!(serde_json::from_value::<NativeTaskScope>(bad)
                .unwrap()
                .validate()
                .is_err());
        }
        let mut bad = good.clone();
        bad["integrity"] = 3.into();
        assert!(serde_json::from_value::<NativeTaskScope>(bad).is_err());
        let mut bad = good;
        bad.as_object_mut().unwrap().remove("attempt_id");
        assert!(serde_json::from_value::<NativeTaskScope>(bad).is_err());
    }
}
