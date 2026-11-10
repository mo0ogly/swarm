# M27 — frontière des lignes source de mémoire versionnée

17 octobre 2026, reprise « merci la suite maintenant ». Codex seul, replan/APEX
borné. M26 est conservé ; l'identité poursuit son service protégé et son
checkpoint, sans raccord public. Aucun message inter-chat ni appel de modèle.

## Besoin et observation

Le lecteur `memoryWork` décode seulement `body`. Il ne compare pas identité,
schéma et révision avec la ligne SQL ni ne refuse UTF-8 invalide avant décodage.
`memoryLegacyPaths` parcourt aussi des corps seuls ; le catalogue utilise
`s.list`, et ne consomme donc pas `memoryWork` contrairement à l'hypothèse de
la première mise à jour. Ce sont trois voies mémoire à vérifier ensemble.
Sélection, profil/guide hérité et garde avant fournisseur utilisent ces lectures.
Une attribution propriétaire/auteur n'est pas déduite d'une concordance SQL.

Comparer : renforcer uniquement la résolution explicite, ou partager un lecteur
de ligne mémoire entre résolution, catalogue et guide. Retenir la seconde,
sans changer les lecteurs globaux du Store ni les sources identité possédées
par l'autre chantier. Requête id/révision/body dans une seule lecture ; le
modèle valide doit conserver schéma 1, ID demandé et révision positive concordante.
Un refus ne retourne ni Work partiel ni catalogue vide considéré réussi.

## Plan et critères

| ID | Propriétaire / effet observable |
| --- | --- |
| M27-01 | Codex : reproduire avant correction le décodage d'une ligne à identité/révision incompatible, sorties et sources conservées. |
| M27-02 | Lecteur mémoire partagé : UTF-8/JSON/schéma/ID/révision invalides refusés, Work vide ; pas d'écriture ou de migration, récupération avec ligne valide. |
| M27-03 | Catalogue et guide utilisent la même validation ; ligne invalide après une ligne valide ne laisse pas de résultat partiel. Les deux lecteurs ferment leur curseur avant tout autre accès SQL. |
| M27-04 | CLI/API réels en processus : refus explicite sans succès vide ni contenu mémoire, état inchangé ; normal, sélection et fournisseur scripté restent valides. |
| M27-05 | Gardes version/retrait/import/guide, héritage de dialogue et cohérence des contextes vérifiées sur leurs consommateurs, concurrence ciblée si affectée. |
| M27-06 | Entrées stables, statique/compilation ciblées, revue locale, docs FR/EN, audit et RETEX ; aucune protection d'identité ou déploiement déduit. |

Fichiers : `project_memory.go`, `project_memory_public.go`, nouveau lecteur et
ses tests dédiés ; docs mémoire FR/EN, contrat, plan et résultats. Pas de nouveau
schéma de stockage, endpoint, politique d'exécution ou limite. Les lecteurs
QueryRow et Rows partagent la même validation ; parcours du guide en flux,
sans charger toutes les missions simultanément ou réentrer dans SQL.

Les fixtures migrées conservent leurs triggers. Les formes que ces triggers
refusent déjà sont testées sur une petite fixture de lecture SQL explicitement
séparée, sans mission réelle ni suppression de garde. Ne pas présenter ce
scénario comme une attaque publique démontrée. Les tests de contrat précèdent
la correction ; `go test -timeout=0`, aucun timeout Python, progression observée.

## Améliorations et périmètre

Les lecteurs globaux `Store.get/list` ont aussi une validation limitée du corps.
Options : modifier immédiatement tous les consommateurs applicatifs, ou corriger
la frontière mémoire puis instruire ce contrat transversal avec ses propriétaires.
Retenir l'incrément mémoire borné ; l'élargissement exige inventaire des appelants,
fixtures/migrations et coordination du fichier `store.go`, pas un simple patch
sans preuve. Propriétaire de la suite : moteur/stockage ; critère : lire/refuser
les lignes incompatibles dans tous les consommateurs sans perdre les historiques.
État : découverte conservée, effet global non vérifié ; condition de reprise :
ce contrat transversal et son périmètre de tests explicités. Aucune garantie
contre un acteur réécrivant conjointement ligne, preuve et contrôle n'est créée.

