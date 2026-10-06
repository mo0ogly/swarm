# Dossier D03 — qualification finale du candidat

## Portée et identité gelée

- Exigence : `req-3`, qualification D03 seulement.
- Base Git : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`, branche `codex/clear-launch-recovery`, worktree sale.
- Manifeste : `docs/graph-delivery-qualification-manifest.json`.
- Identité : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed+sha256:f9cf4c3678f72b9c1a8e8654bb253818a74cb72b552dc1d256f73f6b616b95c8`.
- Couverture : SHA-256 `40ecbfee659a1910df25ec5ef97036e86f99d6c95185d3ef996be979b9b4e66a` sur les lignes triées R01–R18.
- Périmètre gelé : 62 entrées explicites, soit 16 sources, 5 tests, 21 documents, 4 éléments de harness et 16 captures. Le commit de base fixe les fichiers suivis inchangés ; le digest fixe les entrées sales nécessaires à la livraison graphe/automatisation D.
- Exclusions volontaires : ce dossier et `docs/D03.md`, qui évoluent avec les résultats ; résultats historiques volumineux sous `execution/**/results`; état `.swarm`; binaire installé et fournisseur réel. Ces exclusions ne peuvent servir de preuve fonctionnelle.

Ce gel ne vaut ni gate moteur, ni suite complète, ni revue indépendante. Toute modification d’une des 62 entrées rend le test D03 rouge. Une modification hors liste exige une analyse d’impact ; si elle affecte la livraison, elle doit être ajoutée et le candidat doit être regélé avant les contrôles hôte et la revue.

## Contrat du manifeste

`graph_delivery_qualification_test.go` charge le JSON depuis la racine du paquet et vérifie :

1. schéma, statut et base Git formellement valides ;
2. exactement R01 à R18, sans doublon, chacune avec état et preuves ;
3. digest de couverture recalculé depuis ID, état, preuves triées et limite ;
4. chemins relatifs sûrs, uniques et lisibles ;
5. présence des cinq classes `source`, `test`, `doc`, `capture`, `harness` ;
6. SHA-256 réel de chaque fichier, digest candidat agrégé et `candidate_id` dérivé ;
7. présence explicite des tests D01/D02/D03, recettes Node/Python, rapports/dossiers D01/D02, harness hôte et captures D01/D02 ;
8. refus discriminant d’une entrée absente, d’une empreinte périmée et d’une couverture amputée.

Algorithme candidat : SHA-256 de la concaténation triée des lignes `sha256␠␠path\n`. Algorithme couverture : SHA-256 des lignes triées `ID|status|evidence triées|limit\n`. Le manifeste ne se hache pas lui-même afin d’éviter une identité circulaire ; sa valeur est contrôlée par les deux digests et le test versionné.

## Raccords fonctionnels

| Surface | Sources / tests gelés | Preuves liées | Limite conservée |
| --- | --- | --- | --- |
| Brouillon de graphe | `graph_draft*.go`, `web/graph-draft.js`, `web/graph.js`, `graph_delivery_e2e_test.go` | `docs/B01.md` à `docs/B04.md`, `docs/D01.md` | fixtures isolées ; aucun fournisseur payant |
| Programmes | `automation_*.go`, `web/automation.js`, `graph_delivery_e2e_test.go` | `docs/C01.md` à `docs/C04.md`, `docs/D01.md` | évènements et ordonnanceur vérifiés sur fixtures |
| Migration / archive / rollback | `store.go`, `graph_delivery_migration_test.go`, `tests/graph_delivery_docs.py` | `docs/D02.md`, `docs/D02-dossier.md` | racines temporaires ; installation finale relève de D04 |
| Produit navigateur | `tests/graph_automation_e2e.cjs`, 16 PNG D01/D02 | compléments hôte D01/D02 | captures et assertions DOM ; pas d’autonomie fournisseur réelle |
| Qualification | `graph_delivery_qualification_test.go`, manifeste | test D03 ciblé puis `verify.py D03` | suite complète uniquement hôte ; revue séparée obligatoire |

## Couverture R01–R18 gelée

Les statuts du manifeste sont volontairement `COVERED_PENDING_FINAL_CONTROL_AND_REVIEW` : ils signifient que des preuves sont liées, pas que D03 est acceptée.

| Exigence | Preuves principales | Lacune qui interdit un verdict final worker |
| --- | --- | --- |
| R01 | B03, D01 | préparation sans fournisseur payant ; contrôle final/revue en attente |
| R02–R03 | B01, D01 | contrôle final/revue en attente |
| R04–R05 | B03, D01 | preuve navigateur fondée sur fixture |
| R06 | B04, D01 | contrôle final/revue en attente |
| R07 | B02, B03 | contrôle final/revue en attente |
| R08 | B02, D01 | contrôle final/revue en attente |
| R09–R10 | C01, D01 | pas d’autonomie fournisseur payante revendiquée |
| R11 | C02, D01 | contrôle final/revue en attente |
| R12 | C03, D01 | évènements externes locaux ; aucun fournisseur réel |
| R13 | C02, D01 | contrôle final/revue en attente |
| R14 | B04, D01 | rattachement au candidat à vérifier après contrôle hôte |
| R15 | B03, C04, D01 | quatre variantes fixture ; revue en attente |
| R16 | D02 et dossier | candidat installé final relève de D04 |
| R17 | C01, D01 | le manifeste n’affirme aucune acceptation moteur |
| R18 | B04, D01 | coût et autonomie fournisseur réels inconnus |

## Recette worker autorisée

Le worker D03 peut démontrer la structure et la sensibilité du gel sans lancer la suite globale :

```sh
go test . -run '^TestGraphDeliveryD03QualificationManifest$' -count=1 -json -timeout=120s
go test -race . -run '^TestGraphDeliveryD03QualificationManifest$' -count=1 -json -timeout=120s
python3 -m json.tool docs/graph-delivery-qualification-manifest.json
python3 docs/plans/graphe-automatisation-20261005/execution/d/verify.py --self-test
python3 tools/agent-workflows/check.py
git diff --check
```

Le self-test du harness démontre seulement le rejet des observations manquantes, ignorées, échouées ou dupliquées et l’intégrité des rapports protégés. `check.py` et `git diff --check` sont statiques. Aucun de ces contrôles ne remplace le parcours hôte ni une revue.

