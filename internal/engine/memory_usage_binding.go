//go:build linux

package engine

import (
	"encoding/json"
	"path/filepath"
	"unicode/utf8"
)

// Private proposal data, never an authenticated destination or a permission.
// A future consumer must resolve this target from protected current state.
type memoryUsageTarget struct {
	Scope     agentIdentityScope `json:"scope"`
	Task      string             `json:"task"`
	Recipient agentIdentityActor `json:"recipient"`
}

type memoryUsageSourceBinding struct {
	ID       string `json:"id"`
	Task     string `json:"source_task"`
	Kind     string `json:"kind"`
	Role     string `json:"role"`
	Resource string `json:"resource"`
	SHA256   string `json:"sha256"`
	Bytes    int    `json:"bytes"`
	Transmit bool   `json:"transmit"`
}

type memoryUsageProjection struct {
	Protocol        string                     `json:"protocol"`
	Project         string                     `json:"project_context_digest"`
	Work            string                     `json:"source_work"`
	Revision        int                        `json:"work_revision"`
	Target          memoryUsageTarget          `json:"target"`
	SourceSHA       string                     `json:"source_packet_sha256"`
	SelectionSHA    string                     `json:"selection_sha256"`
	AttachmentSHA   string                     `json:"attachment_sha256"`
	AttachmentBytes int                        `json:"attachment_bytes"`
	Sources         []memoryUsageSourceBinding `json:"sources"`
}

// No JSON admission path or source text. The digest binds requests, not rights.
// Construction performs no SQL, identity evaluation, commitment or transmission.
type memoryUsageBinding struct {
	projection memoryUsageProjection
	digest     string
}

// Rebuild the supported M19 pipeline on the caller-owned Work snapshot. This
// does not validate plan adoption, enroll/attest the recipient or establish its
// right to receive. Those checks belong to the future transactional consumer.
func memorySummaryUsageBinding(root string, w Work, r Launch, target memoryUsageTarget) (memoryUsageBinding, error) {
	empty := memoryUsageBinding{}
	if !filepath.IsAbs(root) || filepath.Clean(root) != root || w.Schema != 1 || !safeName(w.ID) || w.Revision <= 0 ||
		r.Summary == nil || !safeName(r.TaskID) || target.Task != r.TaskID || !target.Scope.valid() || target.Scope.WorkID != w.ID ||
		!target.Recipient.valid() || target.Recipient.InstanceID == "" {
		return empty, agentIdentityError("memory_usage_binding_invalid")
	}
	body, receipt, err := memorySummaryAttachment(root, w, r)
	if err != nil {
		return empty, err
	}
	if receipt == nil || receipt.AttachmentSHA != hash([]byte(body)) || receipt.AttachmentBytes != len(body) {
		return empty, agentIdentityError("memory_usage_binding_invalid")
	}
	packet, sources, err := summarySources(root, w, r.TaskID, *r.Summary)
	if err != nil {
		return empty, err
	}
	received := map[string]bool{}
	for _, id := range receipt.Received {
		if received[id] || id == "current-task-snapshot" {
			return empty, agentIdentityError("memory_usage_sources_invalid")
		}
		received[id] = true
	}
	projection := memoryUsageProjection{Protocol: "swarm-memory-usage-v2", Project: hash([]byte(root)), Work: w.ID, Revision: w.Revision, Target: target,
		SourceSHA: packet.SHA256, SelectionSHA: receipt.SelectionSHA, AttachmentSHA: receipt.AttachmentSHA, AttachmentBytes: receipt.AttachmentBytes}
	seen := map[string]bool{}
	for _, source := range sources {
		if !safeName(source.ID) || seen[source.ID] || !utf8.ValidString(source.Text) || !agentIdentityHashValid(source.SHA256) || source.SHA256 != hash([]byte(source.Text)) {
			return empty, agentIdentityError("memory_usage_sources_invalid")
		}
		seen[source.ID] = true
		sourceTask := r.TaskID
		if source.Dialogue != nil {
			sourceTask = source.Dialogue.Task
		}
		if !safeName(sourceTask) {
			return empty, agentIdentityError("memory_usage_sources_invalid")
		}
		key, err := json.Marshal(struct {
			Protocol string `json:"protocol"`
			Project  string `json:"project_context_digest"`
			Work     string `json:"work"`
			Task     string `json:"task"`
			ID       string `json:"id"`
			Kind     string `json:"kind"`
			Role     string `json:"role"`
		}{"swarm-memory-resource-v2", projection.Project, w.ID, sourceTask, source.ID, source.Kind, source.Role})
		if err != nil {
			return empty, err
		}
		projection.Sources = append(projection.Sources, memoryUsageSourceBinding{ID: source.ID, Task: sourceTask, Kind: source.Kind, Role: source.Role,
			Resource: "memory:v2:" + hash(key), SHA256: source.SHA256, Bytes: len(source.Text), Transmit: received[source.ID]})
	}
	for id := range received {
		if !seen[id] {
			return empty, agentIdentityError("memory_usage_sources_invalid")
		}
	}
	digest, err := agentIdentityDigest(projection)
	if err != nil {
		return empty, err
	}
	return memoryUsageBinding{projection: projection, digest: digest}, nil
}

// Each loaded source needs independent read and summarization checks for this
// reconstruction pipeline. Only received sources need a transmission check.
// Every check shares the complete proposal digest, including exact output and
// recipient. The kernel still needs a protected frame and fresh caller; these
// requests alone confer no permission and are never durable consumption receipts.
func (b memoryUsageBinding) requests(eventID, grantID string) ([]agentIdentityRequest, error) {
	digest, err := agentIdentityDigest(b.projection)
	if err != nil {
		return nil, err
	}
	if !safeName(eventID) || !safeName(grantID) || !agentIdentityHashValid(b.digest) || digest != b.digest ||
		b.projection.Protocol != "swarm-memory-usage-v2" || !b.projection.Target.Scope.valid() || len(b.projection.Sources) == 0 {
		return nil, agentIdentityError("memory_usage_binding_invalid")
	}
	requests := []agentIdentityRequest{}
	for _, source := range b.projection.Sources {
		operations := []string{"memory.read", "memory.summarize"}
		if source.Transmit {
			operations = append(operations, "memory.transmit")
		}
		for _, operation := range operations {
			key, err := json.Marshal([]string{"swarm-memory-usage-event-v1", eventID, source.Resource, operation})
			if err != nil {
				return nil, err
			}
			request := agentIdentityRequest{Schema: 1, EventID: "memory-" + hash(key), Scope: b.projection.Target.Scope, GrantID: grantID,
				Operation: operation, Resource: source.Resource, ContentDigest: b.digest}
			if !request.valid() {
				return nil, agentIdentityError("memory_usage_binding_invalid")
			}
			requests = append(requests, request)
		}
	}
	return requests, nil
}
