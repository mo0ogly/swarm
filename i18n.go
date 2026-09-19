package main

import (
	"embed"
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

//go:embed locales/en.json
var translationsFS embed.FS
var englishUI = func() map[string]string {
	data, _ := translationsFS.ReadFile("locales/en.json")
	var catalog map[string]string
	if err := json.Unmarshal(data, &catalog); err != nil {
		panic(err)
	}
	return catalog
}()

// UI rendering only: callers must pass source labels/formats, never user data.
// The language is fixed before starting the process. JSON and stored data retain
// their original schema and content; unknown messages retain their source text.
func uiText(source string) string {
	if os.Getenv("SWARM_LANG") == "en" {
		if translated, ok := englishUI[source]; ok {
			return translated
		}
	}
	return source
}

func languageArgs(args []string) ([]string, string, error) {
	language := os.Getenv("SWARM_LANG")
	if language == "" {
		language = "fr"
	}
	filtered := make([]string, 0, len(args))
	for i := 0; i < len(args); i++ {
		if args[i] == "--lang" {
			if i+1 == len(args) {
				return nil, "", fmt.Errorf("--lang requires fr or en")
			}
			i++
			language = args[i]
		} else if strings.HasPrefix(args[i], "--lang=") {
			language = strings.TrimPrefix(args[i], "--lang=")
		} else {
			filtered = append(filtered, args[i])
		}
	}
	if language != "fr" && language != "en" {
		return nil, "", fmt.Errorf("unsupported language %q; use fr or en", language)
	}
	return filtered, language, nil
}

// Localize a known engine message after its canonical error has been classified.
// Captured identifiers, paths and provider details are never translated.
var uiFormat = regexp.MustCompile(`%(?:\.\d+)?[dsfqwv]`)
var uiMessages = func() []struct {
	pattern *regexp.Regexp
	target  string
} { keys := make([]string, 0); for source, target := range englishUI {
	if source != target && uiFormat.MatchString(source) {
		keys = append(keys, source)
	}
}; sort.Slice(keys, func(i, j int) bool {
	if len(keys[i]) == len(keys[j]) {
		return keys[i] < keys[j]
	}
	return len(keys[i]) > len(keys[j])
}); result := []struct {
	pattern *regexp.Regexp
	target  string
}{}; for _, source := range keys {
	positions := uiFormat.FindAllStringIndex(source, -1)
	expression := "(?s)^"
	last := 0
	for _, pos := range positions {
		expression += regexp.QuoteMeta(source[last:pos[0]]) + "(.*?)"
		last = pos[1]
	}
	expression += regexp.QuoteMeta(source[last:]) + "$"
	result = append(result, struct {
		pattern *regexp.Regexp
		target  string
	}{regexp.MustCompile(expression), englishUI[source]})
}; return result }()

func uiEngineText(source string) string {
	if os.Getenv("SWARM_LANG") != "en" {
		return source
	}
	if text := uiText(source); text != source {
		return text
	}
	if len(source) > 10000 {
		return source
	}
	for _, message := range uiMessages {
		if captures := message.pattern.FindStringSubmatch(source); captures != nil {
			index := 0
			return uiFormat.ReplaceAllStringFunc(message.target, func(_ string) string {
				index++
				if index < len(captures) {
					return captures[index]
				}
				return ""
			})
		}
	}
	return source
}