## Recette hôte réservée

Une fois le candidat stabilisé, le conducteur hôte exécute exactement une fois :

```sh
python3 docs/plans/graphe-automatisation-20261005/execution/d/verify.py D03
```

Le harness inventorie le seul test `TestGraphDeliveryD03QualificationManifest`, l’exécute sous race, puis lance `go vet ./...`, `npm test`, `python3 tools/agent-workflows/check.py`, et `python3 tests/supervision_go_suite.py`. Cette dernière compilation est unique et la découverte complète est partitionnée sans doublon ; les skips sont listés et les skips de fonctions requises sont refusés. Enfin, `git diff --check` et les empreintes des rapports A/B/C protégés sont revalidés.

Le reçu hôte doit contenir commandes, codes de sortie, durées, inventaire découvert, nombre de shards, skips explicites, logs et empreintes. Un simple mot `PASS`, une compilation ou l’existence du rapport ne suffit pas. Toute mutation pertinente après le contrôle invalide le reçu et requiert une analyse d’impact avant toute nouvelle suite globale.

## Paquet pour revue indépendante

Le reviewer distinct doit recevoir, sur le même candidat et après le vrai contrôle hôte :

- le présent dossier, `docs/D03.md`, le manifeste et le test D03 ;
- le HEAD, l’état sale et le digest candidat ;
- le reçu complet `verify.py D03`, y compris inventaire, logs, skips et diagnostics ;
- les rapports/dossiers D01 et D02, les compléments hôte et les reçus qui y sont référencés ;
- les limites fixture/fournisseur, D04/install et acceptation moteur.

La revue doit vérifier au minimum les omissions du manifeste, la fidélité de R01–R18, la fraîcheur du reçu, l’absence de skip requis masqué et l’absence d’assimilation entre tests locaux, autonomie réelle et acceptation. Le producteur D03 ne peut ni autosigner cette revue ni déclarer la gate acceptée.

## État au moment de la remise

- Test D03 normal : PASS ciblé, code 0, une exécution, cas positif et trois négatifs.
- Race D03 ciblée : PASS, code 0, test exécuté sans skip.
- JSON, syntaxe des quatre JS gelés, self-test harness, configuration et diff : PASS, codes 0. Le self-test a aussi revalidé les empreintes protégées A/B/C.
- Revues historiques : D01 possède un avis indépendant `passed` et une acceptation moteur ; D02 possède un avis initial `changes_requested`, puis un avis indépendant `passed` après captures fraîches. Elles ne remplacent pas une revue D03 postérieure au reçu final.
- Suite complète hôte, vet/npm du protocole final et revue indépendante D03 : `NOT TESTED` par ce worker jusqu’à réception de preuves externes authentifiables.
- Aucun SQLite vivant, serveur, port 18792, agent, mission, fournisseur, commit, push, release ou plafond n’a été touché.

## Complément de supervision avant qualification

Le manifeste est regélé avant contrôle : 68 entrées, identité `cc3069dc7bb61b90168d21f945cb2eb5e27578ed+sha256:d07015b6d564ef664a7cb21488bcfbac4361ae260568a06b7dd4b43710e38df7`. Les nombres et digests worker ci-dessus sont historiques, remplacés pour ce candidat par ceux-ci. Un wrapper distinct `execution/d/verify_d03_final.py` remplace la commande initiale du contrôle public (plafond300s, enveloppe globale295s). Il utilise le harness inchangé D03, vérifie la preuve navigateur D02 et ses entrées produit, conserve les logs/empreintes, vérifie compilation unique/inventaire/shards disjoints/complets et accounting passed/skips, et interdit les skips requis D/automation. Aucun résultat global n’est encore revendiqué. Les rapports D01/D02 restent inchangés.

### Source intégrale : graph_delivery_qualification_test.go

