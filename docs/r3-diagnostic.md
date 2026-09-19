# R3 — Expliquer la cause réelle du blocage

Date de la tentative : 19 septembre 2026
Périmètre : priorité du diagnostic sur le chemin d'intégration gérée (`managed_integration.go`, `conductor.go`), lot R3 uniquement. Ne refait ni P0-P3 ni R1/R2.

## Verdict borné

Le bug était réel et localisé : `integrateManagedAttempt` re-dérivait systématiquement la preuve du rapport depuis la copie de travail éphémère (`item.Path`), même quand ce rapport était déjà validé et committé dans la copie gérée (`item.Result` déjà renseigné en base). Une copie de travail vidée ou modifiée entre deux passes faisait donc échouer le diagnostic sur « Rapport de tentative absent », masquant la cause réelle d'un échec d'intégration ultérieur (contrôle en échec, fournisseur absent, plafond, verrou). Le correctif fait confiance à la copie gérée dès que `item.Result` existe et ne relit plus jamais la copie de travail pour cette vérification. Cinq tests nouveaux prouvent ce comportement, dont trois échouaient effectivement avant correctif (preuve ci-dessous, sans assertion tautologique).

## Cause réelle

### Où

`managed_integration.go`, fonction `integrateManagedAttempt`, lignes 58-95 (avant correctif).

### Séquence du bug

1. Une tentative complète son travail ; `docs/<tâche>.md` est écrit dans la copie de travail (`item.Path`).
2. `integrateManagedAttempt` commit ce rapport dans le dépôt bare géré (`git commit-tree`, `fetch`, `update-ref`), enregistre `result_commit` en base (`managed_attempts.result_commit`) et passe l'état à `integrating`.
3. Si le processus s'arrête avant la fin (fusion, contrôles, publication) — crash, redémarrage, ou tout appel ultérieur à `integrateManagedAttempt` pour la même tentative — la fonction est rappelée. `item.Result` est maintenant non vide : le rapport existe déjà, vérifié, dans la copie gérée.
4. **Avant correctif** : les lignes 58-69 relisaient quand même `docs/<tâche>.md` depuis `item.Path` à chaque appel, sans condition. Si ce fichier n'était plus présent ou lisible dans la copie de travail (répertoire réutilisé, nettoyage, tout écart entre deux passes), la fonction retournait `"Rapport de tentative absent : " + err` via `managedFailure`, écrasant `task.Blocker` avec un diagnostic faux : le rapport n'était pas absent, il était déjà dans la copie gérée. La cause réelle d'un échec plus loin dans la fonction (fusion, contrôle, fournisseur, plafond) n'était jamais atteinte ni rapportée.
5. **Après correctif** : le bloc de dérivation depuis la copie de travail (lecture, comparaison à la base, commit) est maintenant strictement borné à `item.Result == ""` — c'est-à-dire au tout premier passage, avant que le rapport n'existe dans la copie gérée. Dès que `item.Result != ""`, la fonction lit directement le contenu déjà vérifié dans le dépôt bare (`git show <result>:<rapport>`) et ne touche plus la copie de travail pour cette preuve. Tout échec après ce point (fusion, contrôle, fournisseur, plafond, verrou) est donc rapporté pour ce qu'il est réellement — une intégration échouée — jamais masqué en « rapport absent ».

### Diff appliqué

```go
// managed_integration.go
-	reportPath, err := localFile(item.Path, reportName)
-	if err != nil {
-		return s.managedFailure(a, "Rapport de tentative absent : "+err.Error())
-	}
-	report, err := os.ReadFile(reportPath)
-	if err != nil || len(report) == 0 || len(report) > 1<<20 {
-		return s.managedFailure(a, "Rapport de tentative vide, illisible ou trop grand")
-	}
-	if old, e := managedGit(item.Path, "show", item.Base+":"+reportName); e == nil && hash(...) == hash(...) {
-		return s.managedFailure(a, "Rapport identique à la base ; nouvelle preuve de tentative requise")
-	}
-	if item.Result == "" {
+	var report []byte
+	if item.Result == "" {
+		reportPath, err := localFile(item.Path, reportName)
+		if err != nil {
+			return s.managedFailure(a, "Rapport de tentative absent : "+err.Error())
+		}
+		report, err = os.ReadFile(reportPath)
+		if err != nil || len(report) == 0 || len(report) > 1<<20 {
+			return s.managedFailure(a, "Rapport de tentative vide, illisible ou trop grand")
+		}
+		if old, e := managedGit(item.Path, "show", item.Base+":"+reportName); e == nil && hash(...) == hash(...) {
+			return s.managedFailure(a, "Rapport identique à la base ; nouvelle preuve de tentative requise")
+		}
 		... (commit-tree, fetch, update-ref, UPDATE managed_attempts, vérification) inchangés ...
+	} else {
+		// Rapport déjà committé et vérifié par un passage antérieur : faire
+		// confiance à la copie gérée, pas à la copie de travail éphémère.
+		committed, e := managedGit(bare, "show", item.Result+":"+reportName)
+		if e != nil {
+			return s.managedFailure(a, "Intégration échouée : révision "+item.Result+" absente de la copie gérée ("+e.Error()+")")
+		}
+		report = []byte(committed)
 	}
```

