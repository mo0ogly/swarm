package identitycontract

import (
	"strings"
	"testing"
)

func TestExistingNameRepresentationBoundaries(t *testing.T) {
	for _, tc := range []struct {
		name, value string
		valid       bool
	}{
		{"ordinary", "work-A_01", true}, {"hundred-bytes", strings.Repeat("a", 100), true},
		{"empty", "", false}, {"hundred-one-bytes", strings.Repeat("a", 101), false},
		{"path", "../work", false}, {"unicode", "équipe", false}, {"control", "work\x00", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if Name(tc.value) != tc.valid {
				t.Fatal("changed existing identifier representation")
			}
		})
	}
}

func TestOpaquePrincipalAndExactPermissionAreDifferentContracts(t *testing.T) {
	p := Principal{"human-1", "human", "https://[::1]/issuer", "employee[eu]"}
	if !p.Valid() {
		t.Fatal("opaque issuer/subject treated as a resource glob")
	}
	if (Permission{"memory.read", "memory/[eu]"}).Valid() {
		t.Fatal("permission resource accepted a glob")
	}
	p.Kind = "local"
	if p.Valid() {
		t.Fatal("external subject accepted as a local identity")
	}
	p.Issuer, p.Subject = "urn:swarm:local", p.ID
	if !p.Valid() {
		t.Fatal("existing local identity invalidated")
	}
}
