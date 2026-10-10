# Défauts du moteur relevés par le banc de facturation

Statut : relevé du 5 octobre 2026, à transmettre aux mainteneurs du moteur. Les références
`fichier:ligne` portent sur `main` au commit `53f2564` et viennent de
[`observations.md`](observations.md), où figurent les observations détaillées. Elles sont à
revérifier avant correction : deux passages de `observations.md` situent `automaticCorrection`
à `dispatcher.go:55-77` puis à `dispatcher.go:203-214`.

Le banc ne modifie pas le code Go. Chaque défaut est contourné ou mesuré tel quel ; la colonne
« Effet sur le banc » dit comment.

| N° | Défaut | Invariant touché | Gravité pour un flux financier | Effet sur le banc |
|---|---|---|---|---|
| D1 | Validation jamais reprise en espace de travail propre | aucun (blocage) | moyenne : mission bloquée, sans paiement | contourné : espace partagé |
| D2 | Dépendance non fraîche sautée sans motif ni événement | I3 (observabilité) | moyenne : arrêt sûr mais muet | mesuré : F4e, arrêt attribué au moteur par lecture de l'état |
| D3 | Indisponibilité d'un service traitée comme un échec métier | I6 | haute : régénère le livrable | mesuré : F5, impayés sans absorption |
| D4 | Aucune reprise automatique après arrêt brutal (code 137) | aucun (choix de sûreté) | faible : la décision revient au responsable | contourné : le responsable scripté émet `retry` |
| D5 | SQLITE_BUSY non réessayé à l'application d'une décision | I1 (disponibilité) | moyenne : échec durable sur un verrou transitoire | mesuré : ERREUR possibles sous F4e |
| D6 | Couverture des exigences vérifiée à la clôture, pas avant l'effet | I3 (étendue) | haute avec un responsable réel : effet possible sur un candidat non contrôlé | non exercé : responsable scripté ; prévu sous F12 |

## D1 — Validation jamais reprise en espace de travail propre

**Observé.** Une tâche exécutée dans son propre sous-répertoire reste `submitted` : aucun
contrôle n'est exécuté.

**Cause.** La remise lit le rapport dans l'espace de la tentative (`conductor.go:148-171`) et
publie un chemin relatif à la racine (`prepare/docs/prepare.md`). La validation, retenue jusqu'à
la revue indépendante (`independent_review.go:53-54`), est reprise par
`resumeAutomaticValidations` (`mission.go:257` → `automatic_validation.go:413`), qui cherche le
rapport avec `provenReport` sur la racine ; `taskReports` ne parcourt que `racine/docs`
(`review_dialog.go:24`).

**Masquage.** Le harnais de référence écrit le rapport aux deux endroits
(`tests/organized_coordination_process.py`).

**Piste.** Reprendre la validation sur le chemin publié par la remise, ou chercher le rapport
dans l'espace de la tentative comme le fait la remise. Ajouter un test sans double écriture.

## D2 — Refus silencieux d'une dépendance non fraîche

**Observé (F4e).** Le lot de `prepare` est modifié après acceptation. `settle` n'est jamais
lancée, ce qui est correct, mais le répartiteur n'émet que « aucune tâche candidate » ;
`prepare` reste affichée `accepted` et le responsable ne reçoit aucun événement.

**Cause.** `dispatcher.go:144` : `case !in.depsReady[t.ID]: continue`, sans journal.

**Piste.** Journaliser le motif, émettre un événement au responsable et marquer la preuve de la
dépendance comme périmée dans l'état affiché. L'arrêt est sûr ; seul le diagnostic manque.

## D3 — Indisponibilité traitée comme un échec métier

**Observé (F5).** Le contrôle `check_lot` reçoit six 503 et sort en code 3. Le reçu passe
`blocked`, « contrôle en échec (code 3) » (`automatic_validation.go:196-198`), puis la
correction automatique relance `prepare`, qui régénère le lot.

**Cause.** `automaticCorrection` (`dispatcher.go`, catégorie `recoveryBusiness`) ne distingue
pas un échec du contrôle d'une indisponibilité de l'environnement ; `plan_max_attempts` vaut 2
(`planning.go:562`).

**Pourquoi c'est grave.** Sur un flux de paiement, régénérer le livrable parce qu'un service est
indisponible produit un nouveau candidat à valider, au lieu d'attendre le rétablissement et de
revalider le même. C'est l'écart avec I6 (reprise justifiée) décrit en section 5.3 de l'article.

