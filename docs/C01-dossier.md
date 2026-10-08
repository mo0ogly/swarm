# Dossier de revue — C01 demandes durables

## Portée et décision

Candidat non accepté : HEAD de base `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`,
diff sale partagé, tentative `a-172cbdafe408aee38282d60e`. Les seuls fichiers
fonctionnels attribués à C01 sont `automation_requests.go`,
`automation_requests_test.go`, les raccords v25 de `model.go`/`store.go`, les
raccords de cycle de vie de `lifecycle.go` et les guides `docs/AUTOMATION.md` et
`docs/en/AUTOMATION.md`.

Deux architectures ont été comparées :

| Option | Atomicité et reprise | Compatibilité | Décision |
| --- | --- | --- | --- |
| Journal fichier verrouillé | verrou et recovery séparés de la mission ; risque de double autorité | nouveau format, nouvel export et nouvelle restauration | refusée |
| Tables transactionnelles du `Store` | même verrou SQLite, transactions, FK, migration et cycle de vie | prolonge le stockage existant | retenue |

Le signal `automation-request` est l'effet durable de `request_resume`. Il
modifie l'empreinte observée par le conducteur existant, qui réévalue ensuite la
mission avec ses gardes normales. Il ne crée pas lui-même une tentative et ne
promet pas une exécution fournisseur exactement une fois.

## Contrat de stockage complet

```go
const automationRequestMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS automation_requests(
 request_id TEXT PRIMARY KEY,
 idempotency_key TEXT NOT NULL UNIQUE,
 content_digest TEXT NOT NULL,
 target_work_id TEXT NOT NULL REFERENCES works(id),
 source TEXT NOT NULL,
 action TEXT NOT NULL CHECK(action='request_resume'),
 occurrence_id TEXT NOT NULL UNIQUE,
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_occurrences(
 occurrence_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 state TEXT NOT NULL CHECK(state IN ('received','waiting','claimed','executed','rejected','uncertain_effect')),
 revision INTEGER NOT NULL,
 claimed_by TEXT NOT NULL,
 lease_until TEXT NOT NULL,
 effect_id TEXT NOT NULL,
 reason TEXT NOT NULL,
 actor TEXT NOT NULL,
 next_action TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_effects(
 effect_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE REFERENCES automation_requests(request_id),
 target_work_id TEXT NOT NULL REFERENCES works(id),
 action TEXT NOT NULL CHECK(action='request_resume'),
 created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS automation_occurrences_claimable ON automation_occurrences(state,lease_until);
PRAGMA user_version=25;
COMMIT;`

type AutomationRequestConfig struct {
        Version           int `json:"version"`
        ClaimLeaseSeconds int `json:"claim_lease_seconds"`
}

func defaultAutomationRequestConfig() AutomationRequestConfig {
        return AutomationRequestConfig{Version: 1, ClaimLeaseSeconds: 30}
}

func (c AutomationRequestConfig) validate() error {
        if c.Version != 1 || c.ClaimLeaseSeconds < 5 || c.ClaimLeaseSeconds > 300 {
                return fmt.Errorf("configuration automation invalide : version 1 et bail de 5 à 300 secondes requis")
        }
        return nil
}
```

Raccord migration exact : `schemaVersion` passe de 24 à 25 dans le candidat
sale courant, puis `openStoreWithMigration` sauvegarde une base existante avant
d'exécuter `automationRequestMigration`. Le test repart explicitement d'une base
isolée marquée v24 et vérifie v25 et la table. Le cycle de vie ajoute, dans cet
ordre de restauration, `automation_requests`, `automation_occurrences`, puis
`automation_effects`; la suppression parcourt la liste en sens inverse avant
`works`.

## Identités et lecture jointes — code complet

```go
type AutomationResumeRequest struct {
        Schema         int    `json:"schema_version"`
        IdempotencyKey string `json:"idempotency_key"`
        ContentDigest  string `json:"content_digest"`
        Source         string `json:"source"`
        TargetWorkID   string `json:"target_work_id"`
        Action         string `json:"action"`
}

type AutomationRequestRecord struct {
        RequestID      string `json:"request_id"`
        OccurrenceID   string `json:"occurrence_id"`
        IdempotencyKey string `json:"idempotency_key"`
        ContentDigest  string `json:"content_digest"`
        TargetWorkID   string `json:"target_work_id"`
        Source         string `json:"source"`
        Action         string `json:"action"`
        State          string `json:"state"`
        Revision       int    `json:"revision"`
        ClaimedBy      string `json:"claimed_by,omitempty"`
        LeaseUntil     string `json:"lease_until,omitempty"`
        EffectID       string `json:"effect_id,omitempty"`
        Reason         string `json:"reason,omitempty"`
        Actor          string `json:"actor,omitempty"`
        NextAction     string `json:"next_action,omitempty"`
        CreatedAt      string `json:"created_at"`
        UpdatedAt      string `json:"updated_at"`
}

func scanAutomationRequest(row interface{ Scan(...any) error }) (AutomationRequestRecord, error) {
        var r AutomationRequestRecord
        err := row.Scan(&r.RequestID, &r.OccurrenceID, &r.IdempotencyKey, &r.ContentDigest,
                &r.TargetWorkID, &r.Source, &r.Action, &r.State, &r.Revision, &r.ClaimedBy,
                &r.LeaseUntil, &r.EffectID, &r.Reason, &r.Actor, &r.NextAction, &r.CreatedAt, &r.UpdatedAt)
        return r, err
}

const automationRequestSelect = `SELECT r.request_id,r.occurrence_id,r.idempotency_key,r.content_digest,
 r.target_work_id,r.source,r.action,r.state,r.revision,o.claimed_by,o.lease_until,o.effect_id,
 r.reason,r.actor,r.next_action,r.created_at,r.updated_at
 FROM automation_requests r JOIN automation_occurrences o ON o.request_id=r.request_id`

