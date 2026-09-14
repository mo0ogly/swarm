//go:build !linux

package main

// La conduite automatique dépend de la supervision de processus, disponible
// sous Linux uniquement. Ailleurs, le règlement d'une fin de tentative reste
// identique : la tâche garde son blocage et son motif.
func (s *Store) conduct(a Agent, outcome string) {}

func (s *Store) dispatchAfterSettle(work string) {}