```
//go:build linux

package main

import (
        "crypto/sha256"
        "encoding/hex"
        "encoding/json"
        "errors"
        "fmt"
        "os"
        "path/filepath"
        "sort"
        "strings"
        "testing"
)

const graphDeliveryD03Manifest = "docs/graph-delivery-qualification-manifest.json"

type graphDeliveryD03Input struct {
        Path   string `json:"path"`
        Kind   string `json:"kind"`
        SHA256 string `json:"sha256"`
}

type graphDeliveryD03Requirement struct {
        ID       string   `json:"id"`
        Status   string   `json:"status"`
        Evidence []string `json:"evidence"`
        Limit    string   `json:"limit,omitempty"`
}

type graphDeliveryD03ManifestValue struct {
        SchemaVersion   int                           `json:"schema_version"`
        ManifestStatus  string                        `json:"manifest_status"`
        BaseCommit      string                        `json:"base_commit"`
        CandidateID     string                        `json:"candidate_id"`
        CandidateDigest string                        `json:"candidate_digest"`
        CoverageDigest  string                        `json:"coverage_digest"`
        Requirements    []graphDeliveryD03Requirement `json:"requirements"`
        Inputs          []graphDeliveryD03Input       `json:"inputs"`
}

func graphDeliveryD03Hash(raw []byte) string {
        digest := sha256.Sum256(raw)
        return hex.EncodeToString(digest[:])
}

func graphDeliveryD03CandidateDigest(inputs []graphDeliveryD03Input) string {
        lines := make([]string, 0, len(inputs))
        for _, input := range inputs {
                lines = append(lines, input.SHA256+"  "+input.Path+"\n")
        }
        sort.Strings(lines)
        return graphDeliveryD03Hash([]byte(strings.Join(lines, "")))
}

func graphDeliveryD03CoverageDigest(requirements []graphDeliveryD03Requirement) string {
        lines := make([]string, 0, len(requirements))
        for _, requirement := range requirements {
                evidence := append([]string(nil), requirement.Evidence...)
                sort.Strings(evidence)
                lines = append(lines, requirement.ID+"|"+requirement.Status+"|"+strings.Join(evidence, ",")+"|"+requirement.Limit+"\n")
        }
        sort.Strings(lines)
        return graphDeliveryD03Hash([]byte(strings.Join(lines, "")))
}

func graphDeliveryD03Validate(root string, manifest graphDeliveryD03ManifestValue) error {
        if manifest.SchemaVersion != 1 || manifest.ManifestStatus == "" || len(manifest.BaseCommit) != 40 {
                return errors.New("invalid qualification identity")
        }
        wantedRequirements := make(map[string]bool, 18)
        for i := 1; i <= 18; i++ {
                wantedRequirements[fmt.Sprintf("R%02d", i)] = true
        }
        seenRequirements := make(map[string]bool, 18)
        for _, requirement := range manifest.Requirements {
                if !wantedRequirements[requirement.ID] || seenRequirements[requirement.ID] || requirement.Status == "" || len(requirement.Evidence) == 0 {
                        return fmt.Errorf("invalid requirement coverage: %s", requirement.ID)
                }
                seenRequirements[requirement.ID] = true
        }
        if len(seenRequirements) != len(wantedRequirements) || graphDeliveryD03CoverageDigest(manifest.Requirements) != manifest.CoverageDigest {
                return errors.New("incomplete or stale requirement coverage")
        }

        allowedKinds := map[string]bool{"source": true, "test": true, "doc": true, "capture": true, "harness": true}
        seenKinds := make(map[string]bool)
        seenPaths := make(map[string]bool, len(manifest.Inputs))
        for _, input := range manifest.Inputs {
                clean := filepath.Clean(filepath.FromSlash(input.Path))
                if input.Path == "" || filepath.IsAbs(clean) || clean == "." || clean == ".." || strings.HasPrefix(clean, ".."+string(filepath.Separator)) {
                        return fmt.Errorf("unsafe input path: %q", input.Path)
                }
                if !allowedKinds[input.Kind] || seenPaths[input.Path] {
                        return fmt.Errorf("invalid or duplicate input: %s", input.Path)
                }
                raw, err := os.ReadFile(filepath.Join(root, clean))
                if err != nil {
                        return fmt.Errorf("missing input %s: %w", input.Path, err)
                }
                if graphDeliveryD03Hash(raw) != input.SHA256 {
                        return fmt.Errorf("stale input: %s", input.Path)
                }
                seenPaths[input.Path] = true
                seenKinds[input.Kind] = true
        }
        for _, kind := range []string{"source", "test", "doc", "capture", "harness"} {
                if !seenKinds[kind] {
                        return fmt.Errorf("missing input kind: %s", kind)
                }
        }
        for _, required := range []string{
                "graph_delivery_e2e_test.go",
                "graph_delivery_migration_test.go",
                "graph_delivery_qualification_test.go",
                "tests/graph_automation_e2e.cjs",
                "tests/graph_delivery_docs.py",
                "docs/D01.md",
                "docs/D01-dossier.md",
                "docs/D02.md",
                "docs/D02-dossier.md",
                "docs/plans/graphe-automatisation-20261005/execution/d/verify.py",
                "docs/screenshots/graph-delivery-d01/fr-etat-graph.png",
                "docs/screenshots/graph-delivery-d02/fr-etat-graph.png",
        } {
                if !seenPaths[required] {
                        return fmt.Errorf("required candidate input missing: %s", required)
                }
        }
        if graphDeliveryD03CandidateDigest(manifest.Inputs) != manifest.CandidateDigest {
                return errors.New("candidate digest mismatch")
        }
        if manifest.CandidateID != manifest.BaseCommit+"+sha256:"+manifest.CandidateDigest {
                return errors.New("candidate id mismatch")
        }
        return nil
}

func graphDeliveryD03Load(t *testing.T) (string, graphDeliveryD03ManifestValue) {
        t.Helper()
        root, err := os.Getwd()
        if err != nil {
                t.Fatal(err)
        }
        raw, err := os.ReadFile(filepath.Join(root, graphDeliveryD03Manifest))
        if err != nil {
                t.Fatal(err)
        }
        var manifest graphDeliveryD03ManifestValue
        if err = json.Unmarshal(raw, &manifest); err != nil {
                t.Fatal(err)
        }
        return root, manifest
}

func TestGraphDeliveryD03QualificationManifest(t *testing.T) {
        root, manifest := graphDeliveryD03Load(t)
        if err := graphDeliveryD03Validate(root, manifest); err != nil {
                t.Fatal(err)
        }

        missing := manifest
        missing.Inputs = append([]graphDeliveryD03Input(nil), manifest.Inputs...)
        missing.Inputs[0].Path = "missing-d03-candidate-input"
        if err := graphDeliveryD03Validate(root, missing); err == nil || !strings.Contains(err.Error(), "missing input") {
                t.Fatalf("missing candidate input was not rejected: %v", err)
        }

        stale := manifest
        stale.Inputs = append([]graphDeliveryD03Input(nil), manifest.Inputs...)
        stale.Inputs[0].SHA256 = strings.Repeat("0", 64)
        if err := graphDeliveryD03Validate(root, stale); err == nil || !strings.Contains(err.Error(), "stale input") {
                t.Fatalf("stale candidate input was not rejected: %v", err)
        }

        incomplete := manifest
        incomplete.Requirements = append([]graphDeliveryD03Requirement(nil), manifest.Requirements[:len(manifest.Requirements)-1]...)
        if err := graphDeliveryD03Validate(root, incomplete); err == nil || !strings.Contains(err.Error(), "incomplete") {
                t.Fatalf("incomplete coverage was not rejected: %v", err)
        }
}

```

### Source intégrale : docs/plans/graphe-automatisation-20261005/execution/d/verify_d03_final.py

