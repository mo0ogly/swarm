//go:build linux

package main

import (
	"crypto/sha256"
	"sync"

	"github.com/tiktoken-go/tokenizer/codec"
)

// Fixed storage for pure tokenizer results only: no prompt bytes, authorization,
// capacity verdict, evidence or provider state. A digest collision in a slot
// replaces that slot; a different full digest is always recounted. This is an
// implementation memory bound, not an operator execution or acceptance budget.
type reviewTokenCountCache struct {
	slots [256]struct {
		sync.Mutex
		digest [sha256.Size]byte
		count  int
		valid  bool
	}
}

var reviewTokenCounts reviewTokenCountCache

func (c *reviewTokenCountCache) count(text string) (int, error) {
	// Include the exact tokenizer contract in the content identity.
	h := sha256.New()
	h.Write([]byte(managedReviewTokenizer))
	h.Write([]byte{0})
	h.Write([]byte(text))
	var key [sha256.Size]byte
	copy(key[:], h.Sum(nil))
	slot := &c.slots[key[0]]
	slot.Lock()
	defer slot.Unlock()
	if slot.valid && slot.digest == key {
		return slot.count, nil
	}
	// Each miss owns its regex state, including concurrent misses in other slots.
	n, err := codec.NewO200kBase().Count(text)
	if err != nil {
		return 0, err
	}
	slot.digest, slot.count, slot.valid = key, n, true
	return n, nil
}
