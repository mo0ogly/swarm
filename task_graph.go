package main

import "fmt"

// Validate the graph independently of archive ordering. Parent links between live
// agents are immutable and can only target an existing agent, preventing cycles.
func validateTaskGraph(tasks []Task) error {
	graph := map[string][]string{}
	for _, task := range tasks {
		if _, exists := graph[task.ID]; exists {
			return fmt.Errorf("tâche dupliquée : %s", task.ID)
		}
		graph[task.ID] = task.Depends
		seen := map[string]bool{}
		for _, parent := range task.Depends {
			if seen[parent] {
				return fmt.Errorf("dépendance dupliquée : %s → %s", parent, task.ID)
			}
			seen[parent] = true
		}
	}
	state := map[string]int{}
	var visit func(string) error
	visit = func(id string) error {
		if _, ok := graph[id]; !ok {
			return fmt.Errorf("dépendance absente : %s", id)
		}
		if state[id] == 1 {
			return fmt.Errorf("cycle de dépendances : %s", id)
		}
		if state[id] == 2 {
			return nil
		}
		state[id] = 1
		for _, parent := range graph[id] {
			if err := visit(parent); err != nil {
				return err
			}
		}
		state[id] = 2
		return nil
	}
	for id := range graph {
		if err := visit(id); err != nil {
			return err
		}
	}
	return nil
}