```
"""D03 host qualification, one global suite with bounded total runtime."""
import importlib.util,json,hashlib,re,sys,time,subprocess,os,signal
from pathlib import Path
D=Path(__file__).resolve().parent

def suite_summary(output):
 inventory=re.search(r'GO SUITE package=(\S+) tests=(\d+) shards=(\d+) coverage=disjoint-complete compile=once',output)
 completion=re.search(r'PASS full discovered Go suite: (\d+) tests; all shards exited 0\.',output)
 assert inventory and completion,'full suite inventory/completion missing'
 total=int(inventory[2]);shards=int(inventory[3]);assert total>0 and int(completion[1])==total
 records=re.findall(r'SHARD (\d+) exit_code=(\d+) elapsed=([0-9.]+)s covered=(\d+)/(\d+) passed=(\d+)',output)
 assert len(records)==shards and len({r[0] for r in records})==shards,'missing/duplicate shard receipts'
 assert all(r[1]=='0' and r[3]==r[4] for r in records),'failed/incomplete shard'
 assert sum(int(r[4]) for r in records)==total,'coverage count mismatch'
 skips=[json.loads(line) for line in output.splitlines() if line.startswith('{') and 'optional_skip' in line]
 required=('TestGraphDeliveryD','TestAutomation','TestGraphDraftB','TestGraphPerformance','TestReviewTokenCache')
 assert not any(s['optional_skip'].startswith(required) for s in skips),'required delivery/automation test skipped'
 passed=sum(int(r[5]) for r in records)
 top_skips=[s for s in skips if '/' not in s['optional_skip']]
 assert passed+len(top_skips)==total,'passed/skipped accounting mismatch'
 return {'discovered':total,'passed':passed,'shards':shards,'optional_skips':skips,'compile':'once','coverage':'disjoint-complete'}

def main():
 spec=importlib.util.spec_from_file_location('d_verify',D/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
 m=json.loads((D/'d02-fresh-browser-manifest.json').read_text())
 assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==m['base_git']
 for n,h in {**m['candidate_inputs'],**m['artifacts'],**json.loads((D/'protected-d01.json').read_text())}.items():
  assert hashlib.sha256(Path(n).read_bytes()).hexdigest()==h,'browser/protected input changed: '+n
 receipt=json.loads(Path(m['browser_receipt']).read_text());assert receipt['surface']=='swarm-product'
 assert {x['variant'] for x in receipt['variants']}=={'fr-sombre','fr-etat','en-sombre','en-etat'}
 assert all(len(x['assertions'])==12 and all(x['assertions'].values()) and not x['console_errors'] and not x['network_errors'] for x in receipt['variants'])
 deadline=time.monotonic()+295;summary=None
 def bounded(cmd,timeout,d):
  nonlocal summary
  remaining=deadline-time.monotonic();assert remaining>0,'global qualification deadline exhausted'
  started=time.monotonic();log=d/(str(len(list(d.glob('*.json'))))+'.log')
  with log.open('w') as f:
   proc=subprocess.Popen(cmd,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
   try:code=proc.wait(timeout=min(timeout,remaining))
   except subprocess.TimeoutExpired:
    os.killpg(proc.pid,signal.SIGTERM)
    try:proc.wait(timeout=2)
    except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
    raise AssertionError('qualification command deadline exceeded: '+str(cmd)+'; log='+str(log))
  output=log.read_text();record={'command':cmd,'exit_code':code,'seconds':round(time.monotonic()-started,3),'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
  log.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n')
  assert code==0,json.dumps(record)+'\n'+output[-4000:]
  if cmd==['python3','tests/supervision_go_suite.py']:
   summary=suite_summary(output);summary['log']=record; (d/'full-suite-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
  return output
 v.run=bounded;v.main('D03');assert summary is not None
 print(json.dumps({'global_qualification':'PASS','suite':summary,'browser_candidate_bound':m['browser_receipt'],'limits':'isolated fixtures; provider autonomy and installed D not demonstrated'}))

if __name__=='__main__':
 if '--self-test' in sys.argv:
  sample='GO SUITE package=x tests=2 shards=1 coverage=disjoint-complete compile=once\nSHARD 0 exit_code=0 elapsed=1.0s covered=2/2 passed=1\n'+json.dumps({'optional_skip':'TestOptionalProbe','observed_output':['opt-in disabled']})+'\nPASS full discovered Go suite: 2 tests; all shards exited 0.'
  assert suite_summary(sample)['passed']==1
  for wrong in [sample.replace('PASS full discovered','MISSING full discovered'),sample.replace('TestOptionalProbe','TestGraphDeliveryD03QualificationManifest'),sample.replace('covered=2/2','covered=1/2'),sample.replace('passed=1','passed=2')]:
   try:suite_summary(wrong)
   except AssertionError:continue
   raise AssertionError('invalid suite receipt accepted')
  print('PASS wrapper self-test only; no functional/global suite executed')
 else:main()

```

### Source intégrale : tests/supervision_go_suite.py