**Piste.** Réserver un code de sortie de contrôle à l'indisponibilité (par exemple 75,
`EX_TEMPFAIL`) et le traiter par attente bornée et nouvel essai du contrôle, sans nouvelle
tentative de l'agent.

## D4 — Aucune reprise automatique après arrêt brutal

**Observé (F3).** Un agent tué (code 137) donne une tentative `failed`, une tâche `blocked`,
sans remise (`agents_process.go:387-392`, `agents_store.go:915-935`, `conductor.go:68-70`). Le
répartiteur journalise « cause inconnue ou signal absent ; aucune reprise automatique sûre »
(`dispatcher.go:150-155`).

**Lecture.** C'est un choix de sûreté défendable : la reprise relève d'une opération `retry`
explicite du responsable (`planning.go:564-575`). À documenter comme tel plutôt qu'à corriger.

## D5 — SQLITE_BUSY non réessayé à l'application d'une décision

**Observé (F4e, exploration n° 2).** 2 exécutions sur 12 en ERREUR, dont une avec le motif
« planification suspendue : Proposition non appliquée : database is locked (5) (SQLITE_BUSY) »,
devenu `planning.failure` durable (`planning_runner.go:183`).

**Cause.** La boucle de `planning_runner.go:135-152` ne réessaie que `revision_conflict`. Le
relais de remise (`conductor.go:33`, `:42`) et les échanges (`agent_exchange.go:216`) réessaient
pourtant SQLITE_BUSY.

**Seconde voie (campagne complète, 6 octobre 2026).** Le vérificateur indépendant échoue lui
aussi sur SQLITE_BUSY : `planning.reviewer.failure` portant « database is locked (5)
(SQLITE_BUSY) » (moteur : `independent_review_runtime.go`, chemin exact à localiser). À mi-campagne,
toutes les exclusions de S sont sous F4e et sur ces deux voies : clé `none`, 8 ERREUR et
1 DÉLAI sur 100 ; clé `attempt`, 5 ERREUR et 2 DÉLAI sur environ 60 exécutions faites. Aucune
exclusion dans les autres cas.

**Piste.** Réessayer SQLITE_BUSY avec attente bornée dans la boucle d'application et dans le
vérificateur indépendant, comme les autres chemins.

## D6 — Couverture des exigences vérifiée à la clôture, pas avant l'effet

**Constaté à la lecture du code (6 octobre 2026), non observé en exécution.** La clôture d'un
périmètre refuse une exigence sans tâche acceptée à preuve fraîche (`planning.go:603-620`), et
la création d'une tâche exige au moins une exigence possédée (`planning.go:471-480`). Rien
n'impose en revanche qu'une tâche à effet (le règlement) ne parte qu'après l'acceptation des
tâches qui portent les contrôles du candidat : c'est au responsable de déclarer la dépendance.

**Pourquoi c'est grave.** Avec un responsable réel, une erreur de planification (contrôle du
lot rattaché à une autre tâche, dépendance oubliée) laisse partir le règlement sur un lot non
contrôlé ; la clôture est refusée ensuite, le paiement déjà fait. Dans le banc, seul l'exécutant
l'empêche (E3 : il ne paie que la remise acceptée de `prepare`).

**Piste.** Permettre de déclarer, au niveau du travail et non du responsable, qu'une exigence
doit être acceptée avant le départ de toute tâche portant une exigence donnée (par exemple
`req-1` avant `req-2`), vérifié par le répartiteur.


## Complément du 8 octobre — attribution des 32 exclusions historiques

L’analyse complète [des 32 cas](analyse-32-exclusions-20261008.md) précise les observations
intermédiaires de D5 : 27 ERREUR SQLite sont explicites (14 décisions, 13 revues), dont 26
sous F4e et une sous F7. Les cinq DÉLAI sous F4e n’ont aucun événement SQLite dans leur
historique public ; ils conservent un claim de 120 secondes sans décision suivante.
Leur cause initiale demeure inconnue. Le banc considère le holder comme une activité
jusqu’au délai global, lui aussi de 120 secondes. Les attributuer toutes à D5 n’est pas justifié.
Les archives et critères historiques sont conservés ; aucune nouvelle campagne n’a été lancée.