func (s *Store) automationRequest(id string) (AutomationRequestRecord, error) {
        return scanAutomationRequest(s.db.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
}
```

La jointure rend visibles dans un même enregistrement l'identité durable de la
demande, celle de son occurrence, le propriétaire du bail et l'identité d'effet.
Le reviewer peut ainsi suivre sans déduction cachée les lectures réalisées par
`submitAutomationResume`, `claimAutomationRequest` et
`processAutomationRequest`.

## Contenu, digest et autorisation — code critique complet

```go
func automationResumeDigest(r AutomationResumeRequest) string {
        canonical, _ := json.Marshal(struct {
                Schema       int    `json:"schema_version"`
                Source       string `json:"source"`
                TargetWorkID string `json:"target_work_id"`
                Action       string `json:"action"`
        }{r.Schema, r.Source, r.TargetWorkID, r.Action})
        sum := sha256.Sum256(canonical)
        return hex.EncodeToString(sum[:])
}

func validateAutomationResumeRequest(r AutomationResumeRequest) error {
        if r.Schema != 1 || !safeName(r.IdempotencyKey) || !safeName(r.TargetWorkID) || r.Action != "request_resume" {
                return &CommandError{Code: "invalid_input", Message: "schema_version, idempotency_key, cible et action request_resume requis"}
        }
        if strings.TrimSpace(r.Source) == "" || len([]byte(r.Source)) > 200 {
                return &CommandError{Code: "invalid_input", Message: "source requise et limitée à 200 octets"}
        }
        if r.ContentDigest != automationResumeDigest(r) {
                return &CommandError{Code: "invalid_input", Message: "content_digest ne correspond pas au contenu canonique"}
        }
        return nil
}

func automationTerminal(w Work) bool {
        if w.Planning == nil {
                return false
        }
        root, err := w.Planning.scope("root")
        return err == nil && root.State == "closed"
}

func automationPolicyTx(tx *sql.Tx, work string) (bool, string, error) {
        var body []byte
        if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&body); err != nil {
                return false, "unknown_target", err
        }
        var w Work
        if err := json.Unmarshal(body, &w); err != nil {
                return false, "invalid_target", err
        }
        var archived int
        if err := tx.QueryRow("SELECT count(*) FROM mission_lifecycle WHERE work_id=?", work).Scan(&archived); err != nil {
                return false, "storage_error", err
        }
        if archived > 0 || automationTerminal(w) {
                return false, "terminal_target", nil
        }
        if err := organizationGuard(w); err != nil {
                return false, "authorization_required", nil
        }
        var policyRaw string
        if err := tx.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", work).Scan(&policyRaw); err != nil {
                if errors.Is(err, sql.ErrNoRows) {
                        return false, "authorization_required", nil
                }
                return false, "storage_error", err
        }
        var policy MissionPolicy
        if err := json.Unmarshal([]byte(policyRaw), &policy); err != nil {
                return false, "authorization_required", nil
        }
        var autonomy string
        var paused int
        if err := tx.QueryRow("SELECT autonomy,paused FROM cockpit_controls WHERE work_id=?", work).Scan(&autonomy, &paused); err != nil {
                return false, "authorization_required", nil
        }
        if !policy.Enabled || autonomy != autonomyAuto || paused != 0 {
                return false, "authorization_required", nil
        }
        if w.Planning != nil && (w.Planning.Paused || w.Planning.Failure != "" || w.Planning.Activations >= w.Planning.MaxActivations || w.Planning.Decisions >= w.Planning.MaxDecisions) {
                return false, "budget_or_planning_blocked", nil
        }
        return true, "", nil
}
```

## Réception idempotente — fonction complète

```go
func (s *Store) submitAutomationResume(r AutomationResumeRequest) (AutomationRequestRecord, bool, error) {
        var zero AutomationRequestRecord
        if err := validateAutomationResumeRequest(r); err != nil {
                return zero, false, err
        }
        tx, err := s.db.Begin()
        if err != nil {
                return zero, false, err
        }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", r.TargetWorkID); err != nil {
                return zero, false, err
        }
        existing, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.idempotency_key=?", r.IdempotencyKey))
        if err == nil {
                if existing.ContentDigest != r.ContentDigest || existing.TargetWorkID != r.TargetWorkID || existing.Source != r.Source || existing.Action != r.Action {
                        return zero, false, &CommandError{Code: "idempotency_conflict", Message: "clé déjà utilisée pour un contenu différent"}
                }
                return existing, false, nil
        }
        if !errors.Is(err, sql.ErrNoRows) {
                return zero, false, err
        }
        requestID := "request-" + hash([]byte(r.IdempotencyKey))[:24]
        occurrenceID := "occurrence-" + hash([]byte(requestID))[:24]
        stamp := now()
        state, reason, actor, next := "received", "", "request-service", "claim"
        authorized, code, policyErr := automationPolicyTx(tx, r.TargetWorkID)
        if policyErr != nil {
                if errors.Is(policyErr, sql.ErrNoRows) {
                        return zero, false, &CommandError{Code: "unknown_target", Message: "cible inconnue"}
                }
                return zero, false, policyErr
        }
        if !authorized {
                state, reason, next = "rejected", code, "none"
        }
        if _, err = tx.Exec(`INSERT INTO automation_requests(request_id,idempotency_key,content_digest,target_work_id,source,action,occurrence_id,state,revision,reason,actor,next_action,created_at,updated_at)
 VALUES(?,?,?,?,?,?,?,?,1,?,?,?,?,?)`, requestID, r.IdempotencyKey, r.ContentDigest, r.TargetWorkID, r.Source, r.Action, occurrenceID, state, reason, actor, next, stamp, stamp); err != nil {
                return zero, false, err
        }
        if _, err = tx.Exec(`INSERT INTO automation_occurrences(occurrence_id,request_id,state,revision,claimed_by,lease_until,effect_id,reason,actor,next_action,created_at,updated_at)
 VALUES(?,?,?,1,'','','',?,?,?, ?,?)`, occurrenceID, requestID, state, reason, actor, next, stamp, stamp); err != nil {
                return zero, false, err
        }
        if err = tx.Commit(); err != nil {
                return zero, false, err
        }
        got, err := s.automationRequest(requestID)
        return got, err == nil, err
}
```

## Attribution, attente et effet — gardes complètes

```go
func (s *Store) automationWorkspaceBusyTx(tx *sql.Tx, work string) (bool, error) {
        var body []byte
        if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&body); err != nil {
                return false, err
        }
        var w Work
        if err := json.Unmarshal(body, &w); err != nil {
                return false, err
        }
        workspaces := []string{}
        if w.Profile != nil && w.Profile.Workspace != "" {
                workspaces = append(workspaces, w.Profile.Workspace)
        }
        for _, task := range w.Tasks {
                if task.Profile != nil && task.Profile.Workspace != "" {
                        workspaces = append(workspaces, task.Profile.Workspace)
                }
        }
        for _, workspace := range workspaces {
                var count int
                if err := tx.QueryRow(`SELECT count(*) FROM agents WHERE status IN ('queued','starting','running','stopping')
 AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)`, workspace, workspace, workspace).Scan(&count); err != nil {
                        return false, err
                }
                if count > 0 {
                        return true, nil
                }
        }
        return false, nil
}

