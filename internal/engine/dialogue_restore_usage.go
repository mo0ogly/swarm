//go:build linux

package engine

func cloneDialogueUsage(v *Usage) *Usage {
	if v == nil {
		return nil
	}
	u := *v
	for _, dst := range []**int64{&u.CacheRead, &u.CacheCreation, &u.CachedInput} {
		if *dst != nil {
			n := **dst
			*dst = &n
		}
	}
	if u.ReportedCost != nil {
		n := *u.ReportedCost
		u.ReportedCost = &n
	}
	return &u
}

func addDialogueUsage(total, u *Usage) *Usage {
	if total == nil {
		total = &Usage{Source: "Événements fournisseur du dialogue", Scope: "Somme des usages rapportés par échange ; couverture partielle si un échange n’en fournit pas"}
	}
	for _, pair := range []struct {
		dst **int64
		src *int64
	}{{&total.CacheRead, u.CacheRead}, {&total.CacheCreation, u.CacheCreation}, {&total.CachedInput, u.CachedInput}} {
		if pair.src != nil {
			if *pair.dst == nil {
				n := int64(0)
				*pair.dst = &n
			}
			**pair.dst += *pair.src
		}
	}
	total.Input += u.Input
	total.Output += u.Output
	total.At = u.At
	if u.ReportedCost != nil {
		if total.ReportedCost == nil {
			n := 0.0
			total.ReportedCost = &n
		}
		*total.ReportedCost += *u.ReportedCost
	}
	return total
}

// Mission aggregates sum executions. The private guard and dialogue boundary
// retain conversation totals; projecting those totals again would double count.
func (m *dialogueMonitor) executionSnapshot() (AgentProgress, *Usage, string, string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	p, u, reason, activity := m.snapshotLocked()
	if base := m.baseProgress; base != nil {
		p.ToolCalls -= base.ToolCalls
		p.ToolResults -= base.ToolResults
		p.Reads -= base.Reads
		p.Writes -= base.Writes
		p.Unclassified -= base.Unclassified
		p.Errors -= base.Errors
		p.Repeats -= base.Repeats
		u = cloneDialogueUsage(m.runUsage)
	}
	return p, u, reason, activity
}
