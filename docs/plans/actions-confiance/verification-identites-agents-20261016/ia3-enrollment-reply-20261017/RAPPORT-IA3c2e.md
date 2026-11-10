# IA3c.2e — lecture versionnée, aucun repli implicite

17 octobre 2026 — **PASS du cadre de lecture local ; service réel ouvert.**

ActiveReply exige maintenant `schema_version: 1`. Le journal l’émet et un décodeur
commun strict le vérifie avant l’interprétation de l’autorité. Un cadre absent,
inconnu ou malformé est refusé, même signé par la clé épinglée. Aucune migration
de SQLite ou des événements signés ; aucune réécriture des preuves historiques.

## Décision et portée

Le [plan](../../identites-agents-replan.md) compare maintien du décodage non
versionné et cadre explicite commun. Le second est retenu pour I2c-06. Les
contrats [FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md),
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) et
[cible](../../identites-agents-enrolement-IA3c2.md) décrivent les trois champs,
les refus et la rupture du prototype. Support explicite `{1}`, sans négociation
automatique ni downgrade. La sélection et l’interopérabilité de vrais pairs
restent à vérifier avant client externe ; I2c-06 est donc partiel au déploiement.

Codex seul, même checkout. Quatre sources existantes changent : journal,
consommateur d’autorité et leurs deux fixtures d’autorité/demande. Trois nouveaux
fichiers : codec et tests du journal, contre-exemple au niveau de l’autorité.
Les sources moteur, les chantiers mémoire et les processus en cours ne changent
pas. Aucun service, compte, clé durable, endpoint, commit ou déploiement créé.

## Contre-exemple avant correction

[version-red-02](version-red-02-run.json) exécute dix noms : huit PASS, deux FAIL
(parent et sous-cas `absent`). Le [cas original](version-red-02.jsonl) signe un
ancien cadre sans version pour un challenge neuf : le consommateur retourne
réellement un document valide. Le [source du test](red-reply_version_test.go.txt)
est conservé sous son empreinte avant correction. Les huit autres variantes
étaient déjà refusées par la comparaison canonique ; ce ne sont pas huit nouvelles
protections acquises. Les deux noms FAIL représentent un seul contre-exemple.

Cette observation décrit une absence de sélection explicite dans un prototype
sans client externe, pas une élévation de privilèges publique démontrée. La
première tentative `version-red-01` référençait le futur symbole ErrReplyVersion
et ne compilait pas : zéro test exécuté. Retrait de cette assertion dépendante,
puis reproduction avec les interfaces réellement présentes ; la première sortie
reste conservée et n’est jamais comptée comme preuve comportementale.

## Résultats et limites

| Critère | Résultat observé | Preuve |
| --- | --- | --- |
| Journal et codec | 62 noms PASS | [normal final](normal-02-run.json) |
| Autorité et refus authentiques | 73 noms PASS | Même lot, erreurs et causes vérifiées |
| Demande complète | 62 noms PASS | Même lot, fixture de substitution mise explicitement en version courante |
| Ensemble normal | 197 noms PASS, 0 FAIL/SKIP | Trois paquets effectivement collectés |
| Même ensemble race | Mêmes 197 noms, aucune course signalée | [race-02](race-02-run.json) |
| Analyse/compilation des paquets | PASS | [vet-02](vet-02-run.json), [build-02](build-02-run.json) |
| Entrées locales | 16 stables avant/après chaque contrôle final | [source-final](source-final.json) |
| Compilation application | PASS, 1 083 fichiers stables durant le contrôle | [application-build-01](application-build-01-run.json), binaire temporaire non déployé |
| Source moteur | 3 fichiers de référence inchangés | [préservation moteur](engine-source-preservation.json), aucun test moteur relancé |
| Autres sources précédentes | 9 inchangées et 4 changements prévus | [préservation](preservation.json), diffs de reprise |
| Schémas FR/EN | 4 rendus, 2 PNG inspectés | [rendus](diagrams.json), [inspection](visual-inspection.json) |

Les comptes incluent parents et sous-cas, sans équivalence avec autant d’attaques.
Tous les tests ont `-timeout=0` ; aucun watchdog global arbitraire. Après le build,
seules des assertions de test du consommateur ont été renforcées, sans changement
de production. Elles sont exécutées dans le lot final ; elles ne sont pas compilées
dans l’application. Cette réutilisation est explicitée dans
[application-evidence-reuse](application-evidence-reuse.json). Aucun source n’est
édité pendant les vérifications. Le graphe de build référence aussi des fichiers
partagés non archivés intégralement ; reproduire exige leurs versions exactes.
Pas de suite globale, recette navigateur, fournisseur réel, revue indépendante,
version servie ou acceptation globale qualifiée.