func updateAutomationStateTx(tx *sql.Tx, id, state, reason, actor, next string) error {
        stamp := now()
        result, err := tx.Exec(`UPDATE automation_requests SET state=?,revision=revision+1,reason=?,actor=?,next_action=?,updated_at=? WHERE request_id=?`, state, reason, actor, next, stamp, id)
        if err != nil {
                return err
        }
        if n, _ := result.RowsAffected(); n != 1 {
                return sql.ErrNoRows
        }
        _, err = tx.Exec(`UPDATE automation_occurrences SET state=?,revision=revision+1,reason=?,actor=?,next_action=?,updated_at=? WHERE request_id=?`, state, reason, actor, next, stamp, id)
        return err
}

func (s *Store) claimAutomationRequest(id, conductor string, at time.Time, config AutomationRequestConfig) (AutomationRequestRecord, bool, error) {
        var zero AutomationRequestRecord
        if !safeName(id) || !safeName(conductor) {
                return zero, false, &CommandError{Code: "invalid_input", Message: "identités de demande et conducteur invalides"}
        }
        if err := config.validate(); err != nil {
                return zero, false, err
        }
        tx, err := s.db.Begin()
        if err != nil {
                return zero, false, err
        }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil {
                return zero, false, err
        }
        record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
        if err != nil {
                return zero, false, err
        }
        if record.State == "executed" || record.State == "rejected" || record.State == "uncertain_effect" {
                return record, false, nil
        }
        if record.State == "claimed" && record.ClaimedBy != conductor {
                until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
                if parseErr == nil && at.Before(until) {
                        return record, false, nil
                }
        }
        authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
        if err != nil {
                return zero, false, err
        }
        if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil {
                        return zero, false, err
                }
                if err = tx.Commit(); err != nil {
                        return zero, false, err
                }
                got, readErr := s.automationRequest(id)
                return got, false, readErr
        }
        busy, err := s.automationWorkspaceBusyTx(tx, record.TargetWorkID)
        if err != nil {
                return zero, false, err
        }
        if busy {
                if err = updateAutomationStateTx(tx, id, "waiting", "workspace_wait", conductor, "retry_after_resource_release"); err != nil {
                        return zero, false, err
                }
                if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil {
                        return zero, false, err
                }
                if err = tx.Commit(); err != nil {
                        return zero, false, err
                }
                got, readErr := s.automationRequest(id)
                return got, false, readErr
        }
        lease := at.Add(time.Duration(config.ClaimLeaseSeconds) * time.Second).UTC().Format(time.RFC3339Nano)
        if err = updateAutomationStateTx(tx, id, "claimed", "", conductor, "apply_request_resume"); err != nil {
                return zero, false, err
        }
        result, err := tx.Exec(`UPDATE automation_occurrences SET claimed_by=?,lease_until=? WHERE request_id=? AND (claimed_by='' OR claimed_by=? OR lease_until<=?)`, conductor, lease, id, conductor, at.UTC().Format(time.RFC3339Nano))
        if err != nil {
                return zero, false, err
        }
        if n, _ := result.RowsAffected(); n != 1 {
                return record, false, nil
        }
        if err = tx.Commit(); err != nil {
                return zero, false, err
        }
        got, err := s.automationRequest(id)
        return got, err == nil, err
}

