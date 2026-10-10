// Package resources embeds canonical Swarm assets without copying their source trees.
// This file remains at the module root because go:embed cannot traverse parent directories.
package resources

import "embed"

//go:embed web/*
var Web embed.FS

//go:embed docs/training/casa-pizza/*.pdf docs/training/casa-pizza/*.docx docs/training/casa-pizza/*.zip docs/training/casa-pizza/tutoriels/swarm-logo.png docs/training/casa-pizza/tutoriels/index.html docs/training/casa-pizza/tutoriels/player.css docs/training/casa-pizza/tutoriels/player.js docs/training/casa-pizza/tutoriels/frames/*.png docs/training/casa-pizza/tutoriels/media/*
var Training embed.FS

//go:embed tools/agent-workflows/CONTRACT.md tools/agent-workflows/templates/*.md tools/agent-workflows/templates/ks/*.md .claude/skills/*/SKILL.md .claude/commands/ks-*.md
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

//go:embed config/preparation-timeout.json
var PreparationTimeout []byte

//go:embed config/provider-wait.json
var ProviderWait []byte
