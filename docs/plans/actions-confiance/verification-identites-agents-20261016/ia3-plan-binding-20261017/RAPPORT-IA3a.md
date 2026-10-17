# IA3a — autorité liée au contrat courant du plan

17 octobre 2026, Codex seul, `replan` et APEX, auto-revue locale non indépendante.
Base : `4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, branche
`codex/clear-product-method-names`, changements locaux préexistants préservés.

**Composant interne vérifié ; admission publique non réalisée.** Une nouvelle
course et un défaut de test antérieur sont conservés avec leurs originaux.

## Résultat et portée

La source durable compare désormais sa portée au contrat effectivement enregistré
dans le travail : empreinte de projection et génération persistée. Un permis
contenant une empreinte arbitraire, un travail sans plan ou un contrat amendé
ne produit pas d'autorité. Modifier puis restituer l'ancien contenu conserve
une génération plus récente, y compris après réouverture.

La présence d'un plan enregistré ne prouve aucune adoption humaine attestée.
Les permis et checkpoints restent émis par des fixtures. Le consommateur
n'établit pas encore l'appartenance exacte d'une création au plan, le contenu
résolu du lancement ou son accord humain. Aucun lanceur n'est raccordé.

Une cage réelle permet de vérifier le refus après amendement. Une autre recette
reproduit une décision sur un plan devenu ancien pendant l'attente runtime,
puis vérifie la sérialisation corrigée. Aucun effet métier n'est lancé.

## Contrat, stockage et choix

Quatre fichiers nouveaux : `agent_identity_plan_linux.go`,
`agent_identity_plan_schema_linux.go`, `agent_identity_plan_linux_test.go` et
`agent_identity_plan_freshness_linux_test.go`. Sources préexistantes modifiées :
`model.go`, `store.go`, lecteur/publiant et fixtures du registre d'identité.
Les trois fichiers de tests de revue d'un essai rejeté sont restaurés exactement.
Les lanceurs et les sources mémoire restent identiques à la base de cette tranche.
[Base et empreintes](baseline.json), préimages et sources avant/après conservées.

Le schéma **37** ajoute `agent_identity_plan_heads` : travail, génération entière
positive, digest et version de projection. La migration sauvegarde le stockage
existant, conserve les corps Work et initialise les stamps à 1, sans publier
de droit. Les triggers d'insertion/modification de `works` maintiennent le stamp
dans la transaction du travail. Une fonction SQLite pure est enregistrée une fois
avant ouverture des connexions. Une incohérence du stamp précédent refuse
l'écriture ; un dépassement de représentation entière annule toute la mutation.
Ce compteur technique n'est pas une politique de ressources à paramétrer.

Options examinées : utiliser la révision entière du travail, ou distinguer le
contrat de ses résultats runtime. La première invaliderait aussi les événements
d'exécution. La projection **v1** retient les définitions, instructions, rôles,
propriétaires, dépendances, hiérarchie, budgets, profils et champs inconnus ; elle
retire les champs de résultats/exécution identifiés dans le code. Les listes
restent ordonnées ; les clés d'objets sont canonisées récursivement, avec
`UseNumber` pour conserver les grands entiers. Changer une entrée conservée peut
donc invalider une portée de manière conservatrice ; ce coût n'est pas mesuré.

Compter les changements uniquement lors d'une publication était insuffisant.
Entre un helper à imposer à tous les écrivains et une frontière transactionnelle
SQLite commune, les triggers sont retenus : un amendement puis retour en arrière
augmente deux fois la génération, sans réactiver l'ancienne portée. Un rollback
de transaction annule le changement et son stamp ensemble.

`readAgentIdentityNextPlanScope` propose le scope courant en lecture seule, sous
checkpoint de tête existante. Aucun rôle métier, import ou résultat de cette
lecture ne construit un permis. La publication normale réserve SQLite puis
compare son permis au plan courant avant l'intention durable. La reprise exacte
peut finir son historique après amendement du plan ; l'usage de cet historique
reste refusé et les anciens octrois restent historiques.

## Course découverte et frontière de décision

La première version libérait SQLite après lecture puis attendait le verrou
runtime. Les écritures Work n'acquièrent pas Trust : le plan pouvait changer
durant cette attente. Le contre-exemple observe le consommateur dans `withCaller`
via une pile Go locale bornée, fait engager l'amendement par un second Store,
puis libère runtime. **Avant : amendement engagé et décision positive ancienne.**

Options : imposer un autre verrou à tous les écrivains, ou conserver une
réservation d'écrivain SQLite pendant la décision pure. La seconde est réalisée :
**Trust partagé → écrivain SQLite réservé → source courante → runtime → temps
frais/évaluation pure → COMMIT**. Le callback fermé n'appelle pas Store et les
méthodes runtime appelées n'accèdent pas à SQLite. Une erreur de COMMIT ne
retourne aucune décision positive. Les inspections seules restent en transaction
de lecture. Cette décision ne consomme toujours pas un droit et n'engage aucun
lancement ; les futurs consommateurs doivent respecter cet ordre et revalider
à leur propre frontière d'effet. Le coût en concurrence n'est pas mesuré.

Après correction, l'amendement attend la décision sérialisée, puis l'ancienne
portée est effectivement refusée. La fenêtre de 100 ms est celle de la fixture,
pas une politique opérateur ni une preuve fondée sur le seul temps écoulé :
attribution/décision, engagement ultérieur et refus après amendement sont assertés.

## Preuves et contrôles

| Contrôle | Résultat / original |
| --- | --- |
| Sélection ciblée | **232 noms PASS / 232**, 72 tests racine, aucun FAIL/SKIP : [commande](tests-final-run.json), [JSONL](tests-final.jsonl), [résumé](tests-final-summary.json) |
| Nouveau plan | **32 noms PASS / 32**, inclus dans les 232, pas 32 attaques indépendantes |
| Concurrence | **84 noms PASS / 84 sous `-race`**, aucun FAIL/SKIP ; résultats exacts et entrées : [commande `-race`](race-final-run.json), [JSONL](race-final.jsonl), [résumé](race-final-summary.json) ; sous-ensemble rejoué, pas à additionner |
| Statique et compilation | `go vet ./internal/engine`, `go build ./cmd/swarm` : [codes zéro](static-build-final.json), [binaire isolé non déployé](binary-final.json) |
| Candidat | **1 056 entrées** Go/tests/ressources embarquées : [empreintes](inputs-final.json). Aucune modifiée pendant les contrôles finaux ; `source-final/` conserve les octets des sources d'identité et du stockage |

Les 181 contrôles de la tranche précédente sont rejoués, avec 32 nouveaux
contrôles du plan et 19 noms de dépendances. Neuf sous-tests mémoire portent
la version dans leur nom : les variantes -1/0/+1 sont [corrélées](test-name-correlation.json)
entre schémas 36 et 37, avec leur source inchangée et les noms originaux conservés. L'élargissement couvre
les écrivains concernés : adoption classique, conversion de préparation,
décision hiérarchique, lancement, migration/inspection, archive, révision de
contrat et contexte mémoire. Pas de source mémoire modifiée ni de suite globale
revendiquée. Aucun modèle distant, revue indépendante ou déploiement.

Les variantes couvrent absence/altération, définition/rôle/propriétaire,
instructions/dépendances/réservation/profils, résultats runtime, champs inconnus,
grands entiers, hiérarchie réelle, conversion, génération après réouverture,
annulation et dépassement entier sans mutation partielle. Le publiant relit
le plan après son attente SQLite. Les deux reprises après intention/COMMIT
terminent l'histoire sans réactivation après amendement. La cage réelle vérifie
révocation d'octroi et refus du plan amendé ; aucune ligne Agent n'est créée.

| Découverte | Originaux, action et état |
| --- | --- |
| IA3-P01 : plan non vérifié | [Trois échecs initiaux](plan-before.jsonl), sources `source-before/`. Lecture depuis le stockage et refus sans intention ; vérifié localement. |
| IA3-P02 : retour à l'ancien contenu | [Échec](generation-before.jsonl), sources `source-before-generation-fix/`. Génération transactionnelle, reprise/restitution et rollback vérifiés. |
| IA3-P03 : canonisation incomplète | Même exécution rouge ; les RawMessage conservaient l'ordre imbriqué. Canonisation récursive sans float64, variantes entières vérifiées. |
| IA3-P04 : témoin du plan | Analyse du modèle de restauration ; exigence ouverte. Le témoin indépendant doit couvrir le watermark du plan, pas seulement une tête d'identité inchangée. |
| IA3-P05 : plan changé pendant runtime | [Échec observé](freshness-before.jsonl), [trois gardes après](freshness-after.jsonl), sources `source-before-freshness-fix/`. Réservation SQL pendant décision, reprise dans les contrôles finaux. |

Schémas actuels : [FR](contract-fr-4.svg), [EN](contract-en-4.svg),
[rendus et provenance](diagrams.json). Les deux nouveaux PNG ont été inspectés
visuellement. Six diagrammes précédents sont réutilisés à sources/empreintes
identiques, avec leur inspection historique conservée. Contrat FR/EN et dossier
scientifique reportent ces résultats sans revendication d'originalité.

## Diagnostic des contrôles et limites conservées

La recette d'attente initiale déréférençait `commandFailure(nil)`, puis sondait
un verrou partagé de manière répétée, pouvant empêcher elle-même la prise
exclusive non bloquante du publiant. Les originaux sont conservés dans
`generation-first-after.jsonl` et `tests-expanded-before-recovery-diagnosis.jsonl`.
Le pilote utilise maintenant un code d'erreur nil-safe et observe `DB.WaitCount`
avant un seul sondage du verrou. L'attente et le refus frais sont vérifiés.
L'ancien test de version écrivait littéralement 37, devenu courant ; il utilise
désormais `schemaVersion+1`, avec la même assertion `storage_version_conflict`.
Les diagnostics et préimages conservent ces corrections de méthode.

Le motif `TestPlan.*` sélectionnait aussi tous les `TestPlanning*` et a révélé
une panique préexistante de `TestPlanningReviewRetryDeliversSupervisorCorrectionAfterReopen`.
L'[overlay des sources initiales](baseline-review-retry-run.json) reproduit
la [même panique](baseline-review-retry.jsonl) sans changer le checkout.
La fixture historique déclenche un remplacement complet de méthode, alors que
le test attend une reprise sur simple consigne. Un essai de fixture courante
confirme que cette reprise est désormais volontairement refusée ; l'essai est
rejeté et ses trois fichiers restaurés à l'octet. Aucune protection de reprise
assouplie. [Diagnostic et condition de reprise](review-retry-diagnosis.json).

La sélection finale vise `TestPlan[A-Z].*` et les écrivains hiérarchiques nommés,
conformément au contrat modifié. **Le défaut de test antérieur reste ouvert**,
propriétaire Codex/chantier moteur-reprise : réaligner ses anciennes attentes
sur la reprise protégée, puis vérifier réception, décision, application et effet.
Les résultats ciblés ne constituent pas une suite globale verte. Les premières
36 puis 231 réussites et les exécutions interrompues restent historiques.

Le premier index comparait les noms de sous-tests littéralement, sans tenir
compte du paramètre de version : son assertion et son script sont conservés.
La corrélation relative de ces neuf variantes remplace cette hypothèse, sans
modifier de source mémoire ni de résultat.

Les recherches initiales avec des noms supposés et deux contextes de patch
incorrects ont été remplacés par la lecture des chemins/lignes réels ; aucun
patch partiel ni preuve originale effacée. Ces incidents de méthode ne sont pas
des attaques supplémentaires ou des défauts de production démontrés.

## Suite et transmission

Les stamps/triggers ne protègent pas contre l'administrateur hôte, le même UID
non confiné ou une image SQLite hostile arbitraire. Restaurer physiquement toute
la base peut restaurer aussi son ancien stamp ; un checkpoint couvrant seulement
la tête d'identité inchangée ne constate pas les amendements non publiés dans
cette tête. Ce contrôle exige un témoin indépendant actuel du plan, encore absent.
Propriétaires : Codex pour le contrat, opérateur pour enrôlement/protection.

Restent ouverts : émetteur humain du permis, témoin courant, appartenance au plan
et contenu résolu, consommation/réservation/départ incertain, confinement et tous
les lanceurs publics, droits mémoire, IA5 et RETEX global. **F01–F07 et les états
IDA ne sont pas requalifiés.** La documentation transmet les préconditions aux
prochains travaux ; elle ne prouve ni leur réception ailleurs ni leur exécution.

Les apprentissages SCI-ID-16–20 sont capitalisés dans le dossier scientifique.
La leçon de fraîcheur porte sur les données et le point de décision, en plus
de l'horloge ; la leçon de génération porte sur chaque amendement persisté.
[Auto-revue locale finale](review-local-final.md), [index et empreintes](checks-final.json).
