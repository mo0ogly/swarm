//go:build linux

package main

import (
	"bytes"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode/utf8"

	"golang.org/x/sys/unix"
)

const preparationSourceBytes = 12000
const preparationSourcesBytes = 24000

type PreparationSourceRef struct {
	Path  string `json:"path"`
	Start int    `json:"start_line,omitempty"`
	End   int    `json:"end_line,omitempty"`
	Hash  string `json:"sha256,omitempty"`
}
type PreparationSource struct {
	PreparationSourceRef
	Text string `json:"text"`
	At   string `json:"captured_at"`
}
type PreparationSourceList struct {
	Directories []string `json:"directories"`
	Paths       []string `json:"paths"`
	Limited     bool     `json:"limited"`
}

var sourceSecret = regexp.MustCompile(`(?i)(-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:api[_-]?key|password|passwd|access[_-]?token|client[_-]?secret)["']?\s*[:=]\s*["']?[^\s"'{}]{6,}|\b(?:sk-[A-Za-z0-9_-]{16,}|AKIA[A-Z0-9]{16})\b)`)

func preparationSourcePath(path string) bool {
	if path == "" || len(path) > 1024 || filepath.IsAbs(path) || strings.ContainsAny(path, "\\\x00\n\r") || filepath.Clean(path) != path {
		return false
	}
	for _, part := range strings.Split(filepath.ToSlash(path), "/") {
		low := strings.ToLower(part)
		if part == ".." || part == "." || strings.HasPrefix(part, ".") && part != ".claude" && part != ".agents" {
			return false
		}
		switch low {
		case "node_modules", "vendor", "__pycache__", "venv", "dist", "build", "backups", "outputs", "credentials", "secrets", "providers.json", "settings.json", "settings.local.json":
			return false
		}
		if strings.Contains(low, "credential") || strings.Contains(low, "secret") || strings.HasPrefix(low, "id_rsa") || strings.HasPrefix(low, "id_ed25519") {
			return false
		}
	}
	return true
}
func preparationSourceType(path string) bool {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".md", ".txt", ".go", ".py", ".js", ".ts", ".tsx", ".jsx", ".css", ".html", ".sql", ".json", ".toml", ".yaml", ".yml":
		return true
	}
	return false
}

