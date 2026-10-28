//! Current worker identity/contract fragment, rebuilt from candidate engine records.
//! Text is not cryptographic identity, a provider receipt or execution authority.
use crate::{AgentLaunchError, Work};
use sha2::{Digest, Sha256};
/// Encode declared own-contract data for a prompt without IO or mutation. Codec
/// and UTF8 causes remain typed; returned JSON proves no authenticity or grant.
fn encoded_contract<T: serde::Serialize>(contract: &T) -> Result<String, AgentLaunchError> {
    let bytes = crate::encode_model(contract)
        .map_err(|e| AgentLaunchError::caused("agent-current-contract-encoding", e))?;
    String::from_utf8(bytes).map_err(|e| AgentLaunchError::caused("agent-current-contract-utf8", e))
}

/// Compose current identifiers and latest own contract from the actual indexed
/// task in the supplied post-transition Work candidate; an out-of-range index
/// refuses rather than accepting a separately supplied, inconsistent Task.
/// Manual and hierarchical fragments create no rights, IO or mutation. Hierarchy
/// uses the existing full state/ownership validator; its original cause survives.
/// Child scopes receive only their objective/assigned requirements/own contract,
/// without root brief or whole-plan prose. Root receives adopted common context.
/// Native planning.task contracts come from selected Task fields when no manual
/// ApprovedPlan exists; explicit conflicting manual mappings still refuse.
/// Corrective context remains unported. Invalid identity/attempt/contract/encoding
/// refuses without returning partial text; outer composer must apply saved caps.
/// This fragment is not an authenticated identity or a complete scoped prompt.
/// Scoped operator-worker launch composes a scope-only header with this fragment.
/// Shared operator profile text is explicit input; its free prose is not redacted.
/// Parent delegation and automatic scope scheduling remain separate contracts.
pub(crate) fn current(work: &Work, index: usize, agent: &str) -> Result<String, AgentLaunchError> {
    let task = work
        .tasks
        .as_deref()
        .and_then(|tasks| tasks.get(index))
        .ok_or(AgentLaunchError::Refused("agent-current-task-missing"))?;
    if task.corrective_recovery.is_some() {
        return Err(AgentLaunchError::Refused(
            "agent-current-context-branch-unported",
        ));
    }
    let scope = if work.planning.is_some() {
        let chain = crate::planning_scope_ancestors(work, &task.scope_id)
            .map_err(|e| AgentLaunchError::caused("agent-current-scope-invalid", e))?;
        Some(
            *chain
                .first()
                .ok_or(AgentLaunchError::Refused("agent-current-scope-missing"))?,
        )
    } else {
        if !task.scope_id.is_empty() {
            return Err(AgentLaunchError::Refused(
                "agent-current-context-branch-unported",
            ));
        }
        None
    };
    let root_context = scope.is_none_or(|scope| scope.parent.is_empty());
    let attempts = task.attempts.as_deref().unwrap_or_default();
    let attempt = attempts
        .last()
        .ok_or(AgentLaunchError::Refused("agent-current-attempt-missing"))?;
    if work.revision < 1
        || task.plan_max_attempts < 0
        || !matches!(task.plan_role.as_str(), "" | "worker")
        || [&work.id, &task.id, agent, &attempt.id]
            .iter()
            .any(|id| !swarm_identity_contract::name(id.as_bytes()))
    {
        return Err(AgentLaunchError::Refused(
            "agent-current-context-inconsistent",
        ));
    }
    let count = if task.plan_max_attempts > 0 {
        format!("{}/{}", attempts.len(), task.plan_max_attempts)
    } else {
        attempts.len().to_string()
    };
    let mut text=format!("\nIDENTITÉ COURANTE FOURNIE PAR LE MOTEUR : mission {} ; tâche {} ; agent {} ; tentative {} ; départ {} ; révision du travail {}.\n",work.id,task.id,agent,attempt.id,count,work.revision);
    text.push_str("Les identités, limites et conclusions des rapports antérieurs sont historiques. Elles ne remplacent pas cette identité ni le contrat courant ci-dessous. Le contrat courant prévaut sur les consignes historiques recopiées dans un profil. Un ancien rapport ne prouve pas la réussite de cette tentative. Les identifiants affichés ne prouvent ni authentification cryptographique ni droits supplémentaires.\n");
    let mut found_contract = false;
    for plan in work.plans.as_deref().unwrap_or_default().iter().rev() {
        // Ten hex digits are the existing Go plan-ID wire convention, not policy
        // or proof of authenticity. Stored task mappings must agree when present.
        let prefix = format!(
            "plan-{}-",
            &format!("{:x}", Sha256::digest(plan.source.as_bytes()))[..10]
        );
        let map_claims = plan
            .task_map
            .as_ref()
            .map_or(0, |m| m.values().filter(|id| *id == &task.id).count());
        let id_claims = plan
            .task_ids
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|id| *id == &task.id)
            .count();
        if map_claims > 1 || id_claims > 1 {
            return Err(AgentLaunchError::Refused(
                "agent-current-plan-mapping-invalid",
            ));
        }
        let mut own = plan
            .spec
            .tasks
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|candidate| format!("{prefix}{}", candidate.id) == task.id);
        let Some(contract) = own.next() else {
            if map_claims != 0 || id_claims != 0 {
                return Err(AgentLaunchError::Refused(
                    "agent-current-plan-mapping-invalid",
                ));
            }
            continue;
        };
        if own.next().is_some()
            || contract.role != "worker"
            || plan
                .task_ids
                .as_ref()
                .filter(|ids| !ids.is_empty())
                .is_some_and(|ids| !ids.contains(&task.id))
            || plan
                .task_map
                .as_ref()
                .filter(|m| !m.is_empty())
                .is_some_and(|m| m.get(&contract.id) != Some(&task.id))
        {
            return Err(AgentLaunchError::Refused(
                "agent-current-plan-mapping-invalid",
            ));
        }
        let json = encoded_contract(contract)?;
        text.push_str(&format!(
            "CONTRAT COURANT DE LA TÂCHE (brief {}, plan {}) :\n{}\n",
            plan.brief_hash, plan.response_hash, json
        ));
        if root_context {
            if !plan.spec.objective.is_empty() {
                text.push_str(&format!(
                    "Objectif du plan courant : {}\n",
                    plan.spec.objective
                ));
            }
            for assumption in plan.spec.assumptions.as_deref().unwrap_or_default() {
                text.push_str(&format!(
                    "Hypothèse du plan (pas un fait prouvé) : {assumption}\n"
                ));
            }
            for question in plan.spec.questions.as_deref().unwrap_or_default() {
                text.push_str(&format!(
                    "Décision du plan : {} → {}\n",
                    question.question, question.answer
                ));
            }
        }
        text.push_str("Les limites effectives figées et le compteur courant de tentatives restent ceux du moteur, même si le plan historique indique d'autres valeurs. Ce contexte ne vaut ni résultat de contrôle ni acceptation.\n");
        found_contract = true;
        break;
    }
    if !found_contract {
        if scope.is_some() {
            // planning.task is already a declared scoped contract, independently
            // of the optional manual ApprovedPlan archive. Do not copy Task.Next,
            // historical reports, peer tasks or global planning delivery bodies.
            let contract = serde_json::json!({"id":&task.id,"scope_id":&task.scope_id,
                "role":"worker","title":&task.title,"deliverable":&task.deliverable,
                "criteria":&task.criteria,"requirements":&task.requirements,
                "depends":&task.depends,"max_attempts":task.plan_max_attempts,
                "max_tool_calls":task.plan_tool_limit});
            text.push_str(&format!("CONTRAT COURANT AFFECTÉ PAR LE SCOPE (déclaration, pas preuve de réussite) :\n{}\n",encoded_contract(&contract)?));
            text.push_str("Les limites effectives figées restent celles du moteur ; ce contrat ne crée ni résultat de contrôle ni acceptation.\n");
        } else if task.plan_role == "worker" {
            return Err(AgentLaunchError::Refused(
                "agent-current-plan-mapping-invalid",
            ));
        }
    }
    if let Some(scope) = scope {
        text.push_str(&format!("Périmètre courant déclaré : {} ; parent déclaré : {}.\nObjectif du périmètre courant : {}\n",scope.id,scope.parent,scope.objective));
        // Index canonical requirement keys once; preserve the task's assignment
        // order without repeatedly scanning all criteria or introducing a cap.
        let requirements = work
            .criteria
            .iter()
            .flatten()
            .enumerate()
            .map(|(i, text)| (format!("req-{}", i + 1), text))
            .collect::<std::collections::BTreeMap<_, _>>();
        for required in task.requirements.iter().flatten() {
            let criterion = requirements.get(required).ok_or(AgentLaunchError::Refused(
                "agent-current-requirement-invalid",
            ))?;

            text.push_str(&format!("Exigence assignée {required} : {criterion}\n"));
        }
        if root_context {
            let brief = work
                .planning_brief
                .as_deref()
                .map(|brief| brief.text.as_str())
                .unwrap_or(&work.scope);
            text.push_str(&format!(
                "BRIEF COMMUN ADOPTÉ (contexte, pas preuve de réussite) :\n{brief}\n"
            ));
        }
    }
    if attempts.len() > 1 {
        text.push_str("REPRISE : comparer le contrat courant au rapport existant, conserver les preuves encore pertinentes et traiter les lacunes. Produire un rapport actualisé attribuable à cette tentative, même si aucun fichier applicatif ne change. Si une lacune ou un blocage persiste, le signaler explicitement ; ne pas déclarer terminé sur la seule présence d'un rapport ancien.\n");
    }
    Ok(text)
}