Plusieurs chemins ont encore été supposés pendant une recherche (`work.go`,
`operations.go`, `workflow_catalog.go`) : fichiers absents, aucune mutation.
La reprise utilise `rg` sur le répertoire réel et retrouve `store.go` et
`task_definition.go`. Ne pas considérer une hypothèse de chemin comme un fait.

## M27-I01 — contre-exemples observés

Le lot rouge donne deux noms PASS (tableau/JSON malformé déjà refusés) et huit
noms FAIL, plus le paquet : six variantes invalides renvoient un Work utilisable,
et le catalogue CLI publie un résultat pour un Work dont l'ID JSON diffère de
la clé SQL. Les deux parents sont inclus dans les huit noms en échec. Entrées
stables ; sorties et source exact dans `evidence/m27-20261017/red-*`.
Correction générale retenue : même lecteur id/révision/body pour les trois
voies mémoire, refus sans objet partiel, aucune identité ou permission créée.

Une première insertion documentaire n'a pas trouvé son contexte : la ligne
d'ancrage ne correspondait pas au retour à la ligne réel du paragraphe. Le
patch est refusé sans mutation ; relecture des deux paragraphes puis ancrage
exact et modification ciblée. Le diff doit confirmer le seul paragraphe
ajouté et le libellé du graphe concerné ; pas de transformation globale.

## M27-I02 — sélection d'un producteur de fixture conditionnel

Le lot candidat exécute 60 noms PASS, zéro FAIL, mais ignore
`TestProjectMemoryRecipeBundle` parce que `SWARM_MEMORY_RECIPE_BUNDLE` n'est pas
fourni. Le verdict strict du lanceur reste FAIL malgré tests/statique/compilation
terminés à zéro et entrées stables. Ne pas compter le SKIP comme une preuve ni
relancer le lot inchangé : fournir un chemin d'archive neuf, isolé dans les
preuves, puis exécuter ce seul producteur et vérifier son manifeste. La recette
spécifique enregistre cette variable non secrète ; le garde de destination
existante est conservé. Aucun besoin de navigateur ou compte fournisseur réel
n'est déduit de cette production de fixture. Les 60 résultats exécutés restent
réutilisables tant que leurs entrées Go et d'exécution pertinentes sont identiques.

## M27-I03 — diagnostic de la ligne fautive

La revue code-reviewer constate qu'une erreur de scan de plusieurs missions
n'identifie pas la ligne source. Le nouveau garde échoue avant correction :
`TestMemoryWorkRefusalIdentifiesSafeStoredSource`, entrées stables, source et
messages dans `diagnostic-red-*`. Correction : nommer seulement la clé SQL
validée, jamais le corps ni l'ID JSON incohérent. Une clé SQL invalide reçoit
un refus générique sans être recopiée. Les causes SQL/JSON originales restent
propagées et aucun objet partiel n'est renvoyé. Le lot complet est repris avec
une nouvelle destination de recette, puisque le code de diagnostic a changé ;
le premier chemin exporté reste protégé contre l'écrasement.

Choix de disponibilité explicite : une ligne incohérente arrête tout le scan
mémoire/guide, même après des lignes valides ; ignorer la ligne pourrait masquer
une leçon gérée et contourner la sélection explicite. Une quarantaine avec
index indépendant serait un autre contrat à concevoir, pas un succès vide ou
une exception improvisée. Le refus conserve les originaux et exige le traitement
de l'incident de stockage ; ce lot n'ajoute pas d'outil public de réparation.

## Résultat et reprise

Le lot final exécute 63 noms PASS (30 principaux, 33 sous-cas), zéro FAIL/SKIP ;
statique et compilation réussies, entrées stables. Le lot race exécute 21 noms
PASS (11 principaux, dix sous-cas), sans FAIL/SKIP. Le producteur d'archive et
ses blobs sont vérifiés sur deux destinations isolées ; les reçus initiaux ne
sont pas remplacés. Audit, diff, empreintes et liens dans `audit-final.json` ;
limites et RETEX dans [RESULTATS-MEMOIRE-23.md](RESULTATS-MEMOIRE-23.md).
Aucun droit M20 public ni protection globale des lecteurs du Store n'est déduit.
