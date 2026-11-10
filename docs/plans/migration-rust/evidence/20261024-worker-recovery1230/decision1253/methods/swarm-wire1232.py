from pathlib import Path
import sys,json
sys.path.insert(0,'.cache/rust-git-port');from editlib import atomic,edit
p='rust/crates/swarm-cli/src/mission_watch.rs'
edit(p,'/// old-generation queued candidate IDs and whether any automatic worker still','/// old-generation queued candidate IDs, unowned registered IDs, and whether any automatic worker still')
edit(p,') -> Result<(Vec<String>, bool), RootError> {',') -> Result<(Vec<String>, bool, Vec<String>), RootError> {')
edit(p,'    let mut candidates = Vec::new();\n    let mut active_context','    let mut candidates = Vec::new();\n    let mut registered = Vec::new();\n    let mut active_context')
edit(p,'        if adoption_candidate(known, &entry.state) {','        if !known && matches!(entry.state, State::RegisteredObserved) { registered.push(entry.agent_id.clone()); }\n        if adoption_candidate(known, &entry.state) {')
edit(p,'    failure.map_or(Ok((candidates, active_context)), Err)','    failure.map_or(Ok((candidates, active_context, registered)), Err)')
needle='/// Classify retained automatic context conservatively, without inferring exec'
edit(p,needle,'''/// Finalize only unowned registered observations after all continuation siblings
/// passed validation. Missing original/live owner stays visible waiting; invalid
/// evidence is fatal. A terminal result is verified/replayed, never new permission
/// or task acceptance. Preserve known lifecycle/Work through every later read/output
/// failure; observations are deduplicated per Agent/reason, not retained forever.
fn recover_registered(
    store: &mut RootStore, lease: &MissionLease, work: &str, agents: Vec<String>,
    waiting: &mut std::collections::BTreeMap<String,&'static str>,
    recorded: &mut Option<(String,i64)>, out: &mut dyn Write,
) -> Result<bool,RootError> {
    let mut terminal=false;
    for agent_id in agents {
        match store.recover_mission_worker(lease,&agent_id)? {
            swarm_engine::MissionWorkerRecovery::Waiting{reason,cause}=>{
                if waiting.get(&agent_id).copied()!=Some(reason) {
                    let event=json!({"schema_version":1,"event":"mission-watch-blocked","work":work,
                        "agent_id":agent_id,"reason":reason,"causes":crate::safe_causes(&cause),
                        "mission_complete":false,"observation_is_permission":false});
                    if let Err(publication)=emit(out,&event) { return Err(join(Some(cause),publication)); }
                    waiting.insert(agent_id,reason);
                }
            }
            swarm_engine::MissionWorkerRecovery::Finalized{agent}=>{
                terminal=true;waiting.remove(&agent_id);
                let current=store.read_work(work).map_err(|e|preserve_admission(true,e))?;
                *recorded=Some((current.id.clone(),current.revision));
                let event=json!({"schema_version":1,"event":"mission-worker-completion-recovered","work":work,
                    "agent_id":agent.id,"status":agent.status,"work_revision":current.revision,
                    "terminal_verified":true,"new_commit_proven":false,"provider_started_by_recovery":false,
                    "task_accepted":false,"mission_complete":false,"observation_is_permission":false});
                emit(out,&event).map_err(|e|preserve_planning(recorded.as_ref(),preserve_admission(true,e)))?;
            }
        }
    }
    Ok(terminal)
}

'''+needle)
edit(p,'    let mut waiting_reports = std::collections::BTreeMap::new();','    let mut waiting_reports = std::collections::BTreeMap::new();\n    let mut recovery_waiting = std::collections::BTreeMap::new();')
edit(p,'        let (candidates, active_context) = continuation(store, lease, children)?;','''        let (candidates, active_context, registered) = continuation(store, lease, children)?;
        if recover_registered(store,lease,work,registered,&mut recovery_waiting,planning_recorded,out)? {
            // Verified settlement changed the observed context. Discard this old
            // snapshot before any planning/adoption/dispatch; same lease/IDs remain.
            continue;
        }''')
