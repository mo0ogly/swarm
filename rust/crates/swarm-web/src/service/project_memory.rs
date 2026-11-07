//! Protected versioned-memory catalog read and mutation surface (Go
//! `/api/v1/project-memory`). GET/HEAD serve the read-only catalog; POST with
//! `action=…` serves the memory previews and commits through the same
//! RootStore paths as the CLI (preview → `preview_project_memory`, commit →
//! `execute_external("memory.<action>")`). The M20 `summary-source` action and
//! any unknown action refuse with the typed unported code instead of inventing
//! their wire behavior here.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, RootStoreError};

/// The POST actions served by this route. Commit names map to engine
/// `memory.<action>` kinds; preview names strip the `-preview` suffix and read
/// the proposed outcome instead of mutating.
const COMMIT_ACTIONS: &[&str] = &["admit", "replace", "withdraw", "select"];
const PREVIEW_ACTIONS: &[&str] = &[
    "admit-preview",
    "replace-preview",
    "withdraw-preview",
    "select-preview",
];

/// Accept only one `q` member (possibly empty) for the catalog read, refuse
/// unknown or repeated members. Mirrors Go's `r.URL.Query().Get("q")`
/// observable contract for the supported shapes while staying strict on
/// ambiguity.
fn catalog_query(raw: Option<&str>) -> Result<String, E> {
    let mut query = None;
    for (k, v) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        if k != "q" || query.replace(v.into_owned()).is_some() {
            return Err(E::Refused("project-memory-query-invalid"));
        }
    }
    Ok(query.unwrap_or_default())
}

/// Select the POST action from the `action` query member (single, nonempty,
/// known); anything else refuses with the typed unported code — the M20
/// summary-source packet stays a later increment and unknown names never fall
/// back to a default mutation.
fn post_action(raw: Option<&str>) -> Result<&'static str, E> {
    let mut action: Option<String> = None;
    for (k, v) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        if k != "action" || v.is_empty() || action.replace(v.into_owned()).is_some() {
            return Err(E::Refused("project-memory-query-invalid"));
        }
    }
    let action = action.as_deref().ok_or(E::Refused("project-memory-query-invalid"))?;
    if action == "summary-source" {
        return Err(E::Refused("project-memory-operation-unported"));
    }
    COMMIT_ACTIONS
        .iter()
        .chain(PREVIEW_ACTIONS.iter())
        .find(|candidate| **candidate == action)
        .copied()
        .ok_or(E::Refused("project-memory-operation-unported"))
}

impl NativeHttpService {
    /// GET/HEAD catalog read with an optional folded literal `q` filter; empty
    /// body, readonly Root, shared scoped session and datastore binding precede
    /// the guarded catalog read. POST carries the `action` query member: a
    /// preview action reads the proposed outcome under a readonly Root (no
    /// commit), a commit action opens the writable bound Root and runs the
    /// same engine transaction/replay/preview-token checks as the CLI,
    /// reporting a recorded Work state on a later close/encode failure. Go's
    /// response shapes are preserved by the engine types. A read/encode/close
    /// failure refuses — never an empty success. `summary-source` and unknown
    /// actions refuse as unported: the M20 packet is a later G2 increment.
    pub(super) fn project_memory(&self, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if matches!(*request.method(), Method::GET | Method::HEAD) {
            if !request.body().is_empty() {
                return Err(E::Refused("project-memory-query-invalid"));
            }
            let query = catalog_query(request.uri().query())?;
            let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
            let result = (|| {
                self.check_binding(&store)?;
                let catalog =
                    store.memory_catalog(&query).map_err(|e| root("project-memory-read", e))?;
                serde_json::to_vec(&catalog)
                    .map(RouteBody::plain)
                    .map_err(E::ResponseJson)
            })();
            return close(result, store.close());
        }
        if request.method() != Method::POST {
            return Err(E::Method("GET, HEAD, POST"));
        }
        let action = post_action(request.uri().query())?;
        swarm_engine::check_command_input_size(request.body()).map_err(E::Input)?;
        // Decode before opening the Root so malformed memory documents refuse
        // with their stable shape cause at the transport phase. The decode
        // action is the bare action name (Go strips `-preview` before any
        // per-action shape check), never the preview-suffixed query value.
        let bare_action = action.strip_suffix("-preview").unwrap_or(action);
        swarm_engine::decode_project_memory_request(request.body(), bare_action).map_err(|e| {
            root(
                "project-memory-input",
                RootStoreError::Mutation(swarm_engine::WorkEngineError::Memory(e)),
            )
        })?;
        let readonly = PREVIEW_ACTIONS.contains(&action);
        let mut store = RootStore::open(&self.root, !readonly).map_err(|e| root("open", e))?;
        let mut recorded = None;
        let result = (|| {
            self.check_binding(&store)?;
            let raw = request.body();
            if readonly {
                let preview_action = action.strip_suffix("-preview").unwrap_or(action);
                let preview = store
                    .preview_project_memory(&raw_action_work(raw)?, raw, preview_action)
                    .map_err(|e| root("project-memory-preview", e))?;
                serde_json::to_vec(&preview)
                    .map(RouteBody::plain)
                    .map_err(E::ResponseJson)
            } else {
                let work_id = raw_action_work(raw)?;
                let outcome = store
                    .execute_external(
                        &work_id,
                        &format!("memory.{action}"),
                        raw,
                        &mut swarm_engine::SystemEnvironment,
                    )
                    .map_err(|e| root("project-memory-mutate", e))?;
                let work = outcome.result.work;
                recorded = Some((work.id.clone(), work.revision));
                serde_json::to_vec(&work)
                    .map(RouteBody::plain)
                    .map_err(E::ResponseJson)
            }
        })();
        match close(result, store.close()) {
            Ok(body) => Ok(body),
            Err(primary) => match recorded {
                Some((work, revision)) => Err(E::Recorded {
                    primary: Box::new(primary),
                    work,
                    revision,
                }),
                None => Err(primary),
            },
        }
    }
}

/// The request document carries the Work id in its `work` member; decode
/// already succeeded, so a re-read is a plain JSON lookup. A body whose Work
/// member is unusable refuses before any store call.
fn raw_action_work(raw: &[u8]) -> Result<String, E> {
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| E::Refused("project-memory-query-invalid"))?;
    value["work"]
        .as_str()
        .filter(|work| !work.is_empty())
        .map(str::to_owned)
        .ok_or(E::Refused("project-memory-query-invalid"))
}
