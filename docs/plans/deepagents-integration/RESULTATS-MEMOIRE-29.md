# Résultats M33 — lecture de pause sans succès inventé

17 octobre 2026, Codex seul. Incrément vérifié localement sur des entrées effectives stables.
MEM-P1 reste ouvert ; aucune acceptation indépendante ou globale.
[Plan et décisions](INCREMENT-MEMOIRE-29.md), [preuves](evidence/m33-20261017/),
[inventaire](evidence/m33-20261017/consumer-inventory.json).

## Comportement réalisé

Le lecteur Store.paused qui absorbait les erreurs est supprimé. Ses 19 appels
et sept gardes SQL directes utilisent readMissionPaused : booléen contrôlé,
absence historique non pausée, cause conservée et message public sûr sans
recopier la valeur invalide. Le changement touche 21 modules de production,
deux assertions de tests existants et un nouveau fichier de régression.

Les opérations de lancement, modèle, contrat, contrôle, règlement et automation
refusent une pause illisible. Les gardes transactionnelles gardent leurs contrôles
dans la même transaction ; aucune réserve, tentative ou publication n’est autorisée
par ce diagnostic. Les interfaces qui retournent une erreur conservent sa chaîne
Go. Un prédicat d’éligibilité retourne false avec une raison ; les interfaces
textuelles et le contexte partiel transportent un diagnostic, pas une chaîne Go.

Le snapshot et l’état de mission CLI/HTTP propagent le refus au lieu de publier
paused=false. Les trois vues texte affichent le diagnostic. Le contexte de page
omet suspension_departs, attribue la donnée absente au Work dans Missing et ne
propose plus task.start comme disponible. Le cycle conserve son empreinte
précédente sous erreur, au lieu de confondre celle-ci avec une pause métier.

La préparation de lancement conserve son idempotence et l’exception existante
d’aperçu de mission en pause ; une valeur illisible reste refusée. La réparation
publique de pause permet la reprise ; les compteurs et l’historique ne sont ni
effacés ni remboursés. Modèles, droits, identité et politiques Admin ne sont pas
modifiés. L’option désactivée qui ne consultait pas la pause garde son contrat.

## Avant correction, témoins et résultats

Chaque injection suit un témoin valide : les vues lisent la même fixture, les
gardes acceptent le même état légitime dans une transaction, le lancement est
préparé en aperçu sans réserver un agent, le modèle est prévisualisé et la
révision de contrat est appliquée. La qualification renforcée ajoute aussi une
application de modèle avant injection, l’indisponibilité de task.start et la
reprise des révisions/cycles. Ce sont des préconditions locales explicites,
pas une approbation indépendante ou une action d’un modèle distant.

| Reçu / frontière | Résultat observé |
| --- | --- |
| red, 16 noms en échec | Huit parents et huit sous-cas échouent après leurs témoins valides ; source exacte archivée et entrées stables. |
| Snapshot, mission status, dispatch et aperçu de validation | Succès retourné malgré une pause illisible ; CLI/HTTP publient une pause false. |
| Éligibilité, vues et contexte | Cause perdue, départ encore présenté comme possible ou fait false inventé. |
| Préparation, modèle et gardes transactionnelles | Plusieurs refus exposent la valeur privée ; preview modèle, contrat et automation perdent la cause. Cela ne prouve pas un départ non autorisé. |
| Cycle | Retour nil et perte de l’empreinte précédente malgré une lecture invalide. |
| candidate | Les 16 noms passent après correction, entrées stables. Les témoins et reprises sont ensuite renforcés pour les reçus finaux. |

Le test rouge d’origine est conservé tel quel dans red-test-source.go.txt, lié
à son inventaire. Les fichiers Go du candidat renforcé et les notes locales sont
archivés ; le diff limité est calculé contre les préimages, afin de préserver
les modifications antérieures M29–M32 et celles du chantier identité.

## Vérifications

| Contrôle | Résultat réel |
| --- | --- |
| final | 163 noms PASS : 87 parents et 76 sous-cas, aucun FAIL ni skip |
| race | 20 noms PASS : 12 parents et huit sous-cas, déjà présents dans la série normale |
| notes | Quatre noms PASS : parseur embarqué, projection serverVersion, validation des notes locales et CLI sans ouverture de stockage |
| Livraison locale | go vet moteur et compilation cmd/swarm PASS ; binaire de fixture lié par SHA-256 |
| Entrées | Manifestes avant/après identiques dans les trois séries, octets des overlays et code racine inclus |

Cela représente 167 noms distincts, dont 20 exécutés aussi sous race. Les huit
nouveaux parents (16 noms avec sous-cas) sont présents dans les deux séries
mémoire. La synthèse `verification-final-summary.json`, dans les
[preuves](evidence/m33-20261017/), lie les sources et documents courants, les
originaux, les reçus, les schémas et le binaire. Seul son statut PASS permet de
considérer cette liaison qualifiée ; une nouvelle source exige une nouvelle preuve.

- M33-01 : témoins avant défaut, refus rouges et source d’origine conservés.
- M33-02 : 19 consommateurs booléens retirés, sept gardes directes raccordées ;
  inventaire complet calculé sur les sources archivées.
- M33-03 : causes typées, refus sans mutation/réservation dans les cas injectés,
  puis réparations publiques et reprises. Les tests existants couvrent aussi
  idempotence, budgets, concurrence et publication dans les chemins affectés.
- M33-04 : fonction `run` de la CLI et handler HTTP authentifié, trois vues texte
  et contexte partiel avec départ indisponible. Aucune session navigateur courante
  ni fournisseur distant qualifié par ces assertions.
- M33-05 : sélection ciblée, race, vet/build, source exacte, documentation FR/EN,
  schémas rendus et inspectés et auto-revue ; liaison finale conservée dans la synthèse.

La sélection normale couvre 87 parents, la sélection race 12 :
[selection.json](evidence/m33-20261017/selection.json). Les recettes conservent
chaque nom réellement exécuté, commande, sortie, code et empreinte avant/après.
Go -timeout=0 et subprocess sans watchdog global ; les attentes contractuelles
existantes restent intactes. Le TMPDIR privé est sur disque après mesure du
tmpfs peu libre ; aucun cache, lock ou fichier d’un tiers n’est supprimé.

L’inventaire Go vient de go list et inclut le code racine, les paquets, tests,
ressources embarquées et octets effectifs des overlays. Les 138 ressources UI
réellement inventoriées sont figées et archivées ; les autres entrées, dont
version_history.json et les contrats embarqués, restent courantes et contrôlées.
Le binaire construit est un artefact de fixture ; aucun serveur existant n’est
redémarré et aucune version active n’est annoncée. Les notes FR/EN sont prêtes
pour une future livraison autorisée par swarm.sh. README/référence ne changent
pas : commandes, champs et schéma public restent identiques.

## RETEX et reprises

Un booléen d’éligibilité peut refuser sans attribuer un faux état à la base.
La distinction essentielle est entre pause connue et lecture inconnue : les
opérations propagent l’erreur, les vues la montrent et un contexte partiel déclare
la donnée manquante au lieu de l’inventer. La relecture de publication et les
droits restent nécessaires ; ce lecteur ne réserve pas un état futur.

Les erreurs de méthode sont conservées. La première préparation de patch
attendait un seul appel dans console.go, qui en contient deux : précondition
refusée avant toute écriture, puis recette avec cardinalité et préimages exactes.
Le premier total annoncé de 24 appels mélangeait appels et gardes ; le calcul
sur les originaux établit 19 appels et sept gardes dans 21 modules. La documentation
est corrigée sans modifier la source testée ni réécrire les reçus. Une lecture
initiale d’un fichier de test supposé inexistant est aussi conservée ; les
fixtures ont ensuite été découvertes avec rg. Une seconde lecture supposait
version_history_test.go, également absent : les chemins réels ont ensuite été
cherchés avant lecture. Aucun de ces défauts de méthode
n’est compté comme régression produit.

Un contrôle complémentaire supposait binary.history dans la commande version ;
le schéma réel ne fournit que l’identité et le diagnostic. Le KeyError reste
conservé, sans changer le CLI pour satisfaire cette hypothèse. Les quatre tests
existants vérifient le parseur des notes embarquées et la projection serverVersion.
La commande du binaire réel de fixture réussit, et les octets exacts du manifeste
FR/EN y sont présents : cette dernière preuve est structurelle. La modale du
serveur actif, sa livraison numérotée et la consommation visuelle restent NON TESTÉES.

La coordination explicite limite le changement d’agents_store.go à deux blocs :
ancien lecteur et garde de réservation. Les empreintes préalables ont permis
de préserver l’identité et l’autorité d’enrôlement. Les notes, plan et preuves
sont persistants ; le serveur déjà démarré et ses contextes embarqués ne sont
pas actualisés par ces fichiers.

MEM-P1 reste en cours. Les deux lectures jointes noyau/supervision, les autres
replis/vues et les enveloppes Agent restent à auditer. Le défaut éventuel de
persistance des recorders d’usage demeure une hypothèse attribuée au moteur/budgets,
sans dépassement prouvé. Propriétaires, critères et conditions dans
[AUDIT-REPRISE](evidence/m33-20261017/AUDIT-REPRISE.md). P2–P5 et M20 restent ouverts.

Aucun commit/push, déploiement, agent natif ou appel payant. Auto-revue locale
seulement ; aucune acceptation indépendante ou globale revendiquée.