func (s *Store) applyAutomationResumeEffect(id, conductor string) (string, error) {
        tx, err := s.db.Begin()
        if err != nil { return "", err }
        defer tx.Rollback()
        if _, err = tx.Exec("UPDATE automation_requests SET revision=revision WHERE request_id=?", id); err != nil { return "", err }
        record, err := scanAutomationRequest(tx.QueryRow(automationRequestSelect+" WHERE r.request_id=?", id))
        if err != nil { return "", err }
        effectID := "effect-" + hash([]byte(id+"|request_resume"))[:24]
        var existing string
        if err = tx.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&existing); err == nil {
                return existing, nil
        } else if !errors.Is(err, sql.ErrNoRows) { return "", err }
        if record.State != "claimed" || record.ClaimedBy != conductor {
                return "", &CommandError{Code: "stale_lease", Message: "bail de demande absent ou remplacé"}
        }
        until, parseErr := time.Parse(time.RFC3339Nano, record.LeaseUntil)
        if parseErr != nil || !time.Now().Before(until) {
                return "", &CommandError{Code: "stale_lease", Message: "bail de demande expiré"}
        }
        authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
        if err != nil { return "", err }
        if !authorized {
                if err = updateAutomationStateTx(tx, id, "rejected", code, conductor, "none"); err != nil { return "", err }
                if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", id); err != nil { return "", err }
                if err = tx.Commit(); err != nil { return "", err }
                return "", &CommandError{Code: code, Message: "autorisation ou condition retirée avant effet"}
        }
        if _, err = tx.Exec("INSERT INTO automation_effects(effect_id,request_id,target_work_id,action,created_at) VALUES(?,?,?,?,?)", effectID, id, record.TargetWorkID, record.Action, now()); err != nil { return "", err }
        message, _ := json.Marshal(map[string]string{"request_id": id, "effect_id": effectID, "action": record.Action, "source": record.Source})
        if _, err = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", record.TargetWorkID, now(), "automation-request", string(message)); err != nil { return "", err }
        if err = tx.Commit(); err != nil { return "", err }
        return effectID, nil
}

func (s *Store) markAutomationRequestUncertain(id, conductor, reason string) error {
        if strings.TrimSpace(reason) == "" || len([]byte(reason)) > 500 {
                return &CommandError{Code: "invalid_input", Message: "motif d’incertitude requis et limité à 500 octets"}
        }
        tx, err := s.db.Begin()
        if err != nil { return err }
        defer tx.Rollback()
        var state, owner string
        if err = tx.QueryRow(`SELECT r.state,o.claimed_by FROM automation_requests r
 JOIN automation_occurrences o ON o.request_id=r.request_id WHERE r.request_id=?`, id).Scan(&state, &owner); err != nil { return err }
        if state != "claimed" || owner != conductor {
                return &CommandError{Code: "stale_lease", Message: "seul le conducteur attribué peut déclarer un effet incertain"}
        }
        if err = updateAutomationStateTx(tx, id, "uncertain_effect", reason, conductor, "operator_reconciliation"); err != nil { return err }
        if _, err = tx.Exec("UPDATE automation_occurrences SET lease_until='' WHERE request_id=?", id); err != nil { return err }
        return tx.Commit()
}

func (s *Store) completeAutomationRequest(id, conductor, effectID string) error {
        tx, err := s.db.Begin()
        if err != nil {
                return err
        }
        defer tx.Rollback()
        var known string
        if err = tx.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&known); err != nil {
                return err
        }
        if effectID != known {
                return &CommandError{Code: "uncertain_effect", Message: "identité de l’effet non démontrée"}
        }
        if err = updateAutomationStateTx(tx, id, "executed", "", conductor, "none"); err != nil {
                return err
        }
        if _, err = tx.Exec("UPDATE automation_occurrences SET effect_id=?,lease_until='' WHERE request_id=?", effectID, id); err != nil {
                return err
        }
        return tx.Commit()
}

func (s *Store) processAutomationRequest(id, conductor string, at time.Time, config AutomationRequestConfig) (AutomationRequestRecord, error) {
        if record, err := s.automationRequest(id); err == nil && record.State != "executed" {
                var effectID string
                if lookupErr := s.db.QueryRow("SELECT effect_id FROM automation_effects WHERE request_id=?", id).Scan(&effectID); lookupErr == nil {
                        if err = s.completeAutomationRequest(id, conductor, effectID); err != nil { return AutomationRequestRecord{}, err }
                        return s.automationRequest(id)
                } else if !errors.Is(lookupErr, sql.ErrNoRows) { return AutomationRequestRecord{}, lookupErr }
        }
        record, claimed, err := s.claimAutomationRequest(id, conductor, at, config)
        if err != nil || !claimed { return record, err }
        effectID, err := s.applyAutomationResumeEffect(id, conductor)
        if err != nil {
                // An absent effect is retryable after lease expiry. If its presence cannot
                // be established, retain an explicit uncertain terminal state.
                var count int
                lookupErr := s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", id).Scan(&count)
                if lookupErr != nil {
                        tx, beginErr := s.db.Begin()
                        if beginErr == nil {
                                _ = updateAutomationStateTx(tx, id, "uncertain_effect", "effect_lookup_failed", conductor, "operator_reconciliation")
                                _ = tx.Commit()
                        }
                }
                return AutomationRequestRecord{}, err
        }
        if err = s.completeAutomationRequest(id, conductor, effectID); err != nil { return AutomationRequestRecord{}, err }
        return s.automationRequest(id)
}
```

Cette chaîne expose désormais sans renvoi implicite les trois gardes demandées :
`automationWorkspaceBusyTx` observe les workspaces du profil et des tâches sans
créer de tentative ; `updateAutomationStateTx` synchronise demande et occurrence
dans la transaction appelante ; `completeAutomationRequest` exige l'identité
d'effet déjà persistée avant de conclure. Leurs appelants `claim…`, `apply…`,
`mark…` et `process…` figurent dans le même bloc. Le dossier ne remplace pas le
fichier candidat ni son empreinte.

## Tests critiques complets

Annexe exécutable complète des scénarios critiques (les imports sont `errors`,
`sync`, `testing`, `time`) :

```go
func automationC01Fixture(t *testing.T) (*Store, Work) {
        t.Helper()
        s := storeTest(t)
        w, _ := setupAgent(t, s)
        organizedFixtureStore(t, s)
        w, _ = s.get(w.ID)
        var err error
        w, err = s.planningChange(w.ID, "resume", PlanningRequest{Schema: 1, EventID: newID("automation-resume-"), Revision: w.Revision, Scope: "root"})
        if err != nil { t.Fatal(err) }
        if err := s.setProfile(w.ID, "", LaunchProfile{Provider: "fixture", Role: "worker", Workspace: s.root}, w.Revision); err != nil { t.Fatal(err) }
        if err := s.setAutonomy(w.ID, autonomyAuto, 1); err != nil { t.Fatal(err) }
        if err := s.setMission(w.ID, true); err != nil { t.Fatal(err) }
        w, _ = s.get(w.ID)
        return s, w
}

