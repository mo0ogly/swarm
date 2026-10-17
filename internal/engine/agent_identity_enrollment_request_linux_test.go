//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"reflect"
	"strings"
	"testing"
	"time"

	"swarm.local/companion/internal/enrollmentrequest"
	"swarm.local/companion/internal/enrollmentstate"
)

func enrollmentRequestCaller(t *testing.T, f agentIdentityIssuerFixture) (agentIdentityCaller, time.Time) {
	t.Helper()
	var caller agentIdentityCaller
	var at time.Time
	_, err := f.store.withTrustLock(false, func(st *trustState) (any, error) {
		at = time.Now().UTC()
		var err error
		caller, err = resolveAgentIdentityCaller(f.store.root, st, f.token, at)
		return nil, err
	})
	if err != nil {
		t.Fatal(err)
	}
	return caller, at
}
func enrollmentRequestReference(f agentIdentityIssuerFixture) enrollmentrequest.Enrollment {
	return enrollmentrequest.Enrollment{ID: "enrollment-fixture", Generation: 1, Head: enrollmentstate.Head{Sequence: 1, Digest: strings.Repeat("a", 64)}, DocumentDigest: f.authority.enrollmentDigest}
}

func TestAgentIdentityEnrollmentRequestUsesVerifiedSessionAndCompleteCandidate(t *testing.T) {
	f := issuerFixture(t)
	caller, at := enrollmentRequestCaller(t, f)
	ref := enrollmentRequestReference(f)
	raw := registryRaw(t, f.snapshot)
	prepared, err := prepareAgentIdentityEnrollmentRequest(f.store.root, caller, f.authority, ref, f.permit.base, f.permit.scope, raw, f.permit.eventID, f.permit.roots, at)
	if err != nil {
		t.Fatal(err)
	}
	p, err := prepared.Publication()
	if err != nil {
		t.Fatal(err)
	}
	if p.SessionDigest != trustTokenHash(f.token) || p.SessionDigest != caller.sessionDigest || !p.SessionVerifiedAt.Equal(at) || !p.SessionExpiresAt.Equal(*caller.expiresAt) || p.PreviousRegistryHead != f.permit.base.headDigest || p.SnapshotDigest != hash(p.Snapshot) || !reflect.DeepEqual(p.Roots, f.permit.roots) {
		t.Fatal("projection lost source bindings")
	}
	var snapshot agentIdentitySnapshot
	if err = json.Unmarshal(p.Snapshot, &snapshot); err != nil || !reflect.DeepEqual(snapshot, f.snapshot) {
		t.Fatal("snapshot projection omitted data", err)
	}
	before := prepared.Bytes()
	f.permit.roots[0] = "changed"
	raw[0] = '!'
	p.Snapshot[0] = '!'
	if !bytes.Equal(before, prepared.Bytes()) {
		t.Fatal("producer retained writable caller data")
	}
}

func TestAgentIdentityEnrollmentRequestRejectsInconsistentFramesAndSnapshots(t *testing.T) {
	original := issuerFixture(t)
	for _, kind := range []string{"missing-session", "stale-caller", "service", "instance", "project", "work", "plan", "enrollment-digest", "enrollment-generation", "roots", "invalid-record", "unknown-snapshot-field", "duplicate-snapshot-field"} {
		t.Run(kind, func(t *testing.T) {
			f := original.cloneProposal(t)
			caller, at := enrollmentRequestCaller(t, f)
			ref := enrollmentRequestReference(f)
			base, scope := f.permit.base, f.permit.scope
			raw := registryRaw(t, f.snapshot)
			switch kind {
			case "missing-session":
				caller.sessionDigest = ""
			case "stale-caller":
				caller.verifiedAt = at.Add(-time.Nanosecond)
			case "service":
				caller.actor.Principal.Kind = "service"
			case "instance":
				caller.actor.InstanceID = "self-asserted"
			case "project":
				base.projectRoot = t.TempDir()
			case "work":
				base.workID = "other"
			case "plan":
				scope.PlanVersion++
			case "enrollment-digest":
				ref.DocumentDigest = strings.Repeat("b", 64)
			case "enrollment-generation":
				ref.Generation = 2
			case "roots":
				f.permit.roots = []string{"missing"}
			case "invalid-record":
				for id, g := range f.snapshot.Grants {
					g.Permissions = nil
					f.snapshot.Grants[id] = g
					break
				}
				raw = registryRaw(t, f.snapshot)
			case "unknown-snapshot-field":
				raw = []byte(strings.TrimSuffix(string(raw), "}") + `,"unrecognized":true}`)
			case "duplicate-snapshot-field":
				raw = []byte(strings.TrimSuffix(string(raw), "}") + `,"revision":1}`)
			}
			if prepared, err := prepareAgentIdentityEnrollmentRequest(f.store.root, caller, f.authority, ref, base, scope, raw, f.permit.eventID, f.permit.roots, at); err == nil || len(prepared.Bytes()) != 0 {
				t.Fatal("inconsistent private projection succeeded", err)
			}
		})
	}
}