## Contrat effectif

SignActive écrit la version sous la réservation du journal ; la signature couvre
le cadre entier, donc cette version. DecodeActiveReply est l’unique décodeur de
lecture : format canonique exact, tête positive, identifiant selon les règles du
journal, séquence de création positive et couverte par la tête. Le corps est en
base64 JSON et jamais null ; une chaîne vide est valide comme donnée opaque,
sans constituer une autorité. Copies des corps décodés indépendantes vérifiées.

Les variantes sans version, zéro, future, négative, null, chaîne, dupliquée,
variante de casse et champ supplémentaire sont testées sous signature authentique.
Le codec teste aussi tête, ID, génération, corps null, whitespace et trailing.
Chaque refus retourne un résultat nul ; le décodage ne mute aucun journal.
Le consommateur conserve ErrReplyVersion comme cause via errors.Is tout en
conservant sa catégorie ErrDocument. Les fixtures destinées à vérifier les autres
refus indiquent désormais schema_version 1, afin qu’un succès de test ne masque
pas une sortie trop tôt pour version absente.

DecodeActiveReply vérifie seulement le format : signature/challenge doivent être
vérifiés avant, puis contexte indépendant, document et dates après. Un frame
version 1 signé ne prouve pas l’état courant de la source, la fraîcheur d’une
session ou un droit d’action. Le corps opaque n’est pas réinterprété dans le
journal. La règle d’identifiant est partagée entre événements et lectures sans
changer ses acceptations. Aucun timeout, TTL, budget ou plafond métier nouveau.

## Améliorations et suite

| Découverte / origine | Valeur et périmètre | Propriétaire / état / critère |
| --- | --- | --- |
| I2e-01, I2c-06 et cadre antérieur accepté | Éviter une sélection implicite du format | Codex ; version/décodeur commun réalisés, ancien cadre authentique refusé |
| I2e-02, erreurs traversant le consommateur | Conserver la distinction version/forme/document | Codex ; cause chaînée et testée avec errors.Is |
| I2e-03, fixtures de substitution existantes | Éviter un test vert pour la mauvaise raison | Codex ; version courante explicitée avant les cas de refus de contenu |
| I2e-04, premier test lié à un futur symbole | Tester d’abord l’interface existante | Codex ; erreur de collecte diagnostiquée, contre-exemple réel conservé en 02 |
| I2e-05, premier client externe futur | Vérifier support commun et absence de downgrade entre pairs | Codex/opérateur ; local prêt, protocole réel et sélection à vérifier avant activation |

Les anciennes réponses historiques ne sont pas réécrites ou acceptées par fallback.
Le format du journal signé et la demande complète ne changent pas. Le choix de
frontière (compte distinct local ou machine séparée) est demandé à l’opérateur,
sans installation déduite d’une option présélectionnée. Principal initial, racine
protégée, témoin indépendant durable et point de linéarisation restent requis.
F01–F07, IDA, IA3 public, IA4/IA5 et RETEX global restent ouverts. La version de
protocole est une technique classique ; aucune nouveauté scientifique ni gain
comparatif n’est démontré par ces tests locaux.

## Clarification de la frontière et contrôle documentaire

À la demande de l’utilisateur, le plan d’enrôlement compare les trois choix et
leurs risques : local confiné, autorité sur machine/administration séparées,
préparation sans activation. Clarification I2e-06 : protéger la clé n’autorise
pas une demande ; l’autorité doit refuser les opérations interdites même présentées
par un pair authentifié. Ce contrôle réel et le choix opérateur restent ouverts.
Les contrats FR/EN transmettent cette limite ; aucun nouveau schéma n’est requis
pour ce tableau de décision, les schémas de format restent inchangés.

Un patch combiné refusé avant mutation a été diagnostiqué puis remplacé par un
patch limité au contexte relu ; [preuve](documentation-patch-recovery.json).
[Contrôles finaux](checks-final.json) : liens, empreintes des sources, archive,
inspection des rendus et whitespace. Aucun résultat Go n’est étendu au déploiement.
