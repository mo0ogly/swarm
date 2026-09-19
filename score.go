package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"os"
	"sort"
	"strconv"
	"strings"
)

type Evaluation struct {
	Method              string             `json:"method_version"`
	Scope               string             `json:"scope_id"`
	Phase               string             `json:"phase"`
	ConfigDigest        string             `json:"config_digest"`
	ApplicabilityDigest string             `json:"applicability_digest"`
	Artifacts           map[string]string  `json:"artifacts"`
	Scores              map[string]float64 `json:"domain_scores"`
	Quality             *float64           `json:"quality"`
	Provisional         bool               `json:"provisional"`
	Band                string             `json:"quality_band"`
	Progress            Progress           `json:"progress"`
	Severity            string             `json:"max_severity"`
	Blockers            []string           `json:"blockers"`
	Allowed             bool               `json:"allowed"`
	Ship                bool               `json:"ship_allowed"`
}
type Progress struct {
	Passed     int      `json:"passed"`
	Applicable int      `json:"applicable"`
	Percent    *float64 `json:"percent"`
}
type GateRecord struct {
	Name       string          `json:"name,omitempty"`
	Document   json.RawMessage `json:"document"`
	Evaluation Evaluation      `json:"evaluation"`
	At         string          `json:"at"`
}
type scoreRow struct {
	id, domain, gate, severity, status string
	mandatory                          bool
	count, penalty, max                float64
}

func hash(b []byte) string { d := sha256.Sum256(b); return hex.EncodeToString(d[:]) }
func decodeAny(b []byte) (map[string]any, error) {
	d := json.NewDecoder(bytes.NewReader(b))
	d.UseNumber()
	var m map[string]any
	e := d.Decode(&m)
	if e != nil {
		return nil, e
	}
	var extra any
	if err := d.Decode(&extra); err != io.EOF {
		return nil, fmt.Errorf("un seul document JSON requis")
	}
	if m == nil {
		return nil, fmt.Errorf("objet JSON requis")
	}
	return m, nil
}
func numeric(v any) (float64, error) {
	n, ok := v.(json.Number)
	if !ok {
		return 0, fmt.Errorf("nombre fini positif ou nul requis")
	}
	f, e := n.Float64()
	if e != nil || math.IsNaN(f) || math.IsInf(f, 0) || f < 0 {
		return 0, fmt.Errorf("nombre fini positif ou nul requis")
	}
	return f, nil
}
func str(v any) string { s, _ := v.(string); return s }
func records(v any) ([]map[string]any, error) {
	a, ok := v.([]any)
	if !ok {
		return nil, fmt.Errorf("liste requise")
	}
	seen := map[string]bool{}
	out := []map[string]any{}
	for _, v := range a {
		r, ok := v.(map[string]any)
		if !ok || !nonempty(str(r["id"])) || seen[str(r["id"])] {
			return nil, fmt.Errorf("identifiant absent ou dupliqué")
		}
		seen[str(r["id"])] = true
		out = append(out, r)
	}
	return out, nil
}

