package engine

import (
	"context"
	"crypto/ecdsa"
	"sync"
	"sync/atomic"
	"time"
)

type trustServiceProof struct {
	material       *trustServiceMaterial
	requestContext context.Context
	tokenDigest    string
	revision       int
	checkedAt      time.Time
	expiresAt      time.Time
}
type trustServiceMaterial struct {
	revoked   atomic.Bool
	value     []byte
	revision  int
	expiresAt time.Time
}
type trustServiceRuntime struct {
	mu       sync.Mutex
	revision int
	key      *ecdsa.PrivateKey
	tokens   map[string]*trustServiceMaterial
}