#[cfg(test)]
mod scoped_tests {
    use super::*;
    /// Two isolated branches and private root/sibling prose. No tools/providers,
    /// consent or extra rights are encoded by this supplied model fixture.
    fn fixture() -> Work {
        let mut w=crate::decode_work(br#"{"schema_version":1,"id":"work","revision":7,"scope":"ROOT-PRIVATE-BRIEF","criteria":["OWN-REQUIREMENT","SIBLING-PRIVATE-REQUIREMENT"],"tasks":[{"id":"task","scope_id":"child","requirements":["req-1"],"attempts":[{"id":"old-attempt"},{"id":"current-attempt"}]}],"planning":{"version":1,"max_tasks":10,"max_decisions":10,"max_activations":20,"scopes":[{"id":"root","revision":1},{"id":"child","parent":"root","revision":1,"objective":"OWN-SCOPE","requirements":["req-1"]},{"id":"other","parent":"root","revision":1,"requirements":["req-2"]}]}}"#).unwrap();
        let source = "own-source";
        let id = format!(
            "plan-{}-T1",
            &format!("{:x}", Sha256::digest(source.as_bytes()))[..10]
        );
        let task = &mut w.tasks.as_mut().unwrap()[0];
        task.id = id.clone();
        task.plan_role = "worker".into();
        task.plan_max_attempts = 3;
        let own = crate::PlanMission {
            id: "T1".into(),
            role: "worker".into(),
            scope: "OWN-CONTRACT".into(),
            ..Default::default()
        };
        let sibling = crate::PlanMission {
            id: "T2".into(),
            role: "worker".into(),
            scope: "SIBLING-PRIVATE-CONTRACT".into(),
            ..Default::default()
        };
        let plan = crate::ApprovedPlan {
            source: source.into(),
            brief_hash: "current-brief-hash".into(),
            spec: crate::ActionPlan {
                version: 1,
                objective: "ROOT-PRIVATE-PLAN-OBJECTIVE".into(),
                assumptions: Some(vec!["ROOT-PRIVATE-ASSUMPTION".into()]),
                questions: Some(vec![crate::PlanQuestion {
                    question: "ROOT-PRIVATE-QUESTION".into(),
                    answer: "ROOT-PRIVATE-ANSWER".into(),
                }]),
                tasks: Some(vec![own, sibling]),
                ..Default::default()
            },
            ..Default::default()
        };
        w.plans = Some(vec![plan]);
        w
    }
    /// Actual owned contract/current identity is present; root brief, whole-plan
    /// prose and sibling requirement/contract never leak to a delegated worker.
    #[test]
    fn delegated_worker_identity_and_contract_exclude_root_and_sibling_context() {
        let w = fixture();
        crate::validate_planning_state(&w).unwrap();
        let text = current(&w, 0, "current-agent").unwrap();
        for expected in [
            "current-agent",
            "current-attempt",
            "2/3",
            "OWN-SCOPE",
            "OWN-CONTRACT",
            "OWN-REQUIREMENT",
            "révision du travail 7",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
        for private in ["ROOT-PRIVATE", "SIBLING-PRIVATE", "old-attempt"] {
            assert!(!text.contains(private), "leaked {private}");
        }
        let mut reordered = w.clone();
        reordered.criteria.as_mut().unwrap()[1] = "OWN-SECOND-REQUIREMENT".into();
        reordered.tasks.as_mut().unwrap()[0].requirements =
            Some(vec!["req-2".into(), "req-1".into()]);
        let scopes = reordered
            .planning
            .as_mut()
            .unwrap()
            .scopes
            .as_mut()
            .unwrap();
        scopes[2].requirements = None;
        scopes[1].requirements = Some(vec!["req-1".into(), "req-2".into()]);
        let selected = current(&reordered, 0, "current-agent").unwrap();
        assert!(
            selected.find("Exigence assignée req-2").unwrap()
                < selected.find("Exigence assignée req-1").unwrap()
        );
    }
    /// Root receives shared adopted context, but still only assigned requirement
    /// and own task contract; this is descriptive metadata, never a permit.
    #[test]
    fn root_worker_shared_brief_keeps_own_requirement_and_contract() {
        let mut w = fixture();
        w.tasks.as_mut().unwrap()[0].scope_id = "root".into();
        let scopes = w.planning.as_mut().unwrap().scopes.as_mut().unwrap();
        scopes[0].requirements = scopes[1].requirements.take();
        crate::validate_planning_state(&w).unwrap();
        let text = current(&w, 0, "current-agent").unwrap();
        assert!(text.contains("ROOT-PRIVATE-BRIEF"));
        assert!(text.contains("ROOT-PRIVATE-QUESTION"));
        assert!(text.contains("OWN-REQUIREMENT"));
        assert!(!text.contains("SIBLING-PRIVATE"));
        w.planning_brief = Some(Box::new(crate::PlanningBrief {
            text: "ADOPTED-NEW-BRIEF".into(),
            ..Default::default()
        }));
        let adopted = current(&w, 0, "current-agent").unwrap();
        assert!(adopted.contains("ADOPTED-NEW-BRIEF"));
        assert!(!adopted.contains("ROOT-PRIVATE-BRIEF"));
    }
    /// Validate selected ownership before text construction; invalid index,
    /// unknown scope and malformed current identity refuse, then valid input
    /// recovers. Planning cause stays typed; no partial text/IO is returned.
    #[test]
    fn scoped_worker_invalid_index_ownership_identity_refuse_and_recover() {
        use std::error::Error;
        let valid = fixture();
        current(&valid, 0, "current-agent").unwrap();
        assert_eq!(
            current(&valid, 1, "current-agent").unwrap_err().code(),
            "agent-current-task-missing"
        );
        let mut missing = valid.clone();
        missing.tasks.as_mut().unwrap()[0].scope_id = "unknown".into();
        let error = current(&missing, 0, "current-agent").unwrap_err();
        assert_eq!(error.code(), "agent-current-scope-invalid");
        assert!(error
            .source()
            .unwrap()
            .downcast_ref::<crate::PlanningBudgetError>()
            .is_some());
        let mut foreign = valid.clone();
        foreign.tasks.as_mut().unwrap()[0].requirements = Some(vec!["req-2".into()]);
        assert_eq!(
            current(&foreign, 0, "current-agent").unwrap_err().code(),
            "agent-current-scope-invalid"
        );
        let mut attempt = valid.clone();
        attempt.tasks.as_mut().unwrap()[0].attempts = None;
        assert_eq!(
            current(&attempt, 0, "current-agent").unwrap_err().code(),
            "agent-current-attempt-missing"
        );
        assert_eq!(
            current(&valid, 0, "invalid agent").unwrap_err().code(),
            "agent-current-context-inconsistent"
        );
        current(&valid, 0, "current-agent").unwrap();
    }
    /// Actual Go planning.task creates scoped workers without an ApprovedPlan.
    /// Their current assigned Task contract must not inherit a manual-plan
    /// requirement or copy historical status/report text into the fragment.
    #[test]
    fn native_scoped_worker_without_manual_plan_has_current_assigned_contract() {
        let mut w = fixture();
        w.plans = None;
        let task = &mut w.tasks.as_mut().unwrap()[0];
        task.id = "native-child-task".into();
        task.title = "OWN-CURRENT-TITLE".into();
        task.deliverable = "OWN-CURRENT-DELIVERABLE".into();
        task.criteria = Some(vec!["OWN-CURRENT-CRITERION".into()]);
        task.next = "ROOT-PRIVATE-OLD-STATUS".into();
        crate::validate_planning_state(&w).unwrap();
        let text = current(&w, 0, "current-agent").unwrap();
        for expected in [
            "OWN-CURRENT-TITLE",
            "OWN-CURRENT-DELIVERABLE",
            "OWN-CURRENT-CRITERION",
            "native-child-task",
            "current-attempt",
            "OWN-REQUIREMENT",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
        assert!(!text.contains("ROOT-PRIVATE"));
        assert!(!text.contains("SIBLING-PRIVATE"));
    }
}
