//go:build linux

package engine

import (
	"bytes"
	"fmt"
	"sort"
)

type trustIdentityDisabled bool

func (value *trustIdentityDisabled) UnmarshalJSON(raw []byte) error {
	switch string(bytes.TrimSpace(raw)) {
	case "true":
		*value = true
	case "false":
		*value = false
	default:
		return fmt.Errorf("disabled doit être un booléen true ou false")
	}
	return nil
}

func trustPublicIdentities(identities []trustIdentity) []map[string]any {
	result := make([]map[string]any, 0, len(identities))
	for _, id := range identities {
		result = append(result, map[string]any{"subject": id.Subject, "integrity": id.Integrity, "roles": id.Roles, "disabled": bool(id.Disabled)})
	}
	sort.Slice(result, func(i, j int) bool { return result[i]["subject"].(string) < result[j]["subject"].(string) })
	return result
}
