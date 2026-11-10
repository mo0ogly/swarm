//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"sync"
	"time"

	"golang.org/x/sys/unix"
)

type agentIdentityRuntimeBinding struct {
	actor         agentIdentityActor
	id            string
	uid           uint32
	trustRevision int
	namespaces    [2]agentIdentityNamespace
	pins          [2]*os.File
	revoked       bool
}

// Only the controller may populate this table, before opening the execution
// gate. It is not a durable authority source or a JSON import mechanism.
type agentIdentityRuntime struct {
	mu          sync.Mutex
	projectRoot string
	closed      bool
	bindings    map[string]*agentIdentityRuntimeBinding
}

func newAgentIdentityRuntime(projectRoot string) (*agentIdentityRuntime, error) {
	if !filepath.IsAbs(projectRoot) || filepath.Clean(projectRoot) != projectRoot {
		return nil, agentIdentityError("agent_identity_project_required")
	}
	return &agentIdentityRuntime{projectRoot: projectRoot, bindings: map[string]*agentIdentityRuntimeBinding{}}, nil
}

// bootstrapPID and expected namespace inodes come from the controller-owned
// launcher while its gate remains closed, never from a worker request. Verify
// the pinned objects before registration. The caller retains the execution gate.
func (r *agentIdentityRuntime) pin(actor agentIdentityActor, id string, revision int, uid uint32, bootstrapPID int, expected [2]uint64) error {
	if r == nil || !actor.valid() || actor.InstanceID == "" || !safeName(id) || revision <= 0 || bootstrapPID <= 0 || expected[0] == 0 || expected[1] == 0 {
		return agentIdentityError("agent_identity_binding_invalid")
	}
	fd, err := unix.PidfdOpen(bootstrapPID, 0)
	if err != nil {
		return err
	}
	process := os.NewFile(uintptr(fd), "identity-bootstrap-pidfd")
	defer process.Close()
	pins, err := agentIdentityProcessNamespaces(process)
	if err != nil {
		return err
	}
	keep := false
	defer func() {
		if !keep {
			for _, file := range pins {
				file.Close()
			}
		}
	}()
	binding := &agentIdentityRuntimeBinding{actor: actor, id: id, uid: uid, trustRevision: revision, pins: pins}
	for i, file := range pins {
		binding.namespaces[i], err = agentIdentityNamespaceOf(file)
		if err != nil {
			return err
		}
		if binding.namespaces[i].inode != expected[i] {
			return agentIdentityError("agent_identity_bootstrap_mismatch")
		}
	}
	host, err := os.Open("/proc/self/ns/pid")
	if err != nil {
		return err
	}
	hostNS, err := agentIdentityNamespaceOf(host)
	host.Close()
	if err != nil {
		return err
	}
	if hostNS == binding.namespaces[0] {
		return agentIdentityError("agent_identity_isolation_required")
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		return agentIdentityError("agent_identity_runtime_closed")
	}
	for _, previous := range r.bindings {
		// Never reuse a revoked ID, instance or held PID namespace.
		if previous.id == id || previous.actor.InstanceID == actor.InstanceID || previous.namespaces[0] == binding.namespaces[0] {
			return agentIdentityError("agent_identity_binding_conflict")
		}
	}
	r.bindings[id] = binding
	keep = true
	return nil
}

func (r *agentIdentityRuntime) revoke(id string) error {
	if r == nil {
		return agentIdentityError("agent_identity_runtime_closed")
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed || r.bindings[id] == nil {
		return agentIdentityError("agent_identity_binding_unavailable")
	}
	r.bindings[id].revoked = true
	return nil
}

func (r *agentIdentityRuntime) Close() error {
	if r == nil {
		return nil
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		return nil
	}
	r.closed = true
	var first error
	for _, binding := range r.bindings {
		for _, file := range binding.pins {
			if err := file.Close(); err != nil && first == nil {
				first = err
			}
		}
	}
	return first
}

// The callback runs while the runtime lock is held: revocation and Close cannot
// race past evaluation. Commitment/effect execution would need its own protocol;
// the callback here only evaluates IA2.2 and never performs a business effect.
func (r *agentIdentityRuntime) withCaller(message *agentIdentityMessage, root string, revision int, at time.Time, fn func(agentIdentityCaller) error) error {
	if r == nil || message == nil || message.process == nil {
		return agentIdentityError("agent_identity_message_unattributed")
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		return agentIdentityError("agent_identity_runtime_closed")
	}
	if r.projectRoot != root {
		return agentIdentityError("agent_identity_binding_unavailable")
	}
	pins, err := agentIdentityProcessNamespaces(message.process)
	if err != nil {
		return err
	}
	defer func() {
		for _, file := range pins {
			file.Close()
		}
	}()
	var namespaces [2]agentIdentityNamespace
	for i, file := range pins {
		namespaces[i], err = agentIdentityNamespaceOf(file)
		if err != nil {
			return err
		}
	}
	for _, binding := range r.bindings {
		if binding.namespaces == namespaces && binding.uid == message.uid {
			if binding.revoked || binding.trustRevision != revision {
				return agentIdentityError("agent_identity_binding_unavailable")
			}
			// Sample only after all coordination waits and namespace lookups.
			// Nonzero values are an internal fixture clock, never JSON input.
			if at.IsZero() {
				at = time.Now().UTC()
			}
			return fn(agentIdentityCaller{projectRoot: root, actor: binding.actor, trustRevision: revision, bindingID: binding.id, verifiedAt: at})
		}
	}
	return agentIdentityError("agent_identity_message_unattributed")
}

// Internal bridge only. The snapshot/current authority producer is still an
// independently protected input, not supplied by the channel or its message.
// Lock order: receive outside Trust -> Trust shared -> runtime -> pure evaluator.
func (s *Store) evaluateAgentIdentityMessage(message *agentIdentityMessage, runtime *agentIdentityRuntime, snapshot agentIdentitySnapshot, authority agentIdentityAuthority, previous *agentIdentityDecision, at time.Time) (agentIdentityDecision, error) {
	var decision agentIdentityDecision
	if message == nil {
		return decision, agentIdentityError("agent_identity_message_unattributed")
	}
	request, err := decodeAgentIdentityRequest(message.raw)
	if err != nil {
		return decision, err
	}
	_, err = s.withTrustLock(false, func(state *trustState) (any, error) {
		return nil, runtime.withCaller(message, s.root, state.Revision, at, func(caller agentIdentityCaller) error {
			var e error
			if previous != nil {
				decision, e = reuseAgentIdentityDecision(*previous, snapshot, authority, caller, request, caller.verifiedAt)
			} else {
				decision, e = evaluateAgentIdentity(snapshot, authority, caller, request, caller.verifiedAt)
			}
			return e
		})
	})
	return decision, err
}