## Cause, acteur, action, condition de reprise

| Scénario testé | Cause réelle | Acteur | Action attendue | Condition de reprise |
| --- | --- | --- | --- | --- |
| Rapport déjà dans la copie gérée, copie de travail vidée | Reprise après interruption mi-intégration ; aucune perte réelle | Moteur | Aucune : l'intégration continue automatiquement depuis la copie gérée | Aucune reprise humaine requise ; ce n'est pas un blocage |
| Contrôle en échec (« contrôle échoué ») | Un contrôle automatique préautorisé a échoué sur la révision candidate | Responsable du périmètre | Examiner la trace du contrôle (`Contrôle <id> de <tâche> en échec : <résumé> (empreinte <sha>)`), corriger la cause | Rejouer le même contrôle avec les mêmes options après correction vérifiée ; jamais relancer sans changement |
| Fournisseur absent/quota | L'exécutable requis par un contrôle est introuvable ou son quota est dépassé | Opérateur | Installer/authentifier le fournisseur dans l'environnement qui exécute le contrôle, ou attendre le renouvellement du quota | Reprise seulement après vérification que le fournisseur répond réellement (voir `GUIDE-UTILISATEUR.md` §7 « Fournisseur absent ») |
| Plafond (mission mise en pause / autonomie désactivée entre deux passes) | `automaticValidationAuthorized` refuse la poursuite ; l'intégration se suspend | Opérateur | Examiner la préconisation ; ne pas lever le plafond en silence | Reprise seulement après décision explicite de l'opérateur de réactiver la mission continue |
| Espace occupé (verrou d'intégration détenu par une autre tentative) | `managedLock` refuse une opération Git concurrente sur la même mission | Moteur / opérateur | Attendre la fin de l'opération qui détient le verrou, ou l'examiner si elle semble bloquée | Reprise automatique dès libération du verrou ; aucune relance concurrente sur le même espace |
| Rapport réellement absent (premier passage, aucun `item.Result`) | Le rapport n'a jamais été écrit ou committé | Worker / opérateur | Produire réellement `docs/<tâche>.md` avant de clore la tentative | Reprise seulement après nouvelle tentative produisant le rapport |

`task.Blocker` distingue ces causes textuellement (« Contrôle … en échec », « intégration suspendue : … », « une opération Git est déjà en cours … », « Rapport de tentative absent : … »). Le correctif ne change aucun de ces textes existants : il change seulement quand chacun peut légitimement s'appliquer, pour que « rapport absent » ne soit plus jamais produit à tort une fois le rapport déjà dans la copie gérée.

### Ne jamais relancer à identique

Ce correctif ne relance rien automatiquement : une fois `task.Blocker` posé (contrôle en échec, verrou, plafond), la tâche reste `blocked` et attend une décision `retry` explicite et différente (`TestManagedRetryRequiresNewDecisionAndStaysBounded`, préexistant, non modifié par ce lot). Les scénarios transitoires (verrou, plafond) laissent volontairement `task.Blocker` inchangé plutôt que de le réécrire à chaque nouvel essai : c'est `conductor.go` qui, déjà avant ce lot, exclut explicitement « déjà en cours » et `SQLITE_BUSY` de la conversion en blocage permanent — comportement vérifié, pas modifié, par `TestManagedIntegrationWorkspaceLockOutranksMissingReport` et `TestManagedIntegrationCeilingSuspensionNeverMasksMissingReport`.

## Tests fournis

Ajoutés dans `managed_git_test.go` (package `main`, build `linux`) :

- `TestManagedIntegrationTrustsReportAlreadyInManagedCopy` — régression directe : rapport déjà dans la copie gérée + copie de travail vidée → l'intégration se termine (`accepted`), plus de faux blocage.
- `TestManagedIntegrationFailedControlOutranksMissingReport` — contrôle échoué.
- `TestManagedIntegrationMissingProviderOutranksMissingReport` — fournisseur absent/quota (commande introuvable).
- `TestManagedIntegrationCeilingSuspensionNeverMasksMissingReport` — plafond (mission mise en pause entre deux passes).
- `TestManagedIntegrationWorkspaceLockOutranksMissingReport` — espace occupé (verrou `managedLock` détenu).

Un utilitaire de test partagé, `managedSimulateCrashAfterReportCommit`, rejoue exactement les opérations Git que `integrateManagedAttempt` effectue pour committer le rapport dans la copie gérée, positionne `managed_attempts.state='integrating'` avec `result_commit` renseigné (reproduisant un arrêt du processus juste après ce commit), puis supprime le rapport de la copie de travail. Chaque test vérifie ensuite que `task.Blocker` ne contient jamais « Rapport de tentative absent ».

### Preuve que les tests ne sont pas tautologiques

Avant d'appliquer le correctif définitivement, le fichier source a été remis à l'état d'origine (`git stash push -- managed_integration.go`) pendant que les nouveaux tests restaient en place, puis rejoué :

```sh
TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-r3-gocache \
go test -count=1 -run '^TestManagedIntegration(TrustsReportAlreadyInManagedCopy|FailedControlOutranksMissingReport|MissingProviderOutranksMissingReport|CeilingSuspensionNeverMasksMissingReport|WorkspaceLockOutranksMissingReport)$' -v .
```

Résultat observé sur le code non corrigé : **2 passed, 3 failed** — `TestManagedIntegrationTrustsReportAlreadyInManagedCopy`, `TestManagedIntegrationFailedControlOutranksMissingReport` et `TestManagedIntegrationMissingProviderOutranksMissingReport` échouent bien (elles exercent directement la branche corrigée) ; les deux autres passent déjà (elles documentent un comportement préexistant correct — voir tableau ci-dessus). Le correctif a ensuite été restauré (`git stash pop`) et revérifié vert.

## Contrôles réellement exécutés

| Commande | Résultat observé |
| --- | --- |
| `go build ./...` avec caches sous `/dev/shm` | code 0 |
| `go vet ./...` avec caches sous `/dev/shm` | code 0, aucune sortie |
| `go test -count=1 -run '^(TestManaged\|TestEvidenceContract\|TestCursorContract\|TestAutomaticValidation)' -v .` | code 0, 32 tests passés |
| `go test -count=1 -run '^TestManagedIntegration(TrustsReportAlreadyInManagedCopy\|FailedControlOutranksMissingReport\|MissingProviderOutranksMissingReport\|CeilingSuspensionNeverMasksMissingReport\|WorkspaceLockOutranksMissingReport)$' -v .` sur le code **non corrigé** | code non nul, **2 passed, 3 failed** (preuve ci-dessus) |
| `node tests/final_acceptance.cjs --case diagnostic` (`TMPDIR=/dev/shm GOTMPDIR=/dev/shm`) | code 0, 7 tests Go exécutés et passés, message d'acceptation affiché |
| `node tests/final_acceptance.cjs --case git` | code 0, message d'acceptation R2 inchangé |
| `node tests/final_acceptance.cjs --case proofs` | code 0, message d'acceptation R1 inchangé |

Commande exacte du cas ciblé de ce lot :

```sh
TMPDIR=/dev/shm GOTMPDIR=/dev/shm node tests/final_acceptance.cjs --case diagnostic
```

## Limites

- Le correctif couvre uniquement le chemin d'intégration gérée (`managed_integration.go`). Il ne touche pas au relais de handoff non géré (`conductor.go`, `provenReport`), qui a sa propre logique de comparaison temporelle déjà distincte et hors périmètre de ce lot.
- « Fournisseur absent/quota » est simulé par une commande de contrôle introuvable (`exec.CommandContext` échouant avec « executable file not found »). Aucun fournisseur IA réel n'a été invoqué dans ce lot ; le texte produit (« exécution impossible : … ») est la trace réelle produite par `runValidationControl`, pas un texte fabriqué pour le test.
- « Plafond » est démontré via la mise en pause de la mission entre deux passes (`s.setMission(w.ID, false)`), qui déclenche le même refus (`automaticValidationAuthorized`) que la logique de plafond de tentatives ailleurs dans le moteur ; le plafond numérique de tentatives (`PlanMaxAttempts`) lui-même n'est pas ré-exercé ici, il est déjà couvert par des tests préexistants hors de ce lot (`agents_store.go` / `agents_store_test.go`).
- La suite Go complète (`go test ./...`) n'a pas été relancée dans ce lot ; seules les suites ciblées listées ci-dessus l'ont été, conformément au périmètre étroit demandé. Le P0 avait déjà consigné les limites de sandbox (sockets Unix/TCP refusés) qui affectent `go test ./...` sans lien avec ce correctif.

## Handoff au responsable

Changements à relire : `managed_integration.go` (priorité du diagnostic), `managed_git_test.go` (5 tests nouveaux + utilitaire de simulation), `tests/final_acceptance.cjs` (cas `diagnostic` ajouté). Aucun contournement de contrôle, aucune modification de `.swarm/state.db`, aucun push. Risque principal : la branche `item.Result != ""` introduite lit désormais le rapport depuis `git show` plutôt que depuis le disque ; en cas d'échec de cette lecture (révision absente du dépôt bare géré), la tâche est explicitement bloquée avec « Intégration échouée : révision … absente de la copie gérée », jamais silencieusement ignorée. Prochaine action recommandée : revue indépendante du diff, puis exécution de `go test ./...` dans un environnement autorisant les sockets locaux pour confirmer l'absence de régression hors suites ciblées.
