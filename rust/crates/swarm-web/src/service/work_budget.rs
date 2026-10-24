//! Protected existing mission-budget edit contract; no execution permission.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use serde_json::json;
use swarm_engine::{RootStore, SystemEnvironment};

/// Fixed route choice; client fields cannot select another mutation kind.
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Show,
    Preview,
    Apply,
}

/// Require exactly one safe Work name; duplicate/unknown/empty URL keys refuse.
/// URL decoding supplies data only. No path, default Work or identity is inferred.
fn work_query(query: Option<&str>) -> Result<String, E> {
    let mut work = None;
    for (key, value) in form_urlencoded::parse(query.unwrap_or("").as_bytes()) {
        if key != "work" || work.replace(value.into_owned()).is_some() {
            return Err(E::Refused("budget-query-invalid"));
        }
    }
    work.filter(|w| swarm_identity_contract::name(w.as_bytes()))
        .ok_or(E::Refused("budget-query-invalid"))
}

/// Map a budget decoding/validation failure to the root store error shape.
fn budget_input(cause: swarm_engine::BudgetError) -> E {
    root(
        "budget-input",
        swarm_engine::RootStoreError::Mutation(swarm_engine::WorkEngineError::Budget(cause)),
    )
}

impl NativeHttpService {
    /// Run behind shared session/Host/CSRF and configured Root capacity. Exact
    /// method/query/body admission precedes opening the bound Root. GET/HEAD
    /// show the current budget; POST with a decoded change previews or applies.
    /// Preview validates only amounts and revision currency: it grants no launch
    /// permission and reserves nothing. Apply uses saved policy/revision/replay
    /// and preserves known Work through encoding/close failure. No provider,
    /// agent, lease, visit, refund, reset, default, source price lookup, or
    /// authenticated human identity is introduced by this adapter. Missing policy
    /// remains explicit null; show/preview reads are not a cross-producer snapshot.
    /// Preview is informational financial validation only, not event/replay admission;
    /// execute_external is the authoritative apply validation and transaction.
    pub(super) fn work_budget_route(
        &self,
        operation: Operation,
        request: &Request<Vec<u8>>,
    ) -> Result<RouteBody, E> {
        let post = !matches!(operation, Operation::Show);
        if post && request.method() != Method::POST
            || !post && request.method() != Method::GET && request.method() != Method::HEAD
        {
            return Err(E::Method(if post { "POST" } else { "GET, HEAD" }));
        }
        let work = work_query(request.uri().query())?;
        if post {
            swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        } else if !request.body().is_empty() {
            return Err(E::Refused("budget-read-body-invalid"));
        }
        let mut store = RootStore::open(&self.root, matches!(operation, Operation::Apply))
            .map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let bytes = match operation {
                Operation::Show => {
                    let current = store.read_work(&work).map_err(|e| root("budget-read", e))?;
                    let budget = store
                        .work_budget(&work)
                        .map_err(|e| root("budget-read", e))?;
                    let state = store
                        .work_budget_state(&work)
                        .map_err(|e| root("budget-read", e))?;
                    serde_json::to_vec(&json!({"schema_version":1,"work_id":work,
                        "revision":current.revision,"budget":budget,"state":state}))
                        .map_err(E::Json)?
                }
                Operation::Preview => {
                    let mut change = swarm_engine::decode_budget_change(request.body())
                        .map_err(budget_input)?;
                    swarm_engine::validate_budget_change(&change).map_err(budget_input)?;
                    change.budget.actor.clear();change.budget.updated.clear();
                    let current = store.read_work(&work).map_err(|e| root("budget-read", e))?;
                    if change.expected_revision != current.revision {
                        return Err(E::Refused("budget-revision-conflict"));
                    }
                    let budget = store
                        .work_budget(&work)
                        .map_err(|e| root("budget-read", e))?;
                    let state = store
                        .work_budget_state(&work)
                        .map_err(|e| root("budget-read", e))?;
                    serde_json::to_vec(&json!({
                        "schema_version": 1,
                        "work_id": work,
                        "revision": current.revision,
                        "current": budget,
                        "state": state,
                        "proposed": change.budget,
                    }))
                    .map_err(E::Json)?
                }
                Operation::Apply => {
                    let outcome = store
                        .execute_external(
                            &work,
                            "budget.configure",
                            request.body(),
                            &mut SystemEnvironment,
                        )
                        .map_err(|e| root("budget-apply", e))?;
                    let current = outcome.result.work;
                    recorded = Some((current.id.clone(), current.revision));
                    swarm_engine::encode_model(&current).map_err(E::Model)?
                }
            };
            Ok(RouteBody {
                observation_causes: Vec::new(),
                bytes,
                recorded_work: recorded.clone(),
                configuration_recorded: false,
                recorded_configuration: None,
            })
        })();
        match (close(result, store.close()), recorded) {
            (Err(primary), Some((work, revision))) => Err(E::Recorded {
                primary: Box::new(primary),
                work,
                revision,
            }),
            (result, _) => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Valid URL witness precedes ambiguous and traversal-shaped refusals.
    #[test]
    fn budget_work_query_refuses_ambiguity_without_default() {
        assert_eq!(work_query(Some("work=w-one")).unwrap(), "w-one");
        assert_eq!(work_query(Some("work=w%2Done")).unwrap(), "w-one");
        for query in [
            None,
            Some(""),
            Some("work="),
            Some("work=w&work=x"),
            Some("work=w&actor=engine"),
            Some("work=%2E%2E%2Fx"),
        ] {
            assert_eq!(work_query(query).unwrap_err().code(), "budget-query-invalid");
        }
    }
}