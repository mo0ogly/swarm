# Résultats M32 — fraîcheur des appels structurés

17 octobre 2026, Codex seul. Incrément vérifié localement sur sources Go stabilisées après coordination
autorisée avec le chantier identité et ressources UI explicitement figées.
MEM-P1 reste ouvert ; aucune acceptation globale ou indépendante revendiquée.
[Plan et décisions](INCREMENT-MEMOIRE-28.md),
[preuves](evidence/m32-20261017/),
[lecture et reprise des reçus](evidence/m32-20261017/README.md),
[matrice et reprise](evidence/m32-20261017/AUDIT-REPRISE.md).

## Comportement réalisé

Les six chemins de revue/planification relisent le Work puis exécutent leur
contrôle spécifique avec une erreur, au lieu d’effacer la cause dans un booléen.
Le lanceur vérifie avant démarrage, au poll existant et après une fin normale,
avant de rendre une réponse exploitable. Les entrées booléennes historiques
restent adaptées ; un contrôle absent est refusé sans panic. Le contrat typé
conserve errors.Is/As ; un cooldown simultané ne remplace plus la cause initiale.

Une erreur de fraîcheur ne fournit pas de réponse exploitable. Si le processus
avait commencé, le lanceur remet ses observations d’usage au recorder existant ;
ce n’est pas une preuve que toute panne d’écriture de ce recorder est traitée.
Réservations, activations et historique ne sont ni effacés ni remboursés par ce
contrôle. Les politiques Admin et délais existants restent inchangés.

Une fin de lot remplacé vérifie maintenant longueur et identité dans sa mutation
de résultat : aucune indexation d’une liste vide, aucun rétablissement implicite
de l’ancien lot ou publication du candidat. Le compteur persistant d’appels de
la fixture reste incrémenté après annulation ; son refus rejoint les lectures
publiques de la CLI et du snapshot HTTP local.

Le lecteur partagé de pause contrôle la représentation booléenne et conserve
la cause dans Unwrap, avec un message public fixe qui ne recopie pas la valeur
invalide. Il sert aux cinq callbacks qui consultaient la pause et aux transactions
claim/decide/judge/renew, début/journal/finalisation de fragments concernées.
Absence historique signifie toujours non pausé. Le diagnostic explicatif garde
son contrat : aucune nouvelle politique de pause n’est ajoutée à ce chemin.

Le dernier contrôle ne réserve pas l’état futur. Les mutations de publication
conservent leurs propres validations et droits ; le contrôle ne fournit ni
identité protégée, ni propriété/auteur attestés, ni droit d’usage mémoire M20.

## Contre-exemples et erreurs de méthode

| Reçu original | Observation réelle avant correction |
| --- | --- |
| red, deux cas | Réponse immédiate acceptée après un seul contrôle ; contrôle absent provoquant un panic. Source exacte sauvegardée dès la série rouge. |
| red-batch-native | Processus local à barrière, lot remplacé par le Store : panic index 0 d’une liste vide après annulation. |
| red-pause | La transaction refuse déjà la pause invalide, mais son erreur affiche la valeur injectée. Aucun départ non autorisé prouvé par ce cas ; le défaut confirmé est la confidentialité du diagnostic. |
| red-batch, fixture insuffisante | L’écriture SQLite externe est refusée car la connexion ne dispose pas de swarm_identity_plan_digest_v1. Aucun trigger retiré ; reprise par le Store enregistré. Ce refus n’est pas compté comme reproduction du panic. |
| candidate, mesure insuffisante | 140 noms PASS mais assertion du nouveau test sur managedReviewCalls incorrecte : ce helper lit le fichier de l’ancien double, que le nouveau script n’écrit pas. Reprise sur Reviewer.Calls persistant ; aucune consommation modifiée pour passer le contrôle. |
| final / refresh / race, preuve de version insuffisante | 187 noms normaux PASS à chaque série, 34 sous race, vet/build réussis, mais reçus FAIL : deux tests puis sept sources Go identité changent pendant les séries. Aucune source M32 modifiée. Originaux conservés, coordination avec le propriétaire puis requalification stable distincte. |