```
"""Compile once; run every discovered Go test in disjoint bounded processes."""
import concurrent.futures
import json
import re
import subprocess
import tempfile
import time
from pathlib import Path


def partition(names, plan):
    if not names or len(names) != len(set(names)):
        raise ValueError('empty or duplicate test inventory')
    isolated = plan['isolated']
    if len(isolated) != len(set(isolated)) or not set(isolated) <= set(names):
        raise ValueError('isolated test inventory mismatch')
    remaining = set(names) - set(isolated)
    groups = [[] for _ in range(min(plan['groups'], len(remaining)))]
    weights = plan['observed_seconds']
    costs = [0.0 for _ in groups]
    # Greedy scheduling changes order only. Unknown durations have a conservative
    # unit cost; observed durations under load are hints, never acceptance data.
    for name in sorted(remaining, key=lambda n: (-weights.get(n, 1), n)):
        index = min(range(len(groups)), key=lambda i: (costs[i], len(groups[i]), i))
        groups[index].append(name)
        costs[index] += weights.get(name, 1)
    all_groups = [[name] for name in isolated] + groups
    if sorted(name for group in all_groups for name in group) != sorted(names):
        raise ValueError('incomplete or duplicate coverage')
    return all_groups, len(isolated)


def run(group, index, binary, package, timeout):
    pattern = '^(?:' + '|'.join(re.escape(name) for name in group) + ')$'
    command = ['go', 'tool', 'test2json', '-t', '-p', package, str(binary),
               '-test.run', pattern, '-test.count=1', '-test.v=test2json',
               f'-test.timeout={timeout}s']
    started = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    observed, passed, skipped = set(), set(), {}
    last = started
    failures = []
    for line in process.stdout:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            failures.append(line.rstrip())
            continue
        test, action = event.get('Test', ''), event.get('Action')
        if action == 'run' and '/' not in test:
            observed.add(test)
        if action == 'pass' and test and '/' not in test:
            passed.add(test)
        if action == 'skip':
            skipped[test] = list(failures[-5:])
        if action == 'run' and time.monotonic() - last >= 15:
            print(f'SHARD {index} RUN {test} elapsed={time.monotonic()-started:.1f}s', flush=True)
            last = time.monotonic()
        if action == 'output':
            failures.append(event.get('Output', '').rstrip())
            failures = failures[-50:]
        if action == 'fail':
            print(f'SHARD {index} FAIL {test}\n' + '\n'.join(failures), flush=True)
    code = process.wait()
    missing = set(group) - observed
    incomplete = set(group) - passed - set(skipped)
    # Required feature cases may never disappear behind an optional skip.
    required_skips = {name for name in skipped if name.startswith(('TestGraphDraftB', 'TestGraphPerformance', 'TestReviewTokenCache'))}
    print(f'SHARD {index} exit_code={code} elapsed={time.monotonic()-started:.1f}s covered={len(observed)}/{len(group)} passed={len(passed)}', flush=True)
    for name, reason in skipped.items():
        print(json.dumps({'optional_skip': name, 'observed_output': reason}), flush=True)
    if code or missing or incomplete or required_skips:
        print(f'SHARD {index} missing={sorted(missing)} incomplete={sorted(incomplete)} required_skips={sorted(required_skips)}\n' + '\n'.join(failures), flush=True)
        return 1
    return 0


def main():
    plan = json.loads(Path('tests/supervision_go_schedule.json').read_text())
    assert plan['version'] == 1 and 1 <= plan['parallelism'] <= 16
    assert 1 <= plan['groups'] <= 16 and 0 < plan['test_timeout_seconds'] <= 240
    assert all(isinstance(v, (float, int)) and 0 <= v < 10000 for v in plan['observed_seconds'].values())
    packages = subprocess.run(['go', 'list', './...'], capture_output=True, text=True, check=True).stdout.splitlines()
    if len(packages) != 1:
        raise SystemExit('Package inventory changed: update suite coverage before running.')
    with tempfile.TemporaryDirectory(prefix='swarm-go-suite-') as folder:
        binary = Path(folder)/'companion.test'
        subprocess.run(['go', 'test', '-c', '-o', str(binary), packages[0]], check=True)
        listing = subprocess.run([str(binary), '-test.list', '.'], capture_output=True, text=True, check=True)
        names = [name for name in listing.stdout.splitlines() if re.match(r'^(Test|Example|Fuzz)\w*$', name)]
        groups, isolated = partition(names, plan)
        print(f'GO SUITE package={packages[0]} tests={len(names)} shards={len(groups)} coverage=disjoint-complete compile=once', flush=True)
        codes = [run(groups[i], i, binary, packages[0], plan['test_timeout_seconds']) for i in range(isolated)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=plan['parallelism']) as pool:
            codes.extend(pool.map(lambda pair: run(pair[1], pair[0]+isolated, binary, packages[0], plan['test_timeout_seconds']), enumerate(groups[isolated:])))
        if any(codes):
            raise SystemExit(1)
        print(f'PASS full discovered Go suite: {len(names)} tests; all shards exited 0.', flush=True)
        print('Optional Go skips are listed explicitly above, never counted as passed; live-provider probes and opt-in browser recipes are not implied.', flush=True)


if __name__ == '__main__':
    main()

```

### Source intégrale : docs/graph-delivery-qualification-manifest.json

