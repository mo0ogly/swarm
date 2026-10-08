//go:build linux

package engine

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"unicode/utf8"

	"golang.org/x/sys/unix"
)

// Missing project files use the shipped pack. Existing overrides must pass the
// same descriptor-based boundary as document sources; never hide an unsafe file
// behind a bundled fallback or read through a replaced path component.
func (s *Store) preparationMethodSource(path string) ([]byte, error) {
	if !preparationSourcePath(path) || !preparationSourceType(path) {
		return nil, fmt.Errorf("Chemin de méthode refusé : %s", path)
	}
	fd, err := unix.Open(s.root, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC, 0)
	if err != nil {
		return nil, err
	}
	parts := strings.Split(path, "/")
	for i, part := range parts {
		flags := unix.O_RDONLY | unix.O_CLOEXEC | unix.O_NOFOLLOW | unix.O_NONBLOCK
		if i < len(parts)-1 {
			flags |= unix.O_DIRECTORY
		}
		next, openErr := unix.Openat(fd, part, flags, 0)
		unix.Close(fd)
		if errors.Is(openErr, unix.ENOENT) {
			data, readErr := agentWorkflowFiles.ReadFile(path)
			if readErr != nil {
				return nil, fmt.Errorf("Méthode embarquée indisponible : %s", path)
			}
			return preparationMethodText(path, data)
		}
		if openErr != nil {
			return nil, fmt.Errorf("Méthode locale inaccessible (lien ou accès refusé) : %s", path)
		}
		fd = next
	}
	file := os.NewFile(uintptr(fd), path)
	defer file.Close()
	info, err := file.Stat()
	if err != nil || !info.Mode().IsRegular() {
		return nil, fmt.Errorf("Fichier de méthode non régulier : %s", path)
	}
	data, err := io.ReadAll(io.LimitReader(file, 131073))
	if err != nil {
		return nil, fmt.Errorf("Fichier de méthode illisible : %s", path)
	}
	return preparationMethodText(path, data)
}

func preparationMethodText(path string, data []byte) ([]byte, error) {
	if len(data) == 0 || len(data) > 131072 || !utf8.Valid(data) || bytes.ContainsRune(data, 0) {
		return nil, fmt.Errorf("Méthode vide, binaire ou supérieure à 128 Kio : %s", path)
	}
	return data, nil
}
