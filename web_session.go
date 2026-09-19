//go:build linux

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

// A private bearer credential scoped to this project and listening address.
// Removing the file while the server is stopped revokes old session links.
func (s *Store) webSessionToken(address string) (string, error) {
	path := filepath.Join(s.root, ".swarm", "web-session-"+hash([]byte(address))[:16])
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if err == nil {
		token := newID("session-")
		_, err = f.WriteString(token)
		if err == nil {
			err = f.Sync()
		}
		closeErr := f.Close()
		if err != nil {
			return "", err
		}
		return token, closeErr
	}
	if !os.IsExist(err) {
		return "", err
	}
	f, err = os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW, 0)
	if err != nil {
		return "", err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return "", err
	}
	if !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 || info.Size() > 128 {
		return "", fmt.Errorf("clé de session web invalide ou permissions trop larges : %s", path)
	}
	raw := make([]byte, info.Size())
	if _, err = io.ReadFull(f, raw); err != nil {
		return "", err
	}
	token := string(raw)
	if !strings.HasPrefix(token, "session-") || len(token) != len("session-")+24 || !safeName(token) {
		return "", fmt.Errorf("clé de session web invalide : %s", path)
	}
	return token, nil
}
