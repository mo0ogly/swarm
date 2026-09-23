package main

// Keep this list explicit: mutating commands retain their existing migration
// behavior; common observation paths must never upgrade an old live Store.
func cliStorageInspection(pos []string) bool {
	if len(pos) < 2 {
		return false
	}
	switch pos[0] + " " + pos[1] {
	case "pricing list", "pricing estimate", "budget show", "budget preview", "work show", "work list", "planning show", "mission status", "mission preview", "agent show", "agent list", "agent logs", "agent prepared", "providers show", "connections list", "prepare list", "prepare show", "prepare history", "prepare methods", "workspace status", "exchange list", "lifecycle list":
		return true
	}
	return len(pos) > 2 && pos[0] == "providers" && pos[1] == "cooldown" && pos[2] == "show"
}