Quatre contre-exemples produit précis sont conservés. Les quatre instantanés
rouges et la source historique de l’assertion candidate sont liés par empreinte
à leurs reçus ; les échecs de fixture, de mesure et de version restent distincts.

## Vérifications et critères

| Vérification | Résultat réellement exécuté |
| --- | --- |
| Normal, disk-normal | 187 noms PASS : 108 parents et 79 sous-cas ; aucune erreur ni aucun skip |
| Race, qualified-race | 23 noms PASS : 15 parents et 8 sous-cas, déjà présents dans la série normale |
| Livraison locale | go vet moteur et compilation cmd/swarm PASS ; binaire de fixture identifié par SHA-256 |
| Entrées | Manifestes avant/après identiques pour les deux reçus ; 14 sources M32 et sept sources Go identité concurrentes coordonnées |
| Documentation | FR/EN, deux Mermaid rendus et inspectés, liens locaux et formats vérifiés |

Les deux reçus qualifient 187 noms distincts, dont neuf parents nouveaux
(17 noms avec les sous-cas). Les originaux Go du candidat et les octets UI figés
sont archivés. Synthèse liée aux sources, reçus, binaire et documents :
`verification-summary.json`, conservée dans les [preuves](evidence/m32-20261017/).
Cette synthèse doit porter PASS ; une interruption ou une dérive ultérieure
invalide la qualification de la nouvelle version, sans réécrire ces reçus.

- M32-01 : quatre cas précis reproduits, originaux conservés ; aucune protection
  ou base de mission réelle modifiée.
- M32-02 : frontières avant/poll/après réponse, refus sans contrôle, causes
  errors.Is/As, SQL absence/incohérence et reprise valide.
- M32-03 : six chemins raccordés et leurs tests de protocole, budgets et reprise ;
  pause lisible/absente/invalide vérifiée via DB et transaction sans mutation.
- M32-04 : vrais processus de doubles locaux, remplacement concurrent et cause
  de pause ; lectures par la fonction `run` de la CLI en Go et le handler HTTP authentifié.
  Aucun binaire Swarm de production, serveur réel ou modèle distant exécuté.
- M32-05 : tests ciblés/race, vet/build, entrées exactes, auto-revue, documentation
  FR/EN et deux schémas Mermaid rendus et inspectés ; RETEX ci-dessous.

Le chantier identité a confirmé ses sources Go stables après le message autorisé
par l’utilisateur. La première fixture de onze fichiers reste insuffisante : stable-race exécute
23 noms PASS mais son reçu FAIL conserve les changements de web/i18n-en.js et
locales/en.json. La correction de méthode fige toutes les 138 ressources
web/locales inventoriées (7 867 262 octets) dans un overlay explicite partagé par
list/test/vet/build. Leurs octets sont archivés en tar.gz, avec chemins temporaires
et empreintes vérifiables ; aucun fichier source vivant n’est modifié et aucune
source Go n’est substituée par ce dispositif. Les autres entrées compilées restent
contrôlées avant/après. Le binaire de fixture contient ces versions web figées ;
la présente preuve ne qualifie pas l’interface courante. Go ne virtualise pas les
lectures de fichiers à l’exécution : les tests mémoire retenus n’en dépendent pas.

La sélection normale complète est conservée. La sélection race qualifie les
neuf nouveaux parents, le renouvellement, la disponibilité du conducteur, les
réservations concurrentes et les rollbacks ; les variantes de protocole sans
état partagé nouveau restent couvertes en normal. Le précédent reçu race de
34 noms reste explicitement invalide pour la version. Voir
[overlay-selection.json](evidence/m32-20261017/overlay-selection.json) et
[coordination-and-overlay.json](evidence/m32-20261017/coordination-and-overlay.json)
et [la reprise de fixture](evidence/m32-20261017/frontend-fixture-replan.json).