// Open every component without following links; verify the opened descriptor,
// not a path checked earlier. Nonblocking protects against swapped FIFO/device.
func (s *Store) openPreparationSource(path string) (*os.File, error) {
	if !preparationSourcePath(path) || !preparationSourceType(path) {
		return nil, preparationError("source_refused", "Chemin exclu du périmètre documentaire autorisé.")
	}
	fd, e := unix.Open(s.root, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC, 0)
	if e != nil {
		return nil, e
	}
	parts := strings.Split(path, "/")
	for i, part := range parts {
		flags := unix.O_RDONLY | unix.O_CLOEXEC | unix.O_NOFOLLOW | unix.O_NONBLOCK
		if i < len(parts)-1 {
			flags |= unix.O_DIRECTORY
		}
		next, err := unix.Openat(fd, part, flags, 0)
		unix.Close(fd)
		if err != nil {
			return nil, preparationError("source_refused", "Fichier absent, lien symbolique ou accès refusé.")
		}
		fd = next
	}
	file := os.NewFile(uintptr(fd), path)
	info, e := file.Stat()
	if e != nil || !info.Mode().IsRegular() || info.Size() > 262144 {
		file.Close()
		return nil, preparationError("source_refused", "Seuls les fichiers texte réguliers de 256 Kio maximum sont consultables.")
	}
	return file, nil
}
func (s *Store) preparationReadSource(path string, start, end int) (PreparationSource, error) {
	var v PreparationSource
	if start < 1 || end < start || end-start >= 200 {
		return v, preparationError("source_range", "Choisissez de 1 à 200 lignes contiguës.")
	}
	file, e := s.openPreparationSource(path)
	if e != nil {
		return v, e
	}
	defer file.Close()
	b, e := io.ReadAll(io.LimitReader(file, 262145))
	if e != nil {
		return v, e
	}
	if len(b) > 262144 || !utf8.Valid(b) || bytes.ContainsRune(b, 0) {
		return v, preparationError("source_refused", "Fichier binaire, trop volumineux ou non UTF-8.")
	}
	if sourceSecret.Match(b) {
		return v, preparationError("source_refused", "Ce fichier contient une valeur ressemblant à un secret. Fournissez un extrait nettoyé dans le besoin.")
	}
	lines := strings.Split(string(b), "\n")
	if start > len(lines) {
		return v, preparationError("source_range", "La ligne de début dépasse le fichier.")
	}
	end = min(end, len(lines))
	text := strings.Join(lines[start-1:end], "\n")
	if len(text) > preparationSourceBytes {
		return v, preparationError("source_range", "Extrait supérieur à 12 000 octets ; sélectionnez moins de lignes.")
	}
	return PreparationSource{PreparationSourceRef: PreparationSourceRef{Path: path, Start: start, End: end, Hash: hash(b)}, Text: text, At: now()}, nil
}
func (s *Store) preparationSourceFiles(scope, query string) (PreparationSourceList, error) {
	out := PreparationSourceList{Paths: []string{}}
	if scope == "" {
		scope = "."
	}
	if scope != "." && !preparationSourcePath(scope) || len(query) > 120 || !utf8.ValidString(query) {
		return out, preparationError("source_refused", "Périmètre ou recherche invalide.")
	}
	full := filepath.Join(s.root, scope)
	info, e := os.Lstat(full)
	if e != nil || !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return out, preparationError("source_refused", "Répertoire de recherche indisponible.")
	}
	// Resolve scope itself and forbid any symlink parent before enumerating names.
	real, e := filepath.EvalSymlinks(full)
	if e != nil || real != full {
		return out, preparationError("source_refused", "Répertoire symbolique refusé.")
	}
	entries, e := os.ReadDir(full)
	if e != nil {
		return out, e
	}
	out.Directories = []string{}
	for _, entry := range entries {
		rel := filepath.Join(scope, entry.Name())
		if entry.IsDir() && entry.Type()&os.ModeSymlink == 0 && preparationSourcePath(rel) {
			if len(out.Directories) >= 100 {
				out.Limited = true
				break
			}
			out.Directories = append(out.Directories, rel)
		}
	}
	visited := 0
	query = strings.ToLower(query)
	e = filepath.WalkDir(full, func(path string, d fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return nil
		}
		visited++
		if visited > 20000 {
			out.Limited = true
			return fs.SkipAll
		}
		rel, err := filepath.Rel(s.root, path)
		if err != nil {
			return err
		}
		if rel == "." {
			return nil
		}
		if d.Type()&os.ModeSymlink != 0 {
			return nil
		}
		if !preparationSourcePath(rel) {
			if d.IsDir() {
				return fs.SkipDir
			}
			return nil
		}
		if d.IsDir() {
			return nil
		}
		if !d.Type().IsRegular() || !preparationSourceType(rel) || !strings.Contains(strings.ToLower(rel), query) {
			return nil
		}
		if len(out.Paths) >= 100 {
			out.Limited = true
			return fs.SkipAll
		}
		out.Paths = append(out.Paths, rel)
		return nil
	})
	return out, e
}
func (s *Store) editPreparationSource(p *Preparation, r PreparationRequest) error {
	if r.Source == nil {
		return preparationError("source_refused", "Référence de source requise.")
	}
	index := -1
	for i, v := range p.Sources {
		if v.Path == r.Source.Path {
			index = i
			break
		}
	}
	if r.Action == "source-remove" {
		if index < 0 {
			return preparationError("source_refused", "Source absente de cette préparation.")
		}
		p.Sources = append(p.Sources[:index], p.Sources[index+1:]...)
	} else {
		v, e := s.preparationReadSource(r.Source.Path, r.Source.Start, r.Source.End)
		if e != nil {
			return e
		}
		if r.Source.Hash == "" || v.Hash != r.Source.Hash {
			return preparationError("stale_source", "Le fichier a changé. Relisez l’extrait avant de le joindre.")
		}
		if index < 0 && len(p.Sources) >= 8 {
			return preparationError("source_limit", "Huit extraits maximum par préparation.")
		}
		size := len(v.Text)
		for i, source := range p.Sources {
			if i != index {
				size += len(source.Text)
			}
		}
		if size > preparationSourcesBytes {
			return preparationError("source_limit", fmt.Sprintf("Les extraits dépassent %d octets ; retirez ou raccourcissez une source.", preparationSourcesBytes))
		}
		if index < 0 {
			p.Sources = append(p.Sources, v)
		} else {
			p.Sources[index] = v
		}
	}
	// Existing adopted documents remain history; changing the evidence set requires
	// an explicit brief review before a new plan can be validated.
	p.Brief = nil
	p.Verdict = nil
	return nil
}