func automationC01Request(key string, w Work) AutomationResumeRequest {
        r := AutomationResumeRequest{Schema: 1, IdempotencyKey: key, Source: "fixture locale", TargetWorkID: w.ID, Action: "request_resume"}
        r.ContentDigest = automationResumeDigest(r)
        return r
}

func TestAutomationC01Persistence(t *testing.T) {
        s, w := automationC01Fixture(t)
        created, fresh, err := s.submitAutomationResume(automationC01Request("persist-one", w))
        if err != nil || !fresh || created.RequestID == "" || created.OccurrenceID == "" || created.RequestID == created.OccurrenceID {
                t.Fatalf("demande non créée avec identités distinctes : %+v, fresh=%v, err=%v", created, fresh, err)
        }
        reopened, err := openStore(s.root, false)
        if err != nil { t.Fatal(err) }
        defer reopened.db.Close()
        got, err := reopened.automationRequest(created.RequestID)
        if err != nil || got.ContentDigest != created.ContentDigest || got.State != "received" || got.Revision != 1 {
                t.Fatalf("demande perdue au restart : %+v, %v", got, err)
        }
        var requests, occurrences int
        if err = reopened.db.QueryRow("SELECT count(*) FROM automation_requests WHERE request_id=?", created.RequestID).Scan(&requests); err != nil { t.Fatal(err) }
        if err = reopened.db.QueryRow("SELECT count(*) FROM automation_occurrences WHERE request_id=?", created.RequestID).Scan(&occurrences); err != nil { t.Fatal(err) }
        if requests != 1 || occurrences != 1 { t.Fatalf("autorité durable incomplète : demandes=%d occurrences=%d", requests, occurrences) }
        legacyRoot := t.TempDir()
        legacy, err := openStore(legacyRoot, true)
        if err != nil { t.Fatal(err) }
        if _, err = legacy.db.Exec(`DROP TABLE automation_effects; DROP TABLE automation_occurrences; DROP TABLE automation_requests; PRAGMA user_version=24`); err != nil { t.Fatal(err) }
        if err = legacy.db.Close(); err != nil { t.Fatal(err) }
        migrated, err := openStore(legacyRoot, false)
        if err != nil { t.Fatal(err) }
        defer migrated.db.Close()
        var version int
        if err = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion { t.Fatalf("migration v24 incompatible : version=%d err=%v", version, err) }
        if err = migrated.db.QueryRow("SELECT count(*) FROM automation_requests").Scan(&requests); err != nil || requests != 0 { t.Fatalf("table C01 absente après migration : %d, %v", requests, err) }
}

func TestAutomationC01Idempotency(t *testing.T) {
        s, w := automationC01Fixture(t)
        r := automationC01Request("same-key", w)
        first, fresh, err := s.submitAutomationResume(r)
        if err != nil || !fresh { t.Fatal(fresh, err) }
        again, fresh, err := s.submitAutomationResume(r)
        if err != nil || fresh || again.RequestID != first.RequestID || again.Revision != first.Revision { t.Fatalf("rejeu identique non idempotent : %+v fresh=%v err=%v", again, fresh, err) }
        r.Source = "contenu différent"
        r.ContentDigest = automationResumeDigest(r)
        if _, _, err = s.submitAutomationResume(r); err == nil {
                t.Fatal("conflit de contenu accepté")
        } else {
                var command *CommandError
                if !errors.As(err, &command) || command.Code != "idempotency_conflict" { t.Fatalf("mauvais conflit : %v", err) }
        }
        var count int
        if err = s.db.QueryRow("SELECT count(*) FROM automation_requests WHERE idempotency_key=?", r.IdempotencyKey).Scan(&count); err != nil || count != 1 { t.Fatalf("conflit a dupliqué la demande : %d, %v", count, err) }
}

func TestAutomationC01ClaimConcurrency(t *testing.T) {
        s, w := automationC01Fixture(t)
        record, _, err := s.submitAutomationResume(automationC01Request("claim-race", w))
        if err != nil { t.Fatal(err) }
        other, err := openStore(s.root, false)
        if err != nil { t.Fatal(err) }
        defer other.db.Close()
        start := make(chan struct{})
        results := make(chan bool, 2)
        errorsOut := make(chan error, 2)
        var group sync.WaitGroup
        for i, candidate := range []*Store{s, other} {
                group.Add(1)
                go func(index int, store *Store) {
                        defer group.Done(); <-start
                        _, claimed, claimErr := store.claimAutomationRequest(record.RequestID, []string{"driver-one", "driver-two"}[index], time.Now(), defaultAutomationRequestConfig())
                        results <- claimed; errorsOut <- claimErr
                }(i, candidate)
        }
        close(start); group.Wait(); close(results); close(errorsOut)
        claims := 0
        for claimed := range results { if claimed { claims++ } }
        for claimErr := range errorsOut { if claimErr != nil { t.Fatal(claimErr) } }
        if claims != 1 { t.Fatalf("attribution non exclusive : %d claims", claims) }
        got, err := s.automationRequest(record.RequestID)
        if err != nil || got.State != "claimed" || got.ClaimedBy == "" || got.LeaseUntil == "" { t.Fatalf("bail non durable : %+v, %v", got, err) }
}

