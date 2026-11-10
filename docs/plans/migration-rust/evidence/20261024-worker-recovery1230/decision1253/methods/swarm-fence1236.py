from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port')
from editlib import edit,atomic
p='rust/crates/swarm-engine/src/agent_execution_store.rs'
edit(p,'''            if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                return Err(AgentLaunchError::Refused("agent-execution-event-conflict"));
            }
''','''            if !matches!(staged, crate::RecordedChangeOutcome::Staged) {
                return Err(AgentLaunchError::Refused("agent-execution-event-conflict"));
            }
            if let Some(fence) = fence {
                fence.stage_receipt(tx, crate::work_engine::MutationFenceReceipt {
                    work: &a.work_id, event: &event, kind: "task.update",
                    work_revision: work.revision, command_settings: settings,
                }).map_err(|e| AgentLaunchError::caused("agent-recovery-mission-receipt", e))?;
            }
''')
edit(p,'''        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        Ok(a)
''','''        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if let Some(fence) = fence {
            let now = crate::system_timestamp()
                .map_err(|e| AgentLaunchError::caused("agent-recovery-clock", e))?;
            fence.check(tx, &a.work_id, &now)
                .map_err(|e| AgentLaunchError::caused("agent-recovery-mission-fence", e))?;
        }
        Ok(a)
''')
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1236 revue provenance : enregistrer le reçu task.update du conducteur de reprise\ndans la même transaction et revérifier son bail après les écritures/readbacks.\nÉchec de provenance ou bail expiré : rollback partagé, jamais succès vide.\nProfil1235 compilé conservé ; nouvelle qualification requise après cet ajout.\n''')