La tentative qualified-normal n’exécute aucun test : la compilation manque
d’espace sur le tmpfs /tmp (7,2 Gio). Le reçu et sa sortie restent conservés ;
ce n’est pas un échec produit ni une preuve PASS. Le disque garde environ
12 Gio disponibles. La reprise disk-normal utilise seulement un TMPDIR privé
sur ce disque, sans supprimer de preuves/caches ni modifier la politique produit.
La série race déjà engagée s’est achevée avec 23 noms PASS et des entrées stables ;
elle n’a pas été relancée puisque ses entrées pertinentes sont restées identiques. Diagnostic et précondition
mesurée : [disk-resume.json](evidence/m32-20261017/disk-resume.json).

Les recettes Go utilisent -timeout=0 et des subprocess sans watchdog global,
faute de borne globale justifiée. Les barrières de fixture attendent un état
observé ; elles ne fabriquent pas un succès après un délai. La sélection finale
est conservée dans [final-selection.json](evidence/m32-20261017/final-selection.json).
Les recettes à étiquettes exclusives conservent chaque tentative, sa commande,
ses noms réellement exécutés et ses entrées avant/après. Aucun test identité n’a
été modifié pour qualifier M32 ; leurs changements concurrents restent préservés.

## RETEX et suite

La décision fermé/ouvert et la cause d’un contrôle sont deux informations utiles.
Le booléen préservait souvent le refus mais effaçait la cause. Le contrat typé
les garde ensemble et ferme aussi la fenêtre d’une réponse avant le premier poll.
Le contrôle de résultat doit rester défensif même après annulation : le refus
correct du lanceur n’autorisait pas l’ancien accès par index à un lot remplacé.

Une hypothèse doit rester distincte de sa preuve : la pause invalide était déjà
refusée par la transaction. La correction justifiée est son diagnostic sûr,
réutilisé aux frontières concernées. Lire le producteur d’une mesure évite de
confondre fichier d’un double et compteur persistant. Une connexion de fixture
sans les fonctions d’intégrité du moteur ne justifie jamais leur désactivation.

Le contrôle de version a refusé une série pourtant verte : garder ce refus et
requalifier les entrées réelles, sans écraser le travail concurrent. La réception
du message et la décision du propriétaire de stabiliser ses fichiers sont
observées dans son suivi public ; la stabilité effective reste mesurée par les
manifestes de compilation. Pour une ressource web en mouvement, figer des octets
de fixture explicites isole le test backend sans prétendre accepter une autre
version du produit. Figer seulement les fichiers déjà observés mouvants était
insuffisant : la fixture doit couvrir le contrat embarqué complet des ressources
UI, dont les traductions, tout en conservant la détection des autres entrées et
des nouveaux fichiers. Un échec en compilation lié au volume temporaire doit
être distingué d’un test réellement exécuté ; mesurer l’espace et changer le
TMPDIR de la vérification suffit ici, sans effacer les preuves conservées. Le premier contrôle documentaire final
a également refusé un lien vers sa propre sortie encore absente ; le rapport
pointe désormais vers le dossier existant, sans fichier PASS provisoire et sans
relancer les tests Go inchangés. Les tests
nouveaux sont archivés dès leur premier reçu ; l’assertion candidate historique
est également reconstruite et confrontée à son empreinte avant conservation.
Une session de rendu déjà collectée ne doit pas être attendue une seconde fois ;
le refus de handle est conservé et les PNG terminés sont inspectés directement.

MEM-P1 reste en cours : replis/vues, autres lecteurs de pause, enveloppes liées et
injection pendant préflight restent attribués dans la matrice. La panne des
recorders d’usage est une découverte distincte à auditer avec le moteur/budgets,
sans prétendre ici avoir prouvé un dépassement. P2–P5 et M20 restent ouverts.

Documentation et schémas persistants FR/EN actualisés dans cet incrément. Aucun
contexte agent ou asset web embarqué modifié par M32 ; pas de génération produit
requise. README/référence gardent leurs commandes et champs existants ; les détails
opérationnels sont dans RUNTIME-RECOVERY et PROJECT-MEMORY. Aucun commit, push,
redémarrage du serveur existant, déploiement, agent natif ou fournisseur réel.
Auto-revue seulement ; aucune acceptation indépendante ou globale revendiquée.
