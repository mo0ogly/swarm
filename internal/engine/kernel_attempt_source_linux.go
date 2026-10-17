//go:build linux

package engine

import (
	"encoding/json"
	"errors"
	"net"
	"strings"
)

// Hash registered inputs, not executable bytes or future interactive input.
func kernelAttemptResourceDigest(a Agent) (string, error) {
	if a.Mode != "" || a.Provider == "" || a.Role == "" || a.Command == "" || a.CWD == "" || strings.TrimSpace(a.Prompt) == "" {
		return "", errors.New("registered batch observation input required")
	}
	contextDigest, workflowDigest := "", ""
	if a.Context != nil {
		contextDigest = a.Context.SHA256
		if !kernelObservationHex(contextDigest, 32) {
			return "", errors.New("invalid registered context digest")
		}
	}
	if a.Workflow != nil {
		workflowDigest = a.Workflow.SHA256
		if !kernelObservationHex(workflowDigest, 32) {
			return "", errors.New("invalid registered workflow digest")
		}
	}
	command, _ := json.Marshal(struct {
		Command   string
		Args      []string
		Directory string
	}{a.Command, a.Args, a.CWD})
	input := struct{ Domain, Agent, Mission, Task, Attempt, Provider, Role, Prompt, Command, Context, Workflow string }{"swarm-registered-attempt-input-v1", a.ID, a.WorkID, a.TaskID, a.Attempt, a.Provider, a.Role, hash([]byte(a.Prompt)), hash(command), contextDigest, workflowDigest}
	raw, err := json.Marshal(input)
	if err != nil {
		return "", err
	}
	return hash(raw), nil
}

func (s *Store) kernelAttemptObservationSource(agentID string) (kernelObservationLease, error) {
	var agentBody, workBody []byte
	var workID, taskID, status, desired string
	var paused bool
	err := s.db.QueryRow(`SELECT a.body,a.work_id,a.task_id,a.status,a.desired,w.body,COALESCE(c.paused,0)
 FROM agents a JOIN works w ON w.id=a.work_id
 LEFT JOIN cockpit_controls c ON c.work_id=a.work_id WHERE a.id=?`, agentID).Scan(&agentBody, &workID, &taskID, &status, &desired, &workBody, &paused)
	if err != nil {
		return kernelObservationLease{}, err
	}
	a, err := decodeAgentRow(agentBody, status, desired)
	if err != nil {
		return kernelObservationLease{}, err
	}
	var w Work
	if err = json.Unmarshal(workBody, &w); err != nil {
		return kernelObservationLease{}, err
	}
	if paused || a.ID != agentID || a.WorkID != workID || a.TaskID != taskID || w.ID != workID {
		return kernelObservationLease{}, errors.New("observation registry identity or authority changed")
	}
	task, err := w.task(taskID)
	if err != nil || task.Status != "running" || len(task.Attempts) == 0 {
		return kernelObservationLease{}, errors.New("observation task is not active")
	}
	attempt := task.Attempts[len(task.Attempts)-1]
	if attempt.ID != a.Attempt || attempt.Status != "recorded" || attempt.Ended != "" {
		return kernelObservationLease{}, errors.New("observation attempt was superseded or stopped")
	}
	digest, err := kernelAttemptResourceDigest(a)
	if err != nil {
		return kernelObservationLease{}, err
	}
	return newKernelObservationLease(a, digest)
}

// The launcher supplies its owned collector peer; callers cannot supply a digest.
func (s *Store) newKernelAttemptReceiver(c *net.UnixConn, p kernelObservationPeer, agentID, boot string) (*kernelObservationReceiver, error) {
	if !kernelObservationHex(boot, 16) {
		return nil, errors.New("owned collector boot identity required")
	}
	source, err := s.kernelAttemptObservationSource(agentID)
	if err != nil {
		return nil, err
	}
	stream := newKernelObservationStream(source, boot, "swarm-minimal-v1")
	return newKernelObservationReceiver(c, p, stream, func() (kernelObservationLease, error) { return s.kernelAttemptObservationSource(agentID) })
}