```
{
  "schema_version": 1,
  "manifest_status": "frozen_candidate_pending_host_controls_and_independent_review",
  "base_commit": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed",
  "candidate_id": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed+sha256:d07015b6d564ef664a7cb21488bcfbac4361ae260568a06b7dd4b43710e38df7",
  "candidate_digest": "d07015b6d564ef664a7cb21488bcfbac4361ae260568a06b7dd4b43710e38df7",
  "coverage_digest": "40ecbfee659a1910df25ec5ef97036e86f99d6c95185d3ef996be979b9b4e66a",
  "requirements": [
    {
      "id": "R01",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B03.md",
        "docs/D01.md"
      ],
      "limit": "Preparation exercised without a paid provider; final review pending."
    },
    {
      "id": "R02",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B01.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R03",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B01.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R04",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B03.md",
        "docs/D01.md"
      ],
      "limit": "Browser evidence is fixture-backed; final review pending."
    },
    {
      "id": "R05",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B03.md",
        "docs/D01.md"
      ],
      "limit": "Browser evidence is fixture-backed; final review pending."
    },
    {
      "id": "R06",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B04.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R07",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B02.md",
        "docs/B03.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R08",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B02.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R09",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C01.md",
        "docs/D01.md"
      ],
      "limit": "No paid provider autonomy is claimed."
    },
    {
      "id": "R10",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C01.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R11",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C02.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R12",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C03.md",
        "docs/D01.md"
      ],
      "limit": "External events use local fixtures; no live provider is claimed."
    },
    {
      "id": "R13",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C02.md",
        "docs/D01.md"
      ],
      "limit": "Final host suite and review pending."
    },
    {
      "id": "R14",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B04.md",
        "docs/D01.md"
      ],
      "limit": "Current candidate attachment must be checked after the host run."
    },
    {
      "id": "R15",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B03.md",
        "docs/C04.md",
        "docs/D01.md"
      ],
      "limit": "Four browser variants are fixture-backed; final review pending."
    },
    {
      "id": "R16",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/D02.md",
        "docs/D02-dossier.md"
      ],
      "limit": "Installed-candidate verification belongs to D04; final review pending."
    },
    {
      "id": "R17",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/C01.md",
        "docs/D01.md"
      ],
      "limit": "Engine acceptance is not asserted by this manifest."
    },
    {
      "id": "R18",
      "status": "COVERED_PENDING_FINAL_CONTROL_AND_REVIEW",
      "evidence": [
        "docs/B04.md",
        "docs/D01.md"
      ],
      "limit": "Real provider cost and autonomy remain unknown."
    }
  ],
  "inputs": [
    {
      "path": "automation_archive.go",
      "kind": "source",
      "sha256": "7be45c48bf5d9a1f88e28e5a4097c262964776f0248a69d840d8a99cb97b79f3"
    },
    {
      "path": "automation_cli.go",
      "kind": "source",
      "sha256": "bfe5c776d25ae917292aec825801cb41c96d0c514b6ca211eb7baaf9fcc335bb"
    },
    {
      "path": "automation_external.go",
      "kind": "source",
      "sha256": "18f3d8d9d8f8cbea07189b5a353edbada68473320f9f4b6105defb08e5731c2d"
    },
    {
      "path": "automation_http.go",
      "kind": "source",
      "sha256": "c8b2849f59e911a52b84a1e030ee904ff5322c265481aae3d7932e63e85b666e"
    },
    {
      "path": "automation_requests.go",
      "kind": "source",
      "sha256": "6806e77db5107e0acccc583fd86eb7143ca27fd20964fd55de5a75dacfe78c40"
    },
    {
      "path": "automation_schedule.go",
      "kind": "source",
      "sha256": "12679cbefaf5a66b13c0981855256b96208b3e23d977518dc7563427f2d56ca1"
    },
    {
      "path": "graph_draft.go",
      "kind": "source",
      "sha256": "c396c9d2303e213f88e8416ce5bd8aadfbf926077301c691eab2a389e2eb470f"
    },
    {
      "path": "graph_draft_cli.go",
      "kind": "source",
      "sha256": "3b3e3de387c052f6150a1ad1858deb25614c7363da2e724fcb19a79c3d5593d7"
    },
    {
      "path": "graph_draft_http.go",
      "kind": "source",
      "sha256": "88e40ee9dea2525a9051e1f190871469de314233951937381633cbdd5477ac35"
    },
    {
      "path": "graph_draft_projection.go",
      "kind": "source",
      "sha256": "a4085936414f083283a6a3dac6738525a7e114954d2d91d5e6917fb0c1485121"
    },
    {
      "path": "store.go",
      "kind": "source",
      "sha256": "192e1d8bc0bbb04dc1d1fc1697f8622fa03da5a692dd199af775f155e980061a"
    },
    {
      "path": "web_server.go",
      "kind": "source",
      "sha256": "5064e603e1032b19839ceb07e4d227732a0512c8fc401788ebb11f0f94a5a541"
    },
    {
      "path": "main.go",
      "kind": "source",
      "sha256": "258df7e0669ee74fc653c6110c34e7eb40b0cca3cd349e35006ba25c5fce14d7"
    },
    {
      "path": "web/automation.js",
      "kind": "source",
      "sha256": "2778f1430e557b25c0d822fa2b5616caf0319605015984e88322e8cf5ab12eb3"
    },
    {
      "path": "web/graph-draft.js",
      "kind": "source",
      "sha256": "641351c57a57609120aea0e446a9861dbe068409b2d9f5494aff26a019a34c5f"
    },
    {
      "path": "web/graph.js",
      "kind": "source",
      "sha256": "05a40f9cfd160969432750a079a4b0bf2e49834ea6c3eca74c7c5a3f300f2ff8"
    },
    {
      "path": "graph_delivery_e2e_test.go",
      "kind": "test",
      "sha256": "82bf6ac3434af24cd40472dd82a5375e1708819785ed3f190ecea6f2fdd9c731"
    },
    {
      "path": "graph_delivery_migration_test.go",
      "kind": "test",
      "sha256": "35dc372ac9980f851408951bdcfda1fc62db5869451141ff3c6642b3d84fe154"
    },
    {
      "path": "graph_delivery_qualification_test.go",
      "kind": "test",
      "sha256": "bd5d9b2b61b1da56dbfadda8b98158d5e85ff389004cbb6b412491a0d29d2aaf"
    },
    {
      "path": "tests/graph_automation_e2e.cjs",
      "kind": "test",
      "sha256": "c30c9e7d8fc998da75428bffd71175172decd47f61ee3827e399b294a68ef85f"
    },
    {
      "path": "tests/graph_delivery_docs.py",
      "kind": "test",
      "sha256": "c7708bf7dc7ea6b93d3a92be23d3a5fd5925bcd7c579e0b73cc0f5fc6892980a"
    },
    {
      "path": "docs/D01.md",
      "kind": "doc",
      "sha256": "f2921cbeec0787e2a5c667f4bc3396d02e2254d7fc7794c54ac76c654b46cd9b"
    },
    {
      "path": "docs/D01-dossier.md",
      "kind": "doc",
      "sha256": "eec23768a19f22c8e33d7b84ff7f8693c98f8ac41a909a96d64ad2a77e9c96dd"
    },
    {
      "path": "docs/D02.md",
      "kind": "doc",
      "sha256": "9bc40fd0a6a0467a743d859a7eeab0b0d1b1126e39f5ea8f3394d3a4419fcefc"
    },
    {
      "path": "docs/D02-dossier.md",
      "kind": "doc",
      "sha256": "d99300b9f211d60add7c998b4869934c34fbfc451cd04cc55a0ca2f2e98be017"
    },
    {
      "path": "docs/A01.md",
      "kind": "doc",
      "sha256": "3d8b6fe14c2092dbb853fd779e5ee7e4c1221cdadda8dfbf7fa7d0229dc606a8"
    },
    {
      "path": "docs/A02.md",
      "kind": "doc",
      "sha256": "f56d34cdabda43f64d0d19b379e641bacf7aa749f05e416e4ec4d18be4c28751"
    },
    {
      "path": "docs/A03.md",
      "kind": "doc",
      "sha256": "efc3f66735fce452777548f685465cf3427bae7c0decb0e59f39eeaccb2730fa"
    },
    {
      "path": "docs/A04.md",
      "kind": "doc",
      "sha256": "da21b2e6b2df5ba9558779c62df04efe78967f559e5df706a5bb4c9415143a92"
    },
    {
      "path": "docs/B01.md",
      "kind": "doc",
      "sha256": "0680b4092712dc8c805f8cf583be84cbb92e7f28a8713d97db7aedd268bf696d"
    },
    {
      "path": "docs/B02.md",
      "kind": "doc",
      "sha256": "a836ba979f697a446396eebac38553abebb14b854c9a6118bbabe43d6fac6a9c"
    },
    {
      "path": "docs/B03.md",
      "kind": "doc",
      "sha256": "30c21c82fe60f76c52babc24c8364ed4269e2850ec95071d38f15277778afcbc"
    },
    {
      "path": "docs/B04.md",
      "kind": "doc",
      "sha256": "db0994f13b032d2d0ebb3aa0d8a315a162867d5d7cf463d381623463ddd08be1"
    },
    {
      "path": "docs/C01.md",
      "kind": "doc",
      "sha256": "7590cee9a8268b8137eea4303ac1aeb80af52f9554d4a156db3d337d4b5e3916"
    },
    {
      "path": "docs/C02.md",
      "kind": "doc",
      "sha256": "e07d760ce33a02b088c3156ee8c1662ac18da65504046984d97db59ba461efc4"
    },
    {
      "path": "docs/C03.md",
      "kind": "doc",
      "sha256": "8b8370757b802b4116330d8fee7907d35f7f1aee291e9d2cbc99afc54f2e7776"
    },
    {
      "path": "docs/C04.md",
      "kind": "doc",
      "sha256": "ce82d774954be88c5736b46ad851d0b1bbd95bde6a4835281ca672989c5aa58d"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/PLAN.md",
      "kind": "doc",
      "sha256": "dad983571d368780a4a8b65161ea7fa5cadcdf3269299355ab7cba1055ac0e72"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/RECETTE.md",
      "kind": "doc",
      "sha256": "1ef8f4f63fec5aba213e93e20466a9a2eb5d9825eae3f8f47003c97e91843737"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/CONTRACTS.md",
      "kind": "doc",
      "sha256": "eaf91e3adfc61bb1b704a66c435970cd51940c26c7a212e92b3017aa327b5a23"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/execution/PREPARATION-D.md",
      "kind": "doc",
      "sha256": "cde49f38c27efb08385206491f45b3ae09375da33ef659c8f645e6659ca4a7f4"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/execution/d/INSTRUCTIONS.md",
      "kind": "doc",
      "sha256": "894bfaf377c331edfdce898b7a23238cc0982e2e778c79609a1f5617fcb85b23"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/execution/d/verify.py",
      "kind": "harness",
      "sha256": "02c3e01e1126772195dc04adaacdfcf05ff3321fca297adfbcdad494ce7d6d20"
    },
    {
      "path": "tests/supervision_go_suite.py",
      "kind": "harness",
      "sha256": "6319157b107c435e89b3c452cde2e398adb37f91fce341f0faf4dba035e80d5d"
    },
    {
      "path": "tests/supervision_go_schedule.json",
      "kind": "harness",
      "sha256": "b4b04aa02034cd2d67030f9a6048069fa060fe37ad8fa3f39b1573b4cc179ac5"
    },
    {
      "path": "tools/agent-workflows/check.py",
      "kind": "harness",
      "sha256": "1d02fd58332e1d8e3aa3cea2857085b799a9edf14c63956f2d4f476ea54a41d6"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/fr-etat-graph.png",
      "kind": "capture",
      "sha256": "cdef3e04e8c27ae00d8a2b993515570cbbd03c87cfb9337686c11706be6476ca"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/fr-sombre-graph.png",
      "kind": "capture",
      "sha256": "09e0af40ef9a8712a19f681ac1a83ea39a7a74c8028a3f415cd73fbf901fe362"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/en-etat-graph.png",
      "kind": "capture",
      "sha256": "a8cc2dc1051756b8ae47643370d5af33db4af821d5d5358108c8a26944e43b70"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/en-sombre-graph.png",
      "kind": "capture",
      "sha256": "201fc68bcb513d24f89d35c69c4eb0bd3336c07756c652bafd4bf4ccf2765fa8"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/fr-etat-programs.png",
      "kind": "capture",
      "sha256": "0ff3c7320a508ecbf72405eeba81428a185408a535bffc0c55b2a3d53ecb577e"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/fr-sombre-programs.png",
      "kind": "capture",
      "sha256": "b63ebef0687462fcdf528fb2c6278f023852b398064cd1d5b47fab4aaf42431b"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/en-etat-programs.png",
      "kind": "capture",
      "sha256": "c5cfb65d3e393ea27b90a1266fe325188dbd654086a129398a92778ac9b375af"
    },
    {
      "path": "docs/screenshots/graph-delivery-d01/en-sombre-programs.png",
      "kind": "capture",
      "sha256": "b13021506c5a8495d6a6236e0fbf64e9ec11526cdf0dfa775e7a63e31af75d32"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/fr-etat-graph.png",
      "kind": "capture",
      "sha256": "3e978ce9ac23f7c094e9dea069ad2251b4a9339a9d4f9501726edbfb7f3a95d3"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/fr-sombre-graph.png",
      "kind": "capture",
      "sha256": "e2d15e10c8d8124339c383fe2ec13324675f2af03fa226813aa9866862bac17e"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/en-etat-graph.png",
      "kind": "capture",
      "sha256": "2922f3c51d0b281260630a5f0479b92005e4768a76d22d0e07c10ec7c20074f1"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/en-sombre-graph.png",
      "kind": "capture",
      "sha256": "2a7b8ef6d1d776321f3022365b90a83fece7edd4921028c9f0729b3ccf04edd3"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/fr-etat-programs.png",
      "kind": "capture",
      "sha256": "7c0811167dd599b56f6f6f0e59b543b2b1bc53305481ef7bab9fdb85725dc162"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/fr-sombre-programs.png",
      "kind": "capture",
      "sha256": "ed9147643deb65b86fd017318f378498bce52734aa01b051ea5c208b7e19733a"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/en-etat-programs.png",
      "kind": "capture",
      "sha256": "74c00de6f1c4c0ade932e5afcb9825112b6202705cfb3ace5151182e5ef7e721"
    },
    {
      "path": "docs/screenshots/graph-delivery-d02/en-sombre-programs.png",
      "kind": "capture",
      "sha256": "58b9f551b1aa5dba50d6535f55329ddbf9942c7f5877f3c47bcb6fc282b4dc98"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/execution/d/verify_d03_final.py",
      "kind": "harness",
      "sha256": "9209ed44f527ac5fd58cd2ee2723ff8fc22bc50314943700bfaad1e706ec7042"
    },
    {
      "path": "docs/plans/graphe-automatisation-20261005/execution/d/d02-fresh-browser-manifest.json",
      "kind": "harness",
      "sha256": "abf067330dd681de55c1a2f0bd746cf505990a91078cd9896472233f39d2a732"
    },
    {
      "path": "package.json",
      "kind": "harness",
      "sha256": "a960655c5a00558220c3a3e3ea7b3c71885b931beb814881470dda1139386f4d"
    },
    {
      "path": "build.sh",
      "kind": "harness",
      "sha256": "4a89827e8fe630364276beb90012af0456b40e042443aa1ce0269c49d6fac5a6"
    },
    {
      "path": "go.mod",
      "kind": "harness",
      "sha256": "c8894ded99e6e8060f4183e70a3f3b2050c6fb1f038bd2b7b41bf05fa2e8a9b7"
    },
    {
      "path": "go.sum",
      "kind": "harness",
      "sha256": "6d49db237b4443f4102404a1c2cb976d95835d221a5e383935c958bf242d274e"
    }
  ]
}

```

