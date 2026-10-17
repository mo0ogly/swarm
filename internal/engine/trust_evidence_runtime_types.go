package engine

import (
	"crypto/ed25519"
	"sync"
)

type trustEvidenceRuntime struct {
	mu       sync.Mutex
	revision int
	keys     map[string]ed25519.PrivateKey
}
