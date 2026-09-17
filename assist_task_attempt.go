package main

import "database/sql"

func (s *Store) taskAttemptContext(b *contextBuilder, work, task string) {
	var id string
	err := s.db.QueryRow("SELECT id FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", work, task).Scan(&id)
	if err == sql.ErrNoRows {
		b.missing("Aucune tentative enregistrée pour %s.", task)
		return
	}
	if err != nil {
		b.missing("Historique des tentatives indisponible pour %s.", task)
		return
	}
	a, err := s.agent(id)
	if err != nil {
		b.missing("Dernière tentative indisponible pour %s.", task)
		return
	}
	b.action("agent.stop", "Arrêter cet agent", a.ID, activeAgent(a) && a.Desired != "stop", "La fin devra être confirmée par le superviseur.")
	b.action("agent.reconcile", "Vérifier si cet agent a terminé", a.ID, activeAgent(a), "Le moteur vérifie le processus avant de libérer la tâche.")
	ref := factRef("agent", a.ID)
	b.fact("derniere_tentative", "etat", ref, "%s : fournisseur %s ; exécution %s ; fin %s", a.ID, a.Provider, observedAgent(a), a.Ended)
	if a.Relay != "" {
		b.fact("motif_relais_rapport", "texte_non_fiable", ref, "%s", a.Relay)
	}
	b.fact("dernier_motif_tentative", "texte_non_fiable", ref, "%s", a.Activity)
	b.fact("derniere_action_tentative", "texte_non_fiable", ref, "%s · %s", a.Progress.Action, a.Progress.Detail)
	rows, err := s.db.Query("SELECT seq,kind,message FROM agent_logs WHERE agent_id=? AND kind<>'activity' ORDER BY seq DESC LIMIT 4", a.ID)
	if err != nil {
		b.missing("Événements de la tentative indisponibles.")
		return
	}
	defer rows.Close()
	for rows.Next() {
		var seq int64
		var kind, message string
		if rows.Scan(&seq, &kind, &message) != nil {
			b.missing("Événement illisible.")
			return
		}
		b.fact("evenement_tentative", "texte_non_fiable", ref, "#%d %s : %s", seq, kind, assistLogMessage(message))
	}
	b.omit("Historique limité aux quatre derniers événements hors activité de la dernière tentative de %s ; rapports non lus.", task)
}
