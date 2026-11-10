# M25 — ressources cloisonnées et verdict complet des usages

17 octobre 2026, Codex seul : reprise `replan`, APEX borné, revue locale et RETEX.
Les résultats ne constituent ni revue indépendante, ni activation publique de
M20, ni garantie du moteur d'identité global. Le serveur utilisateur est conservé.

## Résultat

Deux réalisations dans les seuls fichiers mémoire : correction du contrat de
ressource privée M24 et évaluateur complet des usages du résumé extractif M19.

La ressource v2 inclut la tâche **source**. Deux snapshots de tâches du même
travail ont des ressources différentes ; un dialogue commun garde sa ressource
quand le destinataire change. Le protocole de proposition est aussi v2. Les
sources publiques M19, leurs IDs et leur paquet ne sont pas modifiés. Aucun
octroi v1 n'est converti en droit v2, aucune migration ou tolérance publique.

`evaluateMemorySummaryUsage` reconstruit les sources et l'attachement, vérifie
tous les droits du demandeur puis la lecture de chaque source transmise par le
destinataire. Son principal et son instance doivent correspondre à la proposition.
Les décisions destinataire ont leurs propres événements, séparés du demandeur.
La fixture vérifie treize contrôles demandeur et trois lectures destinataire,
sans droit de synthèse exigé de ce dernier, ni lecture du snapshot opérateur ou
d'une omission au titre de cette réception.

Un seul refus, même au dernier contrôle, renvoie une décision entièrement vide.
Le réemploi réévalue les deux acteurs avant de comparer l'intégralité du résultat.
Une preuve éditée, partielle, réordonnée, liée à un autre octroi ou à une autre
autorité est refusée. La validité du verdict est limitée par la première
expiration des octrois/sessions. Les causes de refus du noyau restent conservées.

Ce résultat reste **un verdict pur pour un instant fourni** : aucun octet libéré,
octroi émis, reçu consommé, journal durable ou effet appliqué. Le cadre courant,
la preuve d'identité et l'affectation réelle du destinataire doivent encore être
fournis par le contrôle protégé au point d'engagement. Aucun handler ni lanceur
public n'utilise cette brique. Les cachets ne deviennent donc pas « autorisés ».

## Défauts observés et replan

**M25-I01 — alias de source entre tâches.** Le tuple v1 utilisait l'ID fixe
`current-task-snapshot` et le travail, sans tâche. Le test
`TestMemoryUsageResourceSnapshotTaskBoundary` échoue avant correction : les
deux snapshots ont la même ressource. La série `red` conserve 1 nom FAIL et
l'échec de paquet, avec entrées inchangées. La différence du digest de contenu
ne corrigeait pas la portée du droit. Correction générale : tâche source dans
le tuple versionné, sans exception d'ID ou de mission ; le garde vérifie aussi
que les dialogues communs ne sont pas relocalisés sur la tâche destinataire.

**M25-I02 — fixture d'octroi incompatible avec l'immutabilité.** La première
série candidat passe, mais sa variante de changement d'octroi réécrit
`Instance.GrantID`. La revue constate que le contrat réel interdit cette mutation.
La fixture initiale reste dans `decision-test-candidate.go.txt`. Le garde final
ajoute un octroi au demandeur sans instance, conserve les anciens enregistrements,
avance la révision et appelle `validateAgentIdentityTransition` avant d'évaluer
le refus de l'ancienne décision. Aucune source identité ou règle d'autorité
n'est modifiée pour rendre la fixture acceptable.

**M25-I03 — lecture seule et fraîcheur des preuves distinctes.** Le premier
audit exige à tort qu'un fichier identité soit inchangé depuis la reprise.
L'autre chantier a extrait ses types/validateurs vers `internal/identitycontract`
avant le reproducer. Les séries rouge, candidat et finale utilisent toutes la
même nouvelle version, inchangée avant/après. La relecture confirme les champs,
tags, validateurs et opérations pertinents ; `go list` inclut bien le nouveau
paquet dans les manifestes. La correction de méthode compare les entrées
réellement testées, conserve la différence depuis la reprise et respecte
l'ownership. Aucune source identité n'est restaurée ni modifiée, aucun test
inchangé n'est rejoué pour cette différence antérieure. Preuve :
`read-only-contract-revalidation.json` et `audit-first-failure.json`.
Une lecture d'artefact avait aussi été demandée avant la fin de l'audit en cours ;
elle a constaté l'absence du résultat. La reprise attend la fin réelle avant
de consommer le verdict. Une coquille de rédaction locale est corrigée à la
relecture ; elle ne modifie aucun code ni résultat de test.