// Python json.dumps(sort_keys=True, ensure_ascii=False) separators for method 2.
func canonical(v any) string {
	switch x := v.(type) {
	case map[string]any:
		keys := []string{}
		for k := range x {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		a := []string{}
		for _, k := range keys {
			a = append(a, canonical(k)+": "+canonical(x[k]))
		}
		return "{" + strings.Join(a, ", ") + "}"
	case []any:
		a := []string{}
		for _, i := range x {
			a = append(a, canonical(i))
		}
		return "[" + strings.Join(a, ", ") + "]"
	default:
		var b bytes.Buffer
		e := json.NewEncoder(&b)
		e.SetEscapeHTML(false)
		_ = e.Encode(v)
		return strings.TrimSuffix(b.String(), "\n")
	}
}
func round2(v float64) *float64 {
	v, _ = strconv.ParseFloat(strconv.FormatFloat(v, 'f', 2, 64), 64)
	return &v
}
func evaluate(raw []byte, root, phase string) (Evaluation, error) {
	return evaluateWithDigests(raw, root, phase, nil)
}
func evaluateWithDigests(raw []byte, root, phase string, digests map[string]string) (Evaluation, error) {
	out := Evaluation{Method: "2", Phase: phase, Artifacts: map[string]string{}, Scores: map[string]float64{}, Blockers: []string{}, Severity: "none"}
	fail := func(s string) (Evaluation, error) { return out, fmt.Errorf("évaluation invalide : %s", s) }
	ranks := map[string]int{"entry": 0, "validation": 1, "delivery": 2, "audit": 2}
	rank, ok := ranks[phase]
	if !ok {
		return fail("phase inconnue")
	}
	d, e := decodeAny(raw)
	if e != nil {
		return out, e
	}
	if d["method_version"] != "2" || !nonempty(str(d["scope_id"])) {
		return fail("method_version ou scope_id")
	}
	out.Scope = str(d["scope_id"])
	arts, ok := d["artifacts"].(map[string]any)
	if !ok || len(arts) == 0 {
		return fail("empreintes absentes")
	}
	protected := map[string]bool{}
	for name, v := range arts {
		p, e := localFile(root, name)
		if e != nil {
			return out, e
		}
		digest, known := digests[p]
		if !known {
			b, e := os.ReadFile(p)
			if e != nil {
				return out, e
			}
			digest = hash(b)
			if digests != nil {
				digests[p] = digest
			}
		}
		if digest != str(v) {
			return fail("empreinte périmée : " + name)
		}
		out.Artifacts[name] = str(v)
		protected[p] = true
	}
	domains, ok := d["domains"].(map[string]any)
	if !ok || len(domains) == 0 {
		return fail("domaines absents")
	}
	weights := map[string]float64{}
	for k, v := range domains {
		f, e := numeric(v)
		if !nonempty(k) || e != nil || f == 0 {
			return fail("poids invalide")
		}
		weights[k] = f
	}
	checks, e := records(d["checks"])
	if e != nil || len(checks) == 0 {
		return fail("contrôles invalides")
	}
	results, e := records(d["results"])
	if e != nil {
		return out, e
	}
	obs := map[string]map[string]any{}
	known := map[string]bool{}
	for _, c := range checks {
		known[str(c["id"])] = true
	}
	for _, r := range results {
		if !known[str(r["id"])] {
			return fail("résultat inconnu")
		}
		obs[str(r["id"])] = r
	}
	rows := []scoreRow{}
	appIDs := []any{}
	penalties := map[string]float64{}
	severities := map[string]int{"none": 0, "minor": 1, "major": 2, "critical": 3}
	blocks := map[string]bool{}
	for _, c := range checks {
		id := str(c["id"])
		domain := str(c["domain"])
		if c["domain"] != nil {
			if _, ok := c["domain"].(string); !ok || weights[domain] == 0 {
				return fail(id + ": domaine inconnu")
			}
		}
		mandatory, ok := c["mandatory"].(bool)
		if !ok {
			return fail(id + ": mandatory doit être booléen")
		}
		gate := str(c["gate"])
		if gate != "entry" && gate != "validation" && gate != "delivery" {
			return fail(id + ": gate invalide")
		}
		sev := str(c["severity"])
		if _, ok := severities[sev]; !ok {
			return fail(id + ": sévérité invalide")
		}
		penalty, e := numeric(c["penalty"])
		if e != nil {
			return out, e
		}
		max, e := numeric(c["max_penalty"])
		if e != nil {
			return out, e
		}
		r, ok := obs[id]
		if !ok {
			r = map[string]any{"id": id, "status": "NON TESTÉ", "count": json.Number("0")}
		}
		for k := range r {
			if k != "id" && k != "status" && k != "count" && k != "evidence" && k != "reason" {
				return fail(id + ": résultat modifiant le barème")
			}
		}
		status := str(r["status"])
		if !strings.Contains("|PASS|FAIL|PARTIEL|NON TESTÉ|NON APPLICABLE|", "|"+status+"|") || status == "" {
			return fail(id + ": statut invalide")
		}
		count, e := numeric(r["count"])
		if e != nil {
			return out, e
		}
		if status == "FAIL" && count == 0 || (status == "PASS" || status == "NON APPLICABLE") && count != 0 {
			return fail(id + ": count incompatible")
		}
		if status == "NON APPLICABLE" && !nonempty(str(r["reason"])) {
			return fail(id + ": justification requise")
		}
		ev := []any{}
		if v, exists := r["evidence"]; exists {
			var ok bool
			ev, ok = v.([]any)
			if !ok {
				return fail(id + ": liste de preuves requise")
			}
		}
		if (status == "PASS" || status == "FAIL") && len(ev) == 0 {
			return fail(id + ": preuves absentes")
		}
		for _, v := range ev {
			p, e := localFile(root, str(v))
			if e != nil {
				return out, e
			}
			if !protected[p] {
				return fail(id + ": preuve hors inventaire")
			}
		}
		if status == "NON APPLICABLE" {
			continue
		}
		appIDs = append(appIDs, id)
		row := scoreRow{id, domain, gate, sev, status, mandatory, count, penalty, max}
		rows = append(rows, row)
		if domain != "" {
			if _, ok := penalties[domain]; !ok {
				penalties[domain] = 0
			}
			if status == "FAIL" {
				penalties[domain] += math.Min(count*penalty, max)
			}
			if status != "PASS" && status != "FAIL" {
				out.Provisional = true
			}
		}
	}
	sum, weight := 0.0, 0.0
	keys := []string{}
	for domain := range penalties {
		keys = append(keys, domain)
	}
	sort.Strings(keys)
	for _, domain := range keys {
		score := math.Max(0, 100-penalties[domain])
		out.Scores[domain] = score
		sum += score * weights[domain]
		weight += weights[domain]
	}
	var quality *float64
	if weight > 0 {
		q := sum / weight
		quality = &q
		out.Quality = round2(q)
	}
	for _, r := range rows {
		if ranks[r.gate] > rank {
			continue
		}
		out.Progress.Applicable++
		if r.status == "PASS" {
			out.Progress.Passed++
		}
		if phase == "audit" {
			if r.status != "PASS" && r.status != "FAIL" || r.domain == "" && r.mandatory && r.status != "PASS" {
				blocks[r.id] = true
			}
		} else {
			if r.mandatory && r.status != "PASS" || r.status == "FAIL" && severities[r.severity] >= 2 {
				blocks[r.id] = true
			}
		}
		if r.status == "FAIL" && severities[r.severity] > severities[out.Severity] {
			out.Severity = r.severity
		}
	}
	if out.Progress.Applicable == 0 {
		blocks["empty-applicable-inventory"] = true
	} else {
		out.Progress.Percent = round2(100 * float64(out.Progress.Passed) / float64(out.Progress.Applicable))
	}
	if phase == "delivery" {
		if out.Provisional || quality == nil {
			blocks["quality-incomplete"] = true
		}
		if quality != nil && *quality < 70 {
			blocks["quality-below-70"] = true
		}
		for _, s := range out.Scores {
			if s < 50 {
				blocks["domain-below-50"] = true
			}
		}
	}
	out.Band = "INCOMPLETE"
	if quality != nil && !out.Provisional {
		switch {
		case *quality >= 80:
			out.Band = "OK"
		case *quality >= 70:
			out.Band = "WARN"
		case *quality >= 50:
			out.Band = "BLOQUÉ"
		default:
			out.Band = "STOP"
		}
	}
	for k := range blocks {
		out.Blockers = append(out.Blockers, k)
	}
	sort.Strings(out.Blockers)
	out.Allowed = len(blocks) == 0
	out.Ship = phase == "delivery" && out.Allowed
	sort.Slice(checks, func(i, j int) bool { return str(checks[i]["id"]) < str(checks[j]["id"]) })
	ca := []any{}
	for _, c := range checks {
		ca = append(ca, c)
	}
	sort.Slice(appIDs, func(i, j int) bool { return str(appIDs[i]) < str(appIDs[j]) })
	out.ConfigDigest = hash([]byte(canonical(map[string]any{"domains": domains, "checks": ca})))
	out.ApplicabilityDigest = hash([]byte(canonical(appIDs)))
	return out, nil
}
func (s *Store) validGate(t *Task) bool {
	if t.Gate == nil || !t.Gate.Evaluation.Ship || requiredPlanChecks(t, t.Gate.Document) != nil {
		return false
	}
	ev, e := evaluateWithDigests(t.Gate.Document, s.root, "delivery", s.readDigests)
	return e == nil && ev.Ship && ev.Scope == t.ID && revalidationGate(t, ev) == nil
}

// A reopened or stale upstream task invalidates downstream acceptance as well.
func (s *Store) acceptedFresh(w *Work, t *Task, seen map[string]bool) bool {
	return s.acceptedFreshMemo(w, t, seen, map[string]bool{})
}

// Memoization is local to one traversal: never retained across filesystem checks.
func (s *Store) acceptedFreshMemo(w *Work, t *Task, seen, memo map[string]bool) bool {
	if t != nil && s.independentReviewGuard(w, t) != nil {
		return false
	}
	if t == nil || seen[t.ID] {
		return false
	}
	if result, ok := memo[t.ID]; ok {
		return result
	}
	memo[t.ID] = false
	if (t.Status != "accepted" || !s.validGate(t)) && !(t.Status == "waived" && t.Override != nil && nonempty(t.Override.Reason)) {
		return false
	}
	seen[t.ID] = true
	defer delete(seen, t.ID)
	for _, id := range t.Depends {
		dep, e := w.task(id)
		if e != nil || !s.acceptedFreshMemo(w, dep, seen, memo) {
			return false
		}
	}
	memo[t.ID] = true
	return true
}
