//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"reflect"
)

type DialogueRecoveryCheck struct {
	Code    string `json:"code"`
	Status  string `json:"status"`
	Message string `json:"message"`
}

// A diagnosis is neither an authorization nor a successful provider preflight.
// No provider command, reservation or recovery mutation belongs on this path.
type DialogueRecoveryPreview struct {
	Boundary         *DialogueBoundaryView   `json:"boundary"`
	Path             string                  `json:"recommended_path"`
	Recommendation   string                  `json:"recommendation"`
	ResumeSupported  bool                    `json:"resume_supported"`
	DecisionRecorded bool                    `json:"decision_recorded"`
	DecisionOwner    string                  `json:"decision_owner"`
	ObservedAt       string                  `json:"observed_at"`
	FactsSHA256      string                  `json:"facts_sha256"`
	Checks           []DialogueRecoveryCheck `json:"checks"`
}

func dialogueRecoveryCheckLabel(status string) string {
	switch status {
	case "pass":
		return "Vérifié"
	case "fail":
		return "Bloquant"
	default:
		return "Non vérifié"
	}
}

func dialogueRecoveryRecommendation(path string) string {
	switch path {
	case "select_latest_attempt":
		return "Examiner la tentative la plus récente."
	case "wait_for_stop":
		return "Confirmer la fin du processus avant toute nouvelle exécution."
	case "inspect_history":
		return "État historique incomplet : diagnostiquer avant de choisir une nouvelle tentative."
	case "reconcile_effects":
		return "Vérifier les effets incertains ; ne pas rejouer automatiquement les outils."
	case "review_current_contract":
		return "Réexaminer le contrat courant et la cause des refus avant de décider."
	default:
		return "Frontière confirmée à examiner ; la session et la décision de reprise restent à vérifier."
	}
}

// Select from retained storage, independently of the recent UI history window.
func (s *Store) recoveryAgent(work, task, id string) (*Agent, error) {
	if id != "" {
		a, err := s.agent(id)
		if err != nil {
			return nil, err
		}
		if a.WorkID != work || a.TaskID != task {
			return nil, fmt.Errorf("tentative hors tâche ou introuvable")
		}
		return &a, nil
	}
	var body []byte
	var status, desired string
	err := s.db.QueryRow("SELECT body,status,desired FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", work, task).Scan(&body, &status, &desired)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	a, err := decodeAgentRow(body, status, desired)
	return &a, err
}