## Vérification et limites

Préimages et état sale : `evidence/m25-20261017/baseline.json`, `before/`,
`status-before.txt`. Le lanceur `run_checks.py` inventorie les dépendances locales
réelles via `go list`, lie sources Go, ressources embarquées, modules et recette
aux résultats, refuse les sorties existantes et conserve stdout/stderr structurés.
Il n'utilise aucun timeout Python et passe explicitement `go test -timeout=0`.
Ce choix applique la nouvelle instruction utilisateur ; les preuves M24 et
leurs anciennes commandes ne sont pas réécrites. Aucune limite fonctionnelle,
expiration de droit, politique opérateur ou règle de consommation n'est supprimée.

La série candidat donne 70 noms PASS et ses contrôles statique/compilation
réussissent. Après correction de fixture, la série finale donne **72 noms PASS,
zéro FAIL, zéro SKIP : 12 tests principaux et 60 sous-cas**. Elle couvre M24,
les nouveaux cas M25 et deux gardes existantes du parcours source/sélection
CLI/API. Toutes ses entrées sont inchangées ; aucun overlay n'est nécessaire
dans ce lot. La compilation du candidat est réutilisable si les seules entrées
modifiées ensuite sont les tests : la comparaison du manifeste final l'établit
dans `audit-final.json`. L'analyse statique finale est enregistrée séparément.

| Critère | État local | Preuve / limite |
| --- | --- | --- |
| M25-01 Lot complet | PASS | Treize contrôles demandeur, trois lectures destinataire ; événements uniques, aucune mutation des entrées. |
| M25-02 Refus sans préfixe favorable | PASS | Dernier droit demandeur ou destinataire retiré : cause exacte et décision vide, puis utilisation valide à nouveau. |
| M25-03 Correspondance et fraîcheur | PASS | Mauvais principal/instance/canal/projet, instance absente/révoquée, appelants périmés et horloge absente refusés. Cadres de fixture, sans attestation OS réelle. |
| M25-04 Réemploi | PASS | Contenu, sélection, tâche source, octroi, autorité, preuve partielle/éditée/réordonnée refusés ; transition de fixture validée par le moteur. |
| M25-05 Expiration et refus courants | PASS | Révocation, expiration et sessions périmées prises en compte ; réauthentification avant réemploi et fin de validité côté destinataire vérifiées. Aucun effet distribué. |
| M25-06 Livraison | Voir audit final | Noms, entrées, formatage, diff, liens, statique, compilation et documentation. Revue de l'auteur, aucune acceptation indépendante. |

Les commandes, dates et sorties originales figurent dans `red-*`, `candidate-*`,
`final-*`. Les fixtures sont locales, sans base de mission réelle ni fournisseur.
Pas de nouvel appel de modèle, de délégation, de mission native ou de message
inter-chat. Pas de changement de synchronisation : contrôle `-race` non requis
pour ces fonctions pures, sans état partagé ajouté. Aucun écran/API n'est changé ;
les recettes navigateur M23 ne sont pas relancées. Schémas FR/EN mis à jour dans
le Markdown ; leur rendu graphique n'est pas vérifié dans ce lot. Les nouvelles
consignes ne sont pas présentées comme transmises à un processus déjà lancé.

## RETEX et condition de reprise

Une décision liée au contenu exact ne remplace pas une ressource correctement
cloisonnée. Cette leçon est transmise par le garde à deux tâches et le contrat
d'usages courant : source et destinataire sont deux dimensions distinctes.
Une fixture verte doit aussi respecter les invariants de transition du vrai
producteur ; le garde de changement d'octroi utilise désormais ce validateur.

Les paquets preuve et journal d'enrôlement du chantier identité ont progressé,
sans fournir encore le raccord public et le point d'engagement. Le choix d'une
frontière indépendante (compte local distinct ou machine séparée), du principal
humain initial et le bootstrap vérifié restent à décider/réaliser là-bas.
M20-U01–U10 demeurent ouverts pour la conformité applicative : attribution
propriétaire/auteur, identité et affectation du destinataire, autorité avant plan,
engagement courant et chaîne décision/application/réception.

Propriétaires : mémoire pour la persistance et les consommateurs, identité pour
son service/autorité protégés, opérateur pour frontière et principal initial.
Condition de reprise publique : ces contrats utilisables et vérifiés, puis garde
du temps et des sources au point réel d'engagement, sans réentrée SQL ni droits
implicites. Les codes privés de refus devront alors recevoir leur présentation
FR/EN. Le présent lot n'enrôle aucune identité, ne redémarre pas le serveur et
ne clôt pas IA4/IA5 ou le RETEX global.
