// Package resources embeds canonical Swarm assets without copying their source trees.
// This file remains at the module root because go:embed cannot traverse parent directories.
package resources

import "embed"

//go:embed web/*
var Web embed.FS

//go:embed tools/agent-workflows/CONTRACT.md tools/agent-workflows/templates/*.md .claude/skills/*/SKILL.md
var AgentWorkflow embed.FS

//go:embed tools/agent-workflows/templates/preparations.json
var Preparations embed.FS

//go:embed locales/en.json
var Translations embed.FS

//go:embed contracts/assistant-answer.v1.schema.json
var AssistantAnswerSchema []byte

//go:embed version_history.json
var VersionHistory []byte

//go:embed config/storage-retry.json
var StorageRetry []byte

//go:embed config/graph-performance.json
var GraphPerformance []byte

//go:embed config/automation.json
var Automation []byte

//go:embed config/web-session.json
var WebSession []byte
