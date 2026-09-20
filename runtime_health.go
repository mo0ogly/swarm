package main

import (
	"fmt"
	"os"
	"path/filepath"

	"golang.org/x/sys/unix"
)

// Keep a margin for durable results and journals. This is a preflight, not a
// reservation: callers must still handle a filesystem filling during an action.
const storageStopBytes = 256 * 1024 * 1024
const storageWarningBytes = 1024 * 1024 * 1024

type StorageVolume struct {
	Kind      string `json:"kind"`
	Path      string `json:"path"`
	Available uint64 `json:"available_bytes"`
	Inodes    uint64 `json:"available_inodes"`
	State     string `json:"state"`
}

type RuntimeHealth struct {
	State         string          `json:"state"`
	LaunchAllowed bool            `json:"launch_allowed"`
	ObservedAt    string          `json:"observed_at"`
	Message       string          `json:"message"`
	Next          string          `json:"next_step"`
	Volumes       []StorageVolume `json:"volumes"`
}

func readStorageVolume(path string) (uint64, uint64, error) {
	var st unix.Statfs_t
	if err := unix.Statfs(path, &st); err != nil {
		return 0, 0, err
	}
	inodes := st.Ffree
	if st.Files == 0 {
		inodes = ^uint64(0)
	} // Some filesystems do not expose inode limits.
	return st.Bavail * uint64(st.Bsize), inodes, nil
}

// No database read or write: the incident remains observable when SQLite cannot
// persist the conductor's heartbeat. Tests inject filesystem measurements only.
func (s *Store) runtimeHealth() RuntimeHealth {
	read := s.storageProbe
	if read == nil {
		read = readStorageVolume
	}
	h := RuntimeHealth{State: "ready", LaunchAllowed: true, ObservedAt: now(), Volumes: []StorageVolume{}}
	for _, target := range []struct{ kind, path string }{{"store", filepath.Join(s.root, ".swarm")}, {"temporary", os.TempDir()}} {
		bytes, inodes, err := read(target.path)
		v := StorageVolume{Kind: target.kind, Path: target.path, Available: bytes, Inodes: inodes, State: "ready"}
		if err != nil {
			v.State = "unknown"
		} else if bytes < storageStopBytes || inodes < 256 {
			v.State = "blocked"
		} else if bytes < storageWarningBytes {
			v.State = "warning"
		}
		h.Volumes = append(h.Volumes, v)
		if v.State == "unknown" || v.State == "blocked" {
			h.State = "blocked"
			h.LaunchAllowed = false
		} else if v.State == "warning" && h.State == "ready" {
			h.State = "warning"
		}
	}
	switch h.State {
	case "blocked":
		h.Message = "Stockage indisponible ou presque plein : nouveaux départs suspendus."
		h.Next = "Libérez de l’espace sur le volume signalé ou rétablissez son accès, puis vérifiez à nouveau. Swarm reprendra les opérations autorisées lorsque le stockage sera disponible ; les limites et les avis de revue restent applicables."
	case "warning":
		h.Message = "Le stockage approche de la saturation."
		h.Next = "Libérez de l’espace avant la prochaine exécution longue. Les départs seront suspendus sous 256 Mio disponibles."
	default:
		h.Message = "Le stockage est disponible."
		h.Next = "Les autres conditions de lancement restent contrôlées par le moteur."
	}
	return h
}

func (s *Store) storageGuard() error {
	h := s.runtimeHealth()
	if h.LaunchAllowed {
		return nil
	}
	return &CommandError{Code: "storage_unavailable", Message: fmt.Sprintf("%s %s", h.Message, h.Next)}
}
