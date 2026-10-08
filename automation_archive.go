package main

import (
	"database/sql"
	"fmt"
)

func importAutomationTables(tx *sql.Tx, work string, tables []lifecycleTable) error {
	order := []string{"automation_requests", "automation_occurrences", "automation_effects", "automation_schedules", "automation_schedule_journal", "automation_request_origins", "automation_causal_recoveries"}
	allowed := map[string]bool{}
	byName := map[string]lifecycleTable{}
	for _, name := range order {
		allowed[name] = true
	}
	for _, table := range tables {
		if !allowed[table.Name] {
			return fmt.Errorf("table d’automatisation non autorisée")
		}
		if _, exists := byName[table.Name]; exists {
			return fmt.Errorf("table d’automatisation dupliquée")
		}
		columns := map[string]bool{}
		for _, column := range table.Columns {
			if !safeName(column) || columns[column] {
				return fmt.Errorf("colonnes d’automatisation invalides")
			}
			columns[column] = true
		}
		for _, row := range table.Rows {
			if len(row) != len(table.Columns) {
				return fmt.Errorf("ligne d’automatisation invalide")
			}
		}
		byName[table.Name] = table
	}
	value := func(table lifecycleTable, row []lifecycleCell, column string) string {
		for index, name := range table.Columns {
			if name == column && row[index].Kind == "text" {
				return row[index].Value
			}
		}
		return ""
	}
	requests, schedules := map[string]bool{}, map[string]bool{}
	for _, spec := range []struct {
		name, id string
		ids      map[string]bool
	}{{"automation_requests", "request_id", requests}, {"automation_schedules", "schedule_id", schedules}} {
		table := byName[spec.name]
		for _, row := range table.Rows {
			id := value(table, row, spec.id)
			if id == "" || spec.ids[id] || value(table, row, "target_work_id") != work {
				return fmt.Errorf("cible d’automatisation étrangère à l’archive")
			}
			spec.ids[id] = true
		}
	}
	for name, table := range byName {
		for _, row := range table.Rows {
			switch name {
			case "automation_occurrences", "automation_effects", "automation_request_origins", "automation_causal_recoveries":
				if !requests[value(table, row, "request_id")] {
					return fmt.Errorf("demande étrangère à l’archive")
				}
			}
			if (name == "automation_schedule_journal" || name == "automation_request_origins") && !schedules[value(table, row, "schedule_id")] {
				return fmt.Errorf("programme étranger à l’archive")
			}
			if name == "automation_effects" && value(table, row, "target_work_id") != work {
				return fmt.Errorf("effet étranger à l’archive")
			}
			if name == "automation_schedules" && value(table, row, "state") == "enabled" {
				for index, column := range table.Columns {
					if column == "state" {
						row[index] = lifecycleCell{Kind: "text", Value: "disabled"}
					}
				}
			}
		}
	}
	for _, name := range order {
		if table, exists := byName[name]; exists {
			if err := restoreTable(tx, table); err != nil {
				return fmt.Errorf("restauration d’automatisation atomique : %w", err)
			}
		}
	}
	return nil
}
