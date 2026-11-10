//! Explicit adoption of a retained proposal through the existing CAS writer.
use super::{
    finished,
    preparation_read::{body, cause},
    RootStoreError as E,
};
use crate::{EngineSettingsValues, Preparation, PreparationCommand, PreparationTurn};
use rusqlite::{params, Connection};

/// Validate source-turn identity/status/freshness and prepare a save intention.
/// Answers supplied by the model are cleared; only identical existing operator
/// questions may retain their answer. Saved documents keep source_turn, and
/// receipt identity remains the original use-proposal request at the caller.
/// No provider, plan verdict or mission authority is created by this operation.
pub(super) fn edit(
    db: &Connection,
    p: &Preparation,
    r: &PreparationCommand,
    method: &str,
    settings: &EngineSettingsValues,
) -> Result<PreparationCommand, E> {
    let mut s = db
        .prepare("SELECT status,body FROM preparation_turns WHERE id=?1 AND preparation_id=?2")
        .map_err(E::Sql)?;
    let result = (|| {
        let mut rows = s.query(params![r.turn_id, p.id]).map_err(E::Sql)?;
        let row = rows
            .next()
            .map_err(E::Sql)?
            .ok_or(E::Refused("preparation-turn-not-found"))?;
        let status: String = row.get(0).map_err(E::Sql)?;
        let t: PreparationTurn = crate::work::decode(
            &body(row, 1)?,
            crate::preparation_turn::shape_preparation_turn(),
        )
        .map_err(|e| cause("preparation-dialogue-document-invalid", e))?;
        if t.id != r.turn_id || t.preparation_id != p.id {
            return Err(E::Refused("preparation-dialogue-row-identity-mismatch"));
        }
        let a = t
            .answer
            .as_ref()
            .ok_or(E::Refused("preparation-stale-document"))?;
        if status != "answered" || t.revision != p.revision || t.method_hash != method {
            return Err(E::Refused("preparation-stale-document"));
        }
        let mut edit = r.clone();
        edit.action = "save".into();
        edit.adopt_brief = false;
        edit.turn_id.clear();
        if t.target == "plan" {
            if r.adopt_brief {
                return Err(E::Refused("preparation-request-invalid"));
            }
            let brief = p
                .adopted_brief
                .as_ref()
                .ok_or(E::Refused("preparation-stale-brief"))?;
            if p.documents
                .as_ref()
                .and_then(|d| d.get("brief"))
                .is_none_or(|d| d.sha256 != brief.sha256)
                || p.documents
                    .as_ref()
                    .and_then(|d| d.get("besoin"))
                    .map_or("", |d| d.sha256.as_str())
                    != brief.need_hash
            {
                return Err(E::Refused("preparation-stale-brief"));
            }
            let limits = settings.preparation_plan_policy().map_err(E::Settings)?;
            let task = settings.mutation_limits().map_err(E::Settings)?.task;
            let mut plan = super::preparation_plan::validate_draft(
                a.plan.as_bytes(),
                &p.method,
                &limits,
                &task,
            )?;
            // Parse an existing nonempty plan explicitly: corruption is not an
            // excuse to lose prior decisions silently. An absent/empty plan has
            // no prior answers; saving a repaired plan remains a separate action.
            let prior = p
                .documents
                .as_ref()
                .and_then(|d| d.get("plan"))
                .filter(|d| !d.text.is_empty())
                .map(|d| {
                    super::preparation_plan::validate_draft(
                        d.text.as_bytes(),
                        &p.method,
                        &limits,
                        &task,
                    )
                })
                .transpose()?;
            for question in plan.questions.iter_mut().flatten() {
                question.answer = prior
                    .as_ref()
                    .and_then(|p| p.questions.as_ref())
                    .and_then(|qs| {
                        qs.iter().rev().find(|q| {
                            q.question == question.question && crate::text::nonempty(&q.answer)
                        })
                    })
                    .map(|q| q.answer.clone())
                    .unwrap_or_default();
            }
            edit.document = "plan".into();
            edit.text = String::from_utf8(
                crate::encode_model(&plan).map_err(|e| cause("preparation-plan-encode", e))?,
            )
            .map_err(|e| cause("preparation-plan-utf8", e))?;
        } else {
            if crate::text::trim_space(&a.brief).is_empty() {
                return Err(E::Refused("preparation-stale-document"));
            }
            edit.document = "brief".into();
            edit.text = a.brief.clone();
        }
        let cap = settings
            .preparation_write_policy()
            .map_err(E::Settings)?
            .document_max_bytes;
        if cap.is_some_and(|n| edit.text.len() > n) {
            return Err(E::Refused("preparation-document-over-limit"));
        }
        Ok(edit)
    })();
    finished(result, s.finalize().map_err(E::Sql))
}