func (s *Store) dialogueRecovery(w Work, t *Task, a *Agent) (*DialogueRecoveryPreview, error) {
	if a == nil || a.Mode != "dialogue" {
		return nil, nil
	}
	boundary, c, err := s.readDialogueBoundary(*a)
	if err != nil {
		return nil, err
	}
	r := &DialogueRecoveryPreview{Boundary: boundary, Path: "review_boundary", DecisionOwner: "responsible_supervisor_or_local_operator", ObservedAt: now(), Checks: []DialogueRecoveryCheck{}}
	add := func(code, status, message string) {
		r.Checks = append(r.Checks, DialogueRecoveryCheck{code, status, message})
	}
	latest, err := s.recoveryAgent(w.ID, t.ID, "")
	if err != nil {
		return nil, err
	}
	current := latest != nil && latest.ID == a.ID && len(t.Attempts) > 0 && t.Attempts[len(t.Attempts)-1].ID == a.Attempt
	if current {
		add("latest_attempt", "pass", "La tentative sélectionnée est la dernière de cette tâche.")
	} else {
		add("latest_attempt", "fail", "La tentative sélectionnée ne correspond pas à l’état courant de la tâche.")
	}
	ended := recoveryProcessEnded(*a, hostIdentity(), processStamp(a.Child))
	if ended {
		add("process_ended", "pass", "La fin du processus fournisseur est confirmée.")
	} else {
		add("process_ended", "fail", "La fin du processus fournisseur n’est pas confirmée ; aucun second départ.")
	}
	if !boundary.Available {
		add("confirmed_boundary", "unknown", "Aucun état de dialogue exploitable ; examiner l’historique sans reconstituer des gardes supposées.")
		add("effects_observed", "unknown", "Les effets antérieurs restent à vérifier avant toute nouvelle exécution.")
		add("historical_guards", "unknown", "Les gardes cumulées de la conversation sont inconnues.")
	} else {
		if boundary.Confirmed {
			add("confirmed_boundary", "pass", "La fin du tour et son statut répondu sont confirmés.")
		} else {
			add("confirmed_boundary", "fail", "Le dernier tour ne possède pas une fin et un statut répondu confirmés.")
		}
		if len(c.Guard.Pending) > 0 || c.Guard.Degraded != "" {
			add("effects_observed", "fail", "Un résultat d’outil manque ou sa visibilité est perdue ; qualifier les effets avant toute reprise.")
		} else if boundary.Confirmed {
			add("effects_observed", "pass", "Les résultats des outils observés sont reçus à cette frontière.")
		} else {
			add("effects_observed", "unknown", "Un état de début ou interrompu ne prouve pas les effets ultérieurs.")
		}
		if c.Guard.Reason != "" || c.Guard.Degraded != "" {
			add("historical_guards", "fail", "Une garde historique bloque la poursuite ; examiner sa cause sans remettre les compteurs à zéro.")
		} else {
			add("historical_guards", "pass", "Les gardes historiques sont conservées sans remise à zéro.")
		}
	}
	providerSHA := ""
	providers, providerErr := s.providers()
	provider, exists := providers.Providers[a.Provider]
	configuration, caps := "unknown", "unknown"
	if providerErr == nil {
		configuration = "fail"
		if exists {
			raw, _ := json.Marshal(provider)
			providerSHA = hash(raw)
			purpose := "work"
			if a.Brainstorm {
				purpose = "brainstorm"
			}
			level := ""
			if a.ModelRoute != nil {
				level = a.ModelRoute.Level
			}
			resolved, route, routeErr := resolveModel(provider, level, purpose)
			info, statErr := os.Stat(provider.Command)
			if dialogueProvider(provider) == nil && routeErr == nil && statErr == nil && info.Mode().IsRegular() && info.Mode()&0111 != 0 && resolved.Command == a.Command && reflect.DeepEqual(resolved.Args, a.Args) && reflect.DeepEqual(resolved.Env, a.Env) && reflect.DeepEqual(route, a.ModelRoute) {
				configuration = "pass"
			}
			admin, capErr := configuredRunLimitsWith(s.db, w.ID, a.Role, t.ID)
			if capErr == nil {
				caps = "fail"
				bounded := a.Limits.cappedBy(provider.Limits).cappedBy(admin)
				bounded = administrativeDurationThresholds(bounded, admin)
				bounded.ObservationMode = admin.ObservationMode
				if bounded == a.Limits {
					caps = "pass"
				}
			}
		}
	}
	if configuration == "pass" {
		add("launch_configuration", configuration, "Les paramètres de commande, de modèle et d’environnement autorisé correspondent au lancement conservé.")
	} else {
		add("launch_configuration", configuration, "La configuration fournisseur est différente ou indisponible ; examiner le contrat actuel.")
	}
	if caps == "pass" {
		add("current_caps", caps, "Les plafonds historiques restent compatibles avec les réglages courants.")
	} else {
		add("current_caps", caps, "Les réglages courants imposent un nouvel examen des gardes ; aucun plafond n’est relevé.")
	}
	if s.projectMemoryGuard(*a) == nil {
		add("memory_sources", "pass", "Les références mémoire sélectionnées restent valides à cette lecture.")
	} else {
		add("memory_sources", "fail", "Une référence mémoire reçue est retirée, périmée ou inaccessible ; refaire la sélection avant une nouvelle exécution.")
	}
	if s.providerCooldownGuard(a.Provider) == nil {
		add("provider_wait", "pass", "Aucune attente fournisseur enregistrée ne bloque cette lecture ; cela ne prouve pas sa disponibilité.")
	} else {
		add("provider_wait", "fail", "Une attente fournisseur ou son état illisible bloque la poursuite ; aucun délai n’est effacé.")
	}
	add("launch_preflight", "unknown", "Session fournisseur, accès actuels et réservations non vérifiés par cet aperçu.")
	add("supervisor_decision", "unknown", "Le responsable doit décider de la suite sur des preuves fraîches ; cet aperçu n’enregistre aucune autorisation.")
	switch {
	case !current:
		r.Path = "select_latest_attempt"
	case !ended:
		r.Path = "wait_for_stop"
	case !boundary.Available:
		r.Path = "inspect_history"
	case !boundary.Confirmed:
		r.Path = "reconcile_effects"
	default:
		for _, check := range r.Checks {
			if check.Status == "fail" || check.Status == "unknown" && (check.Code == "launch_configuration" || check.Code == "current_caps") {
				r.Path = "review_current_contract"
				break
			}
		}
	}
	r.Recommendation = dialogueRecoveryRecommendation(r.Path)
	// Fingerprint observed facts, excluding the observation clock. This is not
	// an approval token and is never accepted by a mutation endpoint.
	raw, _ := json.Marshal(struct {
		Work                      string
		Revision                  int
		Agent                     string
		Status, Desired, Provider string
		Boundary                  *DialogueBoundaryView
		Checks                    []DialogueRecoveryCheck
	}{w.ID, w.Revision, a.ID, a.Status, a.Desired, providerSHA, boundary, r.Checks})
	r.FactsSHA256 = hash(raw)
	return r, nil
}
