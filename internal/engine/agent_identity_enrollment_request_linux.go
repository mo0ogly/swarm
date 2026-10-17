//go:build linux

package engine

import (
	"encoding/json"
	"time"

	"swarm.local/companion/internal/enrollmentrequest"
	"swarm.local/companion/internal/identitycontract"
)

// Project already-resolved private control frames into the complete wire request.
// This pure function neither reads Trust/SQL nor proves that a supplied plan,
// enrollment or checkpoint is current. The eventual adapter must resolve them
// under the appropriate serialization and reauthenticate at final use.
// The result is immutable data, not an issuance permit or human approval.
func prepareAgentIdentityEnrollmentRequest(root string, caller agentIdentityCaller, enrollment agentIdentityIssuerAuthority,
	reference enrollmentrequest.Enrollment, base agentIdentityRegistryCheckpoint, scope agentIdentityScope,
	raw []byte, eventID string, roots []string, at time.Time) (enrollmentrequest.Prepared, error) {
	deny := func() (enrollmentrequest.Prepared, error) {
		return enrollmentrequest.Prepared{}, agentIdentityError("agent_identity_enrollment_request_invalid")
	}
	if err := validateAgentIdentityIssuer(root, caller, enrollment, scope, at); err != nil {
		return enrollmentrequest.Prepared{}, err
	}
	if !base.valid() || base.projectRoot != root || base.workID != scope.WorkID || !agentIdentityHashValid(caller.sessionDigest) || reference.DocumentDigest != enrollment.enrollmentDigest {
		return deny()
	}
	snapshot, err := decodeAgentIdentitySnapshot(raw)
	if err != nil {
		return enrollmentrequest.Prepared{}, err
	}
	canonical, err := json.Marshal(snapshot)
	if err != nil {
		return deny()
	}
	head := agentIdentityRegistryHead{Schema: 1, ProjectDigest: hash([]byte(root)), Scope: scope, Revision: snapshot.Revision,
		SnapshotDigest: hash(canonical), TrustRevision: caller.trustRevision, Roots: append([]string(nil), roots...),
		Publisher: caller.actor.Principal, EventID: eventID, Previous: base.headDigest}
	if err = validateAgentIdentityRegistrySnapshot(snapshot, head); err != nil {
		return enrollmentrequest.Prepared{}, err
	}
	p := enrollmentrequest.Publication{Schema: 1, Purpose: enrollmentrequest.PublicationPurpose, ProjectDigest: head.ProjectDigest,
		Principal: identitycontract.Principal(caller.actor.Principal), SessionDigest: caller.sessionDigest,
		SessionVerifiedAt: caller.verifiedAt, SessionExpiresAt: *caller.expiresAt, Scope: identitycontract.Scope(scope), TrustRevision: caller.trustRevision,
		Enrollment: reference, EventID: eventID, SnapshotRevision: snapshot.Revision, SnapshotDigest: head.SnapshotDigest,
		Snapshot: canonical, Roots: head.Roots, PreviousRegistryHead: base.headDigest}
	prepared, err := enrollmentrequest.Prepare(p)
	if err != nil {
		return deny()
	}
	return prepared, nil
}
