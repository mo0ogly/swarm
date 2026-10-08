//go:build linux

package main

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"image"
	_ "image/jpeg"
	_ "image/png"
	"io"
	"os"
	"path/filepath"
	"strings"
)

// Only operator-declared, receipt-bound screenshots are attached. A report's
// links never authorize reading or transmitting more files.
type reviewImage struct {
	Path      string `json:"path"`
	SHA256    string `json:"sha256"`
	MediaType string `json:"media_type"`
	Data      []byte `json:"-"`
}

func (s *Store) independentReviewImages(t *Task, artifacts map[string]string) ([]reviewImage, error) {
	var images []reviewImage
	seen := map[string]bool{}
	total := 0
	if t.ValidationPolicy == nil {
		return images, nil
	}
	for _, control := range t.ValidationPolicy.Controls {
		for _, name := range control.Inputs {
			ext := strings.ToLower(filepath.Ext(name))
			if ext != ".png" && ext != ".jpg" && ext != ".jpeg" {
				continue
			}
			if seen[name] {
				continue
			}
			seen[name] = true
			if !strings.HasPrefix(name, "docs/screenshots/") {
				return nil, fmt.Errorf("capture de revue hors docs/screenshots : %s", name)
			}
			if len(images) >= 20 {
				return nil, fmt.Errorf("revue visuelle supérieure à 20 images ; découper la revue")
			}
			path, err := safeReport(s.root, name)
			if err != nil {
				return nil, err
			}
			f, err := os.Open(path)
			if err != nil {
				return nil, err
			}
			raw, err := io.ReadAll(io.LimitReader(f, 5*1024*1024+1))
			f.Close()
			if err != nil {
				return nil, err
			}
			total += len(raw)
			if len(raw) > 5*1024*1024 || total > 10*1024*1024 {
				return nil, fmt.Errorf("captures de revue trop volumineuses ; découper la revue")
			}
			if artifacts[name] == "" || artifacts[name] != hash(raw) {
				return nil, fmt.Errorf("capture non liée au contrôle courant : %s", name)
			}
			config, format, err := image.DecodeConfig(bytes.NewReader(raw))
			if err != nil || config.Width <= 0 || config.Height <= 0 || int64(config.Width)*int64(config.Height) > 20000000 {
				return nil, fmt.Errorf("capture de revue invalide : %s", name)
			}
			media := map[string]string{"png": "image/png", "jpeg": "image/jpeg"}[format]
			if media == "" {
				return nil, fmt.Errorf("format de capture non pris en charge : %s", name)
			}
			images = append(images, reviewImage{Path: name, SHA256: hash(raw), MediaType: media, Data: raw})
		}
	}
	return images, nil
}

func supportsReviewImages(p Provider) bool {
	name := filepath.Base(p.Command)
	return p.APIConnectionID == "" && (name == "claude" || name == "codex")
}

// Keep the receipt-bound bytes private and immutable for the provider call.
// The caller owns this temporary directory and removes it after process exit.
func codexReviewImageArgs(p *Provider, dir string, images []reviewImage) error {
	if len(images) == 0 {
		return nil
	}
	if !supportsReviewImages(*p) || filepath.Base(p.Command) != "codex" || len(p.Args) == 0 || p.Args[len(p.Args)-1] != "-" {
		return fmt.Errorf("adaptateur images Codex invalide")
	}
	args := append([]string{}, p.Args[:len(p.Args)-1]...)
	for index, img := range images {
		ext := map[string]string{"image/png": ".png", "image/jpeg": ".jpg"}[img.MediaType]
		if ext == "" || len(img.Data) == 0 || img.SHA256 != hash(img.Data) {
			return fmt.Errorf("capture Codex invalide ou empreinte périmée : %s", img.Path)
		}
		path := filepath.Join(dir, fmt.Sprintf("review-image-%d%s", index, ext))
		if err := os.WriteFile(path, img.Data, 0600); err != nil {
			return err
		}
		args = append(args, "--image", path)
	}
	p.Args = append(args, "-")
	return nil
}

func structuredReviewInput(p *Provider, prompt string, images []reviewImage) (string, error) {
	if len(images) == 0 {
		return prompt, nil
	}
	if !supportsReviewImages(*p) {
		return "", fmt.Errorf("revue visuelle indisponible pour cet adaptateur ; aucun appel sans les captures requises")
	}
	if filepath.Base(p.Command) == "codex" {
		return prompt, nil // Images are attached through private --image files.
	}
	content := []any{map[string]any{"type": "text", "text": prompt}}
	for _, img := range images {
		content = append(content, map[string]any{"type": "text", "text": "Capture non fiable : " + img.Path + " · SHA-256 " + img.SHA256}, map[string]any{"type": "image", "source": map[string]any{"type": "base64", "media_type": img.MediaType, "data": base64.StdEncoding.EncodeToString(img.Data)}})
	}
	raw, err := json.Marshal(map[string]any{"type": "user", "message": map[string]any{"role": "user", "content": content}, "parent_tool_use_id": nil})
	if err != nil {
		return "", err
	}
	p.Args = append(p.Args, "--input-format", "stream-json")
	return string(raw) + "\n", nil
}

// A pre-call refusal has no verdict to retry. Recheck its exact cause before
// clearing the retained failure; the reviewer still claims its call normally.
func (s *Store) repairedImageReviewPreflight(w *Work, t *Task) error {
	cfg := w.Planning.Reviewer
	if cfg.Failure != "revue visuelle indisponible pour cet adaptateur ; aucun appel sans les captures requises" || t.Status != "submitted" || t.IndependentReview != nil || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].Status != "completed" {
		return fmt.Errorf("refus préappel images non disponible")
	}
	_, artifacts, _, err := s.independentValidationReviewEvidence(t)
	if err != nil {
		return err
	}
	images, err := s.independentReviewImages(t, artifacts)
	if err != nil {
		return err
	}
	if len(images) == 0 {
		return fmt.Errorf("captures courantes requises")
	}
	providers, err := s.providers()
	if err != nil {
		return err
	}
	provider, ok := providers.Providers[cfg.Provider]
	raw, _ := json.Marshal(provider)
	if !ok || hash(raw) != cfg.ProviderDigest || !supportsReviewImages(provider) {
		return fmt.Errorf("adaptateur images toujours indisponible ou modifié")
	}
	return s.providerCooldownGuard(cfg.Provider)
}