func TestAutomationC01CrashRecovery(t *testing.T) {
        s, w := automationC01Fixture(t)
        record, _, err := s.submitAutomationResume(automationC01Request("crash-recovery", w))
        if err != nil { t.Fatal(err) }
        claimed, owned, err := s.claimAutomationRequest(record.RequestID, "crashed-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || !owned || claimed.State != "claimed" { t.Fatal(claimed, owned, err) }
        effectID, err := s.applyAutomationResumeEffect(record.RequestID, "crashed-driver")
        if err != nil || effectID == "" { t.Fatal(effectID, err) }
        reopened, err := openStore(s.root, false)
        if err != nil { t.Fatal(err) }
        defer reopened.db.Close()
        completed, err := reopened.processAutomationRequest(record.RequestID, "recovery-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || completed.State != "executed" || completed.EffectID != effectID { t.Fatalf("effet certain non réconcilié : %+v, %v", completed, err) }
        var effects, signals int
        _ = reopened.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
        _ = reopened.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='automation-request'", w.ID).Scan(&signals)
        if effects != 1 || signals != 1 { t.Fatalf("effet répété après restart : effects=%d signals=%d", effects, signals) }
        uncertain, _, err := reopened.submitAutomationResume(automationC01Request("uncertain-recovery", w))
        if err != nil { t.Fatal(err) }
        if _, owned, claimErr := reopened.claimAutomationRequest(uncertain.RequestID, "uncertain-driver", time.Now(), defaultAutomationRequestConfig()); claimErr != nil || !owned { t.Fatal(owned, claimErr) }
        if err = reopened.markAutomationRequestUncertain(uncertain.RequestID, "uncertain-driver", "confirmation de l’effet indisponible après arrêt"); err != nil { t.Fatal(err) }
        unchanged, err := reopened.processAutomationRequest(uncertain.RequestID, "another-driver", time.Now().Add(time.Minute), defaultAutomationRequestConfig())
        if err != nil || unchanged.State != "uncertain_effect" || unchanged.NextAction != "operator_reconciliation" { t.Fatalf("effet incertain rejoué ou masqué : %+v, %v", unchanged, err) }
        _ = reopened.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", uncertain.RequestID).Scan(&effects)
        if effects != 0 { t.Fatalf("effet incertain rejoué : %d", effects) }
}

func TestAutomationC01Authorization(t *testing.T) {
        s, w := automationC01Fixture(t)
        record, _, err := s.submitAutomationResume(automationC01Request("revoked-auth", w))
        if err != nil { t.Fatal(err) }
        if err = s.stopMission(w.ID); err != nil { t.Fatal(err) }
        got, err := s.processAutomationRequest(record.RequestID, "auth-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || got.State != "rejected" || got.Reason != "authorization_required" { t.Fatalf("autorisation non revalidée : %+v, %v", got, err) }
        var agents, effects int
        _ = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents)
        _ = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
        if agents != 0 || effects != 0 { t.Fatalf("rejet a produit un effet : agents=%d effects=%d", agents, effects) }
        racingStore, racingWork := automationC01Fixture(t)
        racing, _, err := racingStore.submitAutomationResume(automationC01Request("revoked-after-claim", racingWork))
        if err != nil { t.Fatal(err) }
        if _, owned, claimErr := racingStore.claimAutomationRequest(racing.RequestID, "effect-driver", time.Now(), defaultAutomationRequestConfig()); claimErr != nil || !owned { t.Fatal(owned, claimErr) }
        if err = racingStore.stopMission(racingWork.ID); err != nil { t.Fatal(err) }
        if _, err = racingStore.applyAutomationResumeEffect(racing.RequestID, "effect-driver"); err == nil { t.Fatal("autorisation retirée après claim non revalidée avant effet") }
        racing, _ = racingStore.automationRequest(racing.RequestID)
        if racing.State != "rejected" || racing.Reason != "authorization_required" { t.Fatalf("révocation intercalée mal classée : %+v", racing) }
        _ = racingStore.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", racing.RequestID).Scan(&effects)
        if effects != 0 { t.Fatalf("révocation intercalée a produit %d effet(s)", effects) }
        budgetStore, budgetWork := automationC01Fixture(t)
        budgetRequest, _, err := budgetStore.submitAutomationResume(automationC01Request("budget-recheck", budgetWork))
        if err != nil { t.Fatal(err) }
        budgetWork, err = budgetStore.mutate(budgetWork.ID, "automation.test-budget", "exhaust-budget-c01", budgetWork.Revision, []byte(`{"budget":"exhausted"}`), func(candidate *Work) error {
                candidate.Planning.Activations = candidate.Planning.MaxActivations
                return nil
        })
        if err != nil { t.Fatal(err) }
        budgetResult, err := budgetStore.processAutomationRequest(budgetRequest.RequestID, "budget-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || budgetResult.State != "rejected" || budgetResult.Reason != "budget_or_planning_blocked" { t.Fatalf("plafond non revalidé : %+v, %v", budgetResult, err) }
        _ = budgetStore.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", budgetRequest.RequestID).Scan(&effects)
        if effects != 0 { t.Fatalf("plafond épuisé a produit %d effet(s)", effects) }
        terminalStore, terminalWork := automationC01Fixture(t)
        terminalWork, _ = terminalStore.get(terminalWork.ID)
        raw := []byte(`{"close":"root"}`)
        terminalWork, err = terminalStore.mutate(terminalWork.ID, "automation.test-close", "close-for-c01", terminalWork.Revision, raw, func(candidate *Work) error {
                root, rootErr := candidate.Planning.scope("root"); if rootErr == nil { root.State = "closed" }; return rootErr
        })
        if err != nil { t.Fatal(err) }
        closed, fresh, err := terminalStore.submitAutomationResume(automationC01Request("closed-target", terminalWork))
        if err != nil || !fresh || closed.State != "rejected" || closed.Reason != "terminal_target" { t.Fatalf("cible clôturée non refusée durablement : %+v fresh=%v err=%v", closed, fresh, err) }
        _ = terminalStore.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", terminalWork.ID).Scan(&agents)
        if agents != 0 { t.Fatalf("cible clôturée a créé %d tentative(s)", agents) }
}

func TestAutomationC01WorkspaceWait(t *testing.T) {
        s, w := automationC01Fixture(t)
        holder, created, err := s.prepare(w.ID, Launch{Schema: 1, EventID: "workspace-holder", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: s.root, Capture: true})
        if err != nil || !created { t.Fatalf("fixture d’occupation absente : %+v created=%v err=%v", holder, created, err) }
        before, _ := s.get(w.ID)
        var reservationsBefore int
        _ = s.db.QueryRow("SELECT count(*) FROM reservations WHERE work_id=?", w.ID).Scan(&reservationsBefore)
        record, _, err := s.submitAutomationResume(automationC01Request("workspace-wait", before))
        if err != nil { t.Fatal(err) }
        got, err := s.processAutomationRequest(record.RequestID, "waiting-driver", time.Now(), defaultAutomationRequestConfig())
        if err != nil || got.State != "waiting" || got.Reason != "workspace_wait" || got.NextAction != "retry_after_resource_release" { t.Fatalf("attente d’espace incorrecte : %+v, %v", got, err) }
        after, _ := s.get(w.ID)
        if len(after.Tasks[0].Attempts) != len(before.Tasks[0].Attempts) { t.Fatalf("attente a consommé une tentative : avant=%d après=%d", len(before.Tasks[0].Attempts), len(after.Tasks[0].Attempts)) }
        var effects int
        _ = s.db.QueryRow("SELECT count(*) FROM automation_effects WHERE request_id=?", record.RequestID).Scan(&effects)
        var reservationsAfter int
        _ = s.db.QueryRow("SELECT count(*) FROM reservations WHERE work_id=?", w.ID).Scan(&reservationsAfter)
        if effects != 0 || reservationsAfter != reservationsBefore { t.Fatalf("attente a produit un effet ou altéré la consommation : effects=%d réservations=%d→%d", effects, reservationsBefore, reservationsAfter) }
}
```

Aucun test n'est skippé et la fixture `fixture` est le sous-processus de test
local existant, jamais un fournisseur payant. La revue doit comparer le SHA-256
du fichier exécutable au candidat ; la condensation de certaines lignes dans
cette annexe ne change aucune assertion.

## Raccord explicite à req-1 et recette de revue indépendante

Candidate à examiner : HEAD de base
`cc3069dc7bb61b90168d21f945cb2eb5e27578ed`, avec diff sale. L'identité du
comportement C01 est fixée par les empreintes SHA-256 suivantes :

| Entrée candidate | SHA-256 | Rôle dans req-1 |
| --- | --- | --- |
| `automation_requests.go` | `0f69ed148778dab75c0f9148eccb22da741b89e332864e3ed40edb3c42633cbd` | service, tables, transactions et gardes |
| `automation_requests_test.go` | `4262ebf343e461833724ba82f9521eac569784ad292ec26e07db87912c5f2e9b` | six scénarios isolés obligatoires |
| `model.go` | `31e5d2214dded33b5c20381d1fcc1d94d07779f9c6d9cd7636d39a500dfd0125` | version globale du schéma ; fichier partagé |
| `store.go` | `91d81e3d82ea22eca08e044d7efa187d87107e47a53450f4c1ff816dd4af1b6d` | appel de migration v25 ; fichier partagé |
| `lifecycle.go` | `0a4e7c9e2971c988d58bdc5a16494aa87bd05235adf159b3a125f1bde8038786` | snapshot/restauration/suppression ; fichier partagé |
| `docs/AUTOMATION.md` | `852d6e6762e6b55fc73fa2522d612d3c41d3e47398bc558a7981163a6436edca` | guide FR |
| `docs/en/AUTOMATION.md` | `042b4e61c180cc4a52c3c6f7b21b73e4e14d1b4da84ee9f654262b7699a8a8ca` | guide EN |

Les fichiers partagés portent aussi des changements d'autres lots : leur
empreinte sert à détecter toute mutation entre contrôle et revue, sans attribuer
leur diff entier à C01. Les empreintes du service, des tests et des guides sont
identiques à celles consignées lors des tests locaux historiques ; cette tentative
n'a modifié que `docs/C01.md` et ce dossier.

Revue indépendante requise sur ces entrées exactes, transactions et gardes complètes ci-dessus. Les contrôles hôte doivent précéder le verdict.

| Sous-critère req-1 | Code/garde à suivre | Assertion discriminante |
| --- | --- | --- |
| demandes/occurrences durables | migration, `submitAutomationResume`, lecteur joint | restart et migration v24→v25, une ligne de chaque identité |
| clé/contenu idempotents, conflit sûr | digest canonique + branche clé existante | même ID/révision ; contenu différent refusé ; count=1 |
| attribution exclusive | transaction de claim + bail conditionnel | deux connexions, exactement un claim |
| reprise sans rejeu aveugle | table d'effets + `process…` + `complete…` + état incertain | même effet réconcilié une fois ; incertain non rejoué |
| autorisation et clôture revalidées | `automationPolicyTx` à la réception, au claim et avant effet | révocation/plafond/clôture : rejet et zéro effet/agent |
| espace occupé sans tentative | `automationWorkspaceBusyTx` avant attribution | attente durable ; attempts/réservations/effets inchangés |
| vraie revue indépendante | recette ci-dessus sur les mêmes empreintes | encore requise ; ce dossier n'est pas un verdict |

## Raccords exacts de migration et cycle de vie

`model.go` :

```go
const schemaVersion = 25
```

`store.go` :

```go
        if version < 25 {
                if version != 0 {
                        if _, e = db.Exec("VACUUM INTO ?", filepath.Join(dir, newID("state-pre-v25-")+".db")); e != nil {
                                return fail(e)
                        }
                }
                if _, e = db.Exec(automationRequestMigration); e != nil {
                        return fail(e)
                }
        }
```

Début de la liste transactionnelle `lifecycleSpecs` ; les tables existantes suivent :

```go
        return []lifecycleSpec{
                {"works", "id=?", []any{work}},
                {"automation_requests", "target_work_id=?", []any{work}},
                {"automation_occurrences", "request_id IN (SELECT request_id FROM automation_requests WHERE target_work_id=?)", []any{work}},
                {"automation_effects", "target_work_id=?", []any{work}},
```

Suppression en ordre inverse dans `applyDelete` :

```go
        for i := len(specs) - 1; i >= 0; i-- {
                spec := specs[i]
                if _, err = tx.Exec("DELETE FROM "+spec.name+" WHERE "+spec.where, spec.args...); err != nil {
                        return fmt.Errorf("suppression atomique de %s : %w", spec.name, err)
                }
        }
```

Restauration transactionnelle dans `applyRestore` :

```go
        for _, table := range snap.Tables {
                if err := restoreTable(tx, table); err != nil {
                        return fmt.Errorf("restauration atomique de %s : %w", table.Name, err)
                }
        }
```

`restoreTable` conserve sa liste blanche existante, étendue uniquement à `automation_requests`, `automation_occurrences`, `automation_effects`. Le fichier entier est lié aux contrôles et au verdict par empreinte publique.

## Guide fourni intégralement : docs/AUTOMATION.md

# Automatisation durable

## Service disponible dans le moteur

Le moteur possède un service interne de demandes de reprise durables. Ce lot ne
livre pas encore de commande CLI, de route HTTP, de programmation horaire ni de
connecteur externe : ces surfaces appartiennent aux lots suivants.

Une demande C01 contient :

- une clé d'idempotence stable choisie par l'appelant ;
- une source bornée, une mission cible existante et l'action unique
  `request_resume` ;
- un digest SHA-256 du contenu canonique ;
- une identité de demande et une identité d'occurrence distinctes.

Les tables transactionnelles du `Store` sont l'unique autorité. Un rejeu avec la
même clé et le même contenu retourne l'enregistrement existant ; la même clé avec
un contenu différent retourne `idempotency_conflict` sans seconde occurrence.

## Autorisation, attribution et attente

La réception exige une autorisation locale explicite de la mission. La prise en
charge la revalide, ainsi que l'autonomie, la pause, les plafonds de planification
et la clôture de la cible. Une mission clôturée est enregistrée comme rejetée avec
`terminal_target` ; aucune mission ni tentative n'est créée.

Une occurrence est attribuée par bail à un seul conducteur. Le réglage versionné
actuel est `version: 1`, avec `claim_lease_seconds: 30` par défaut et une plage
admise de 5 à 300 secondes. Une occupation de l'espace produit l'état `waiting`,
la cause `workspace_wait`, l'acteur et l'action suivante
`retry_after_resource_release`, sans consommer de tentative.

## Reprise après arrêt

L'effet `request_resume` est un signal durable du moteur, identifié de façon
stable à partir de la demande et enregistré dans la même base que la mission. Si
le processus s'arrête après l'effet mais avant la clôture de l'occurrence, le
redémarrage retrouve cette identité et termine la même demande sans répéter
l'effet. Si la présence ou l'absence de l'effet ne peut pas être démontrée, le
service conserve `uncertain_effect` et demande une réconciliation opérateur ; il
ne promet pas une exécution exactement une fois chez un fournisseur.

L'état `executed` signifie que la demande a été traitée. Il ne signifie ni qu'une
mission est terminée, ni que son résultat est accepté.

## Guide fourni intégralement : docs/en/AUTOMATION.md

# Durable automation

## Service currently available in the engine

The engine has an internal service for durable resume requests. This increment
does not yet provide CLI commands, HTTP routes, time schedules, or external
connectors; those surfaces belong to later increments.

A C01 request contains:

- a stable idempotency key selected by the caller;
- a bounded source, an existing target mission, and the sole
  `request_resume` action;
- a SHA-256 digest of the canonical content;
- distinct request and occurrence identities.

Transactional `Store` tables are the single authority. Replaying the same key
and content returns the existing record. Reusing the key with different content
returns `idempotency_conflict` and creates no second occurrence.

## Authorization, claiming, and waiting

Reception requires explicit local mission authorization. Claiming revalidates
that authorization, autonomy, pause state, planning ceilings, and whether the
target is closed. A closed mission is durably rejected with `terminal_target`;
no mission or attempt is created.

An occurrence is claimed by one conductor under a lease. The current versioned
setting is `version: 1`, with a default `claim_lease_seconds: 30` and an allowed
range of 5 to 300 seconds. An occupied workspace leaves the occurrence in
`waiting` with reason `workspace_wait`, the actor, and next action
`retry_after_resource_release`, without consuming an attempt.

## Restart recovery

The `request_resume` effect is a durable engine signal with an identity derived
from the request and stored in the mission database. If the process stops after
the effect but before occurrence completion, restart finds that identity and
completes the same request without applying the effect again. If effect presence
or absence cannot be established, the service retains `uncertain_effect` and
requires operator reconciliation; it does not promise exactly-once execution at
an external provider.

The `executed` state means that the request was processed. It does not mean the
mission is complete or its result has been accepted.
