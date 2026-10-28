//! Protected calculated Work resume; no view publication or visit mutation.
use super::{close, root, HttpServiceError as E, NativeHttpService, RouteBody};
use http::{Method, Request};
use swarm_engine::{RootStore, WorkViewLanguage};

/// Strict explicit Work/language pair. Reject unknown/duplicate/empty values
/// before store IO; identifiers are data, never another Root or path choice.
fn query(raw: Option<&str>) -> Result<(String, String), E> {
    let (mut work, mut language) = (None, None);
    for (key, value) in form_urlencoded::parse(raw.unwrap_or("").as_bytes()) {
        let slot = match key.as_ref() {
            "work" => &mut work,
            "lang" => &mut language,
            _ => return Err(E::Refused("work-resume-query-invalid")),
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(E::Refused("work-resume-query-invalid"));
        }
    }
    match (work, language) {
        (Some(work), Some(language)) if swarm_identity_contract::name(work.as_bytes())
            && matches!(language.as_str(), "fr" | "en") => Ok((work, language)),
        _ => Err(E::Refused("work-resume-query-invalid")),
    }
}

/// Native observation DTO; markdown is generated text, never trusted HTML or
/// proof of agent identity, readiness, fresh acceptance or comparison of visits.
#[derive(serde::Serialize)]
struct Resume {
    observation: &'static str,
    schema_version: i64,
    work_id: String,
    revision: i64,
    language: String,
    markdown: String,
}

/// Fixed read capability selected by route, never an execution/operator policy.
#[derive(Clone, Copy)]
pub(super) enum Operation { Calculated, Recorded }

/// Plain recorded-data summary only. Does not invoke the validation projection,
/// infer acceptance or bypass its refusal; the caller explicitly selected this
/// separate read. User strings stay text. No IO or mutation occurs here.
fn recorded(work: &swarm_engine::Work, language: &str) -> String {
    let (notice, objective, tasks, next) = if language == "fr" {
        ("Données enregistrées uniquement — validation non recalculée.", "Objectif", "Tâches et états enregistrés", "Suite enregistrée")
    } else {
        ("Recorded data only — validation has not been recalculated.", "Objective", "Tasks and recorded states", "Recorded next step")
    };
    let mut text = format!("# {}\n\n{}\n\n## {}\n{}\n\n## {}\n", work.title, notice, objective, work.objective, tasks);
    for task in work.tasks.iter().flatten() {
        text.push_str(&format!("- {} · {} · {}\n", task.id, task.title, task.status));
    }
    text.push_str(&format!("\n## {}\n{}\n", next, work.next));
    text
}

impl NativeHttpService {
    /// Read one explicit Work summary using inspect_work_view under shared HTTP
    /// admission/capacity and the owned Root binding. Reads actual saved policy,
    /// events, files/Git observations; no provider or check command runs, no view
    /// file is published (unlike render_work_view). GET/HEAD and empty body only.
    /// No visit/OODA/budget/Work revision is changed. Typed read/encode/close
    /// failures propagate, never an empty successful placeholder. The separate
    /// Recorded route reads saved fields only on explicit selection, never
    /// converts failed calculated validation to success. Read reservations
    /// are temporary coordination, not a future authority or atomic FS snapshot.
    pub(super) fn work_resume(&self, operation: Operation, request: &Request<Vec<u8>>) -> Result<RouteBody, E> {
        if !matches!(*request.method(), Method::GET | Method::HEAD) {
            return Err(E::Method("GET, HEAD"));
        }
        if !request.body().is_empty() {
            return Err(E::Refused("work-resume-read-body-invalid"));
        }
        let (work, language) = query(request.uri().query())?;
        let store = RootStore::open(&self.root, false).map_err(|e| root("open", e))?;
        let result = (|| {
            self.check_binding(&store)?;
            let (work, markdown, observation) = match operation {
                Operation::Calculated => {
                    let selected = if language == "fr" { WorkViewLanguage::French } else { WorkViewLanguage::English };
                    let view = store.inspect_work_view(&work, selected).map_err(|e| root("resume-read", e))?;
                    (view.work, view.resume_markdown, "calculated")
                }
                Operation::Recorded => {
                    let work = store.read_work(&work).map_err(|e| root("resume-recorded-read", e))?;
                    let markdown = recorded(&work, &language);
                    (work, markdown, "recorded")
                }
            };
            swarm_engine::encode_model(&Resume {
                observation,
                schema_version: 1,
                work_id: work.id,
                revision: work.revision,
                language,
                markdown,
            }).map(RouteBody::plain).map_err(E::Model)
        })();
        close(result, store.close())
    }
}
