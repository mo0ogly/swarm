package main

import (
	"strings"
	"testing"
	"time"
)

func TestCompletionRetainsExchangeHistoryWithoutRequestingAction(t *testing.T) {
	w := Work{Tasks: []Task{{ID: "source", Title: "Source"}, {ID: "recipient", Title: "Destinataire"}}}
	for _, state := range []string{"pending", "acknowledged", "escalated", "stale"} {
		exchanges := []AgentExchange{{SourceTask: "source", RecipientTask: "recipient", Kind: "handoff", State: state}}
		done := MissionStatus{Understanding: MissionUnderstanding{Situation: "termine"}}
		phases := missionCoordinationPhases(&w, done, exchanges, time.Now())
		if !strings.Contains(phases[2].NextStep, "Aucune action requise") || exchanges[0].State != state {
			t.Fatalf("historique altéré ou action terminale : %+v", phases[2])
		}
		phases = missionCoordinationPhases(&w, MissionStatus{}, exchanges, time.Now())
		if strings.Contains(phases[2].NextStep, "Aucune action requise") {
			t.Fatalf("action masquée après réouverture : %+v", phases[2])
		}
	}
}