p=Path('version_history.json');v=json.loads(p.read_text());v['local_changes']['items'].insert(0,{'fr':'Candidat Rust : récupération sous bail du reçu de fin d’un worker enregistré après départ de son superviseur ; sans relancement ni annulation queued, qualification en cours.','en':'Rust candidate: lease-fenced recovery of registered worker completion receipts after supervisor departure; no relaunch or queued cancellation, qualification pending.'});atomic(p,json.dumps(v,ensure_ascii=False,indent=2)+'\n')
for name,text in [('docs/architecture/NATIVE-WORKER-RECOVERY.md','''\n## Candidat1232 : finalisation sous le bail du conducteur\nLe conducteur CLI/web peut examiner ses agents automatiques déjà enregistrés\nsans handle local. Il ne les relance pas. Reçu absent et superviseur encore vivant\nsont des attentes explicites avec cause conservée ; un reçu invalide reste une\nerreur. La finalisation relit l'original, vérifie le départ réel du superviseur,\npuis contrôle le bail dans la transaction commune Agent/Work/réservation/FIFO.\nUne finalisation ou son rejeu ne vaut ni nouveau commit prouvé ni acceptation.\nLe contexte de continuation est relu avant le prochain dispatch. Le chemin\ngénérique qui annule les queued n'est pas utilisé. Les workers détenus restent\nsous leur propriétaire initial ; son échec est conservé, pas maquillé en succès.\nCe port concerne le journal de fin natif ; le règlement des dépôts gérés\n(planning.repository) reste explicitement refusé dans le producteur de settlement.\nQualification publique en cours, aucun restart/installation ni revue indépendante\ndu nouveau candidat établis.\n\n```mermaid\nsequenceDiagram\n  participant C as Conducteur sous bail\n  participant J as Journal original\n  participant T as Transaction de finalisation\n  C->>J: Agent automatique enregistré sans handle local\n  alt Original absent ou propriétaire vivant\n    J-->>C: Attente et cause typée, aucune action\n  else Original valide et propriétaire parti\n    C->>T: Relire preuve + fence du bail courant\n    T->>T: Même finalisation Agent/Work/réservation/FIFO\n    T-->>C: Terminal vérifié ou erreur complète\n    C->>C: Relire le contexte avant dispatch\n  end\n```\n'''),('docs/en/NATIVE-WORKER-RECOVERY.md','''\n## Candidate1232: conductor lease-fenced completion recovery\nCLI/web conductors may inspect registered automatic workers without a local\nchild handle; they never relaunch them. Missing receipts and a live original\nsupervisor return explicit waiting with retained typed causes; invalid originals\nremain errors. Recovery rechecks original proof and actual owner departure, then\nchecks the current lease inside the shared Agent/Work/reservation/FIFO settlement\ntransaction. Terminal success/replay proves neither a new commit nor acceptance.\nContinuation is reobserved before dispatch. Generic queued cancellation is never\ncalled. Owned workers retain their original lifecycle; its failure is not hidden.\nThis ports native completion journal recovery, not planning.repository managed\nsettlement, which remains explicitly refused. Public qualification is pending;\nno installation, restart or independent acceptance is established.\n\n```mermaid\nsequenceDiagram\n  participant C as Leased conductor\n  participant J as Original journal\n  participant T as Settlement transaction\n  C->>J: Unowned registered automatic worker\n  alt Missing original or live owner\n    J-->>C: Waiting and typed cause, no action\n  else Valid original and departed owner\n    C->>T: Recheck proof and current lease fence\n    T->>T: Same Agent/Work/reservation/FIFO settlement\n    T-->>C: Verified terminal state or complete error\n    C->>C: Reobserve before dispatch\n  end\n```\n''')]:
 p=Path(name);atomic(p,p.read_text()+text)
print('1232 wired into owned CLI/web loop; qualification pending')
