//go:build linux

package main

// Only explicit public assistant text is retained. Reasoning, thinking, tool
// payloads and user messages remain outside this structured activity channel.
func publicAgentMessages(data map[string]any) []string {
	var messages []string
	if data["type"] == "item.completed" {
		item, _ := data["item"].(map[string]any)
		if item["type"] == "agent_message" {
			if text, _ := item["text"].(string); text != "" {
				messages = append(messages, text)
			}
		}
	}
	if data["type"] == "assistant" {
		message, _ := data["message"].(map[string]any)
		content, _ := message["content"].([]any)
		for _, value := range content {
			block, _ := value.(map[string]any)
			if block["type"] == "text" {
				if text, _ := block["text"].(string); text != "" {
					messages = append(messages, text)
				}
			}
		}
	}
	return messages
}