### Source intégrale : docs/plans/graphe-automatisation-20261005/execution/d/verify.py

```
"""Supervisor-owned stage D host checks; missing/skipped feature tests fail closed."""
from pathlib import Path
import hashlib,json,subprocess,sys,tempfile,time,uuid
ROOT=Path(__file__).resolve().parent
EXPECTED={
 'D01':['PublicLifecycle','RequestReplay','Conflict','WorkspaceWait','RestartRetention','ProofFreshness'],
 'D02':['Migration','ArchiveCompatibility','BackupRestoreSafety'],
 'D03':['QualificationManifest'],
 'D04':['InstallVersion','RollbackSafety']}

def protect():
 for name,wanted in json.loads((ROOT/'protected-reports.json').read_text()).items():
  assert hashlib.sha256(Path(name).read_bytes()).hexdigest()==wanted,'accepted report changed: '+name
def passed(inventory,events):
 assert inventory and len(inventory)==len(set(inventory)),'empty/duplicate inventory'
 done={e.get('Test') for e in events if e.get('Action')=='pass'}
 assert not any(e.get('Action') in ['skip','fail'] for e in events),'required feature skipped/failed'
 assert set(inventory)<=done,'feature not all executed'
def run(cmd,timeout,d):
 start=time.monotonic();p=subprocess.run(cmd,capture_output=True,text=True,timeout=timeout)
 log=d/(str(len(list(d.glob('*.json'))))+'.log');log.write_text(p.stdout+p.stderr)
 rec={'command':cmd,'exit_code':p.returncode,'seconds':round(time.monotonic()-start,3),'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()}
 (log.with_suffix('.json')).write_text(json.dumps(rec,indent=2)+'\n')
 assert p.returncode==0,json.dumps(rec)+'\n'+p.stdout[-4500:]+p.stderr[-2000:]
 return p.stdout
def main(stage):
 assert stage in EXPECTED
 protect();assert Path('docs/'+stage+'.md').is_file(),'report missing'
 d=ROOT/'results'/uuid.uuid4().hex;d.mkdir(parents=True)
 pattern='^TestGraphDelivery'+stage
 listing=run(['go','test','./...','-list',pattern],90,d)
 inventory=[n for n in listing.splitlines() if n.startswith('TestGraphDelivery'+stage)]
 wanted={'TestGraphDelivery'+stage+n for n in EXPECTED[stage]}
 assert wanted<=set(inventory),'missing feature tests: '+','.join(sorted(wanted-set(inventory)))
 output=run(['go','test','-race','./...','-run',pattern,'-count=1','-json','-timeout=120s'],150,d)
 events=[json.loads(l) for l in output.splitlines() if l.startswith('{')];passed(inventory,events)
 if stage=='D01':
  recipe=Path('tests/graph_automation_e2e.cjs');assert recipe.is_file(),'native product browser recipe missing'
  with tempfile.TemporaryDirectory(prefix='swarm-delivery-d-') as tmp:
   binary=str(Path(tmp)/'swarm');run(['sh','build.sh',binary],90,d)
   run(['node',str(recipe),binary,str(d/'browser')],150,d)
   p=d/'browser/results.json';assert p.is_file(),'browser receipt missing';b=json.loads(p.read_text());assert b['surface']=='swarm-product'
   assert {v['variant'] for v in b['variants']}=={'fr-sombre','fr-etat','en-sombre','en-etat'}
   req=['prepare_roles','dependency_apply','cycle_rejected','agent_journal','program_replay','content_conflict','workspace_wait','restart_retention','causal_recovery','unknown_cost','focus_restore','graph_preserved']
   for v in b['variants']:
    assert all(v['assertions'].get(k) is True for k in req),v['variant']
    assert not v.get('console_errors') and not v.get('network_errors')
    capture=(p.parent/v['screenshot']).read_bytes();assert capture[:8]==b'\x89PNG\r\n\x1a\n' and len(capture)>1000
   print(json.dumps({'browser':str(p),'variants':[v['variant'] for v in b['variants']],'assertions':req},ensure_ascii=False))
 if stage=='D02':
  run(['python3','tests/graph_delivery_docs.py',str(d/'documentation')],90,d)
 if stage=='D03':
  run(['go','vet','./...'],60,d)
  run(['npm','test'],60,d)
  run(['python3','tools/agent-workflows/check.py'],30,d)
  output=run(['python3','tests/supervision_go_suite.py'],210,d)
  assert 'PASS' in output,'suite completion missing'
 run(['git','diff','--check'],10,d);protect()
 print(json.dumps({'stage':stage,'status':'PASS','tests':inventory,'artifacts':str(d),'scope':'targeted host control; fixtures, no real provider autonomy or installed C claim'},ensure_ascii=False))
if __name__=='__main__':
 if sys.argv[1]=='--self-test':
  passed(['X'],[{'Action':'pass','Test':'X'}])
  for inv,events in [([],[]),(['X'],[]),(['X'],[{'Action':'skip','Test':'X'}]),(['X'],[{'Action':'fail','Test':'X'}]),(['X','X'],[{'Action':'pass','Test':'X'}])]:
   try:passed(inv,events)
   except AssertionError:continue
   raise AssertionError('fail-closed harness regression')
  protect();print('PASS harness rejects missing/skipped/failed/duplicate observations; feature functionality not yet tested')
 else:main(sys.argv[1])

```
