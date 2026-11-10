from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port')
from editlib import edit,atomic
p='rust/crates/swarm-engine/src/agent_execution_store.rs'
edit(p,'''        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        Ok(a)
    })
}

/// Preserve an owned process failure''','''        crate::agent_store::verify_agent_ownership(tx, &a)
            .map_err(|e| AgentLaunchError::caused(e.code(), e))?;
        if let Some(fence) = fence {
            let now = crate::system_timestamp()
                .map_err(|e| AgentLaunchError::caused("agent-recovery-clock", e))?;
            fence.check(tx, &a.work_id, &now)
                .map_err(|e| AgentLaunchError::caused("agent-recovery-mission-fence", e))?;
        }
        Ok(a)
    })
}

/// Preserve an owned process failure''')
edit(p,'''/// before callback admission.''','''/// before callback admission.''') if False else None
edit(p,'''/// owner/task/reservation changes. It adds no launch, consent or cancellation;''','''/// owner/task/reservation changes and after final readback. Stages the current
/// conductor link for a newly recorded task event in that transaction. It adds
/// no launch, consent or cancellation;''')
p=Path('docs/plans/migration-rust/PLAN.md');atomic(p,p.read_text()+'''\n1238 replan local : édition1236 partielle (2correspondances au second span),\nreçu provenance écrit mais fence finale non ajoutée. Build1237 lancé trop tôt\naprès cette erreur : compilation seule, jamais preuve du contrat complet.\nCorrection par contexte de fonction unique ; profil frais requis. Enregistrer\nprovenance du finish dans la même transaction et revérifier bail après readbacks.\n''')
s=Path('/tmp/rust_mission_worker_recovery.py').read_text()
s=s.replace("        assert task['status']!='done' and task['attempts']==json.loads(before['work'][0][0])['tasks'][0]['attempts'] or task['status']=='blocked'", "        previous=next(t for t in json.loads(before['work'][0][0])['tasks'] if t['id']=='task')\n        assert task['status'] in ['blocked','submitted']\n        assert [a['id'] for a in task['attempts']]==[a['id'] for a in previous['attempts']]")
atomic(Path('tests/rust_mission_worker_recovery.py'),s)
