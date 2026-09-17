package main

import "fmt"

// Identifies one real reservation from the launch transaction, never a mix of rows.
type WorkspaceBusyError struct {
	AgentID string `json:"agent_id"`
	WorkID  string `json:"work_id"`
	TaskID  string `json:"task_id"`
	Title   string `json:"title"`
}

func (e *WorkspaceBusyError) Error() string {
	return fmt.Sprintf("Espace de travail occupé : agent %s (tâche %s, travail %s). Attendre sa fin ou suivre cette tentative.", e.AgentID, e.TaskID, e.WorkID)
}
