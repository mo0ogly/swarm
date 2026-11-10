# M27 — lignes source cohérentes dans les lectures mémoire

17 octobre 2026, Codex seul, replan/APEX, revue locale code-reviewer et parcours
verify-fix. Checkout direct et travaux tiers conservés ; aucune orchestration,
revue indépendante ou acceptation globale M20 n'est prétendue.

## Résultat

La résolution d'une mémoire, le catalogue et le filtrage du guide hérité
partagent `scanMemoryWork`. Ce lecteur reçoit clé SQL, révision et corps dans
un même résultat de ligne ; il exige UTF-8/JSON valides, schéma 1, identité
concordante et révision positive identique au corps. Toute erreur renvoie un
Work vide, ou aucune collection avec erreur. Une source incohérente ne devient
plus une mémoire consultable sous une autre identité ou un succès vide.

`memoryWork` reste utilisable avec la base ou la transaction du lancement.
`memoryWorks` conserve l'ordre récent du catalogue ; `memoryLegacyPaths`
parcourt les lignes en flux, sans conserver toutes les missions ni emprunter
une autre connexion. Les curseurs sont fermés sur succès et erreur avant les
lectures suivantes. Un refus après une ligne valide ne livre pas de collection
partielle. Le diagnostic nomme seulement une clé SQL valide, sans recopier une
clé invalide ni le corps du travail.

Les sources originales, événements, versions et contextes ne sont pas réécrits.
Aucun schéma de stockage, endpoint, droit, auteur ou propriétaire nouveau ;
aucune politique de délai, volume, consommation ou rétention n'est ajoutée.
Les lecteurs globaux `Store.get/list` gardent leur comportement actuel : le
contrôle partagé ici concerne les trois voies mémoire explicitement nommées.

## Contre-exemples, erreurs et décisions

**M27-I01 — corps décodé sans liaison à la ligne.** Avant correction, six
variantes (identité, révision, révision nulle, schéma, JSON null, UTF-8 invalide)
renvoient un Work utilisable. La CLI publie aussi un catalogue quand l'ID JSON
d'une mission ne correspond pas à sa clé SQL. Le lot rouge conserve deux noms
PASS (tableau et JSON malformé déjà refusés) et huit noms FAIL, plus le paquet ;
les deux parents sont inclus dans ces huit échecs. Entrées stables, source exact
et sorties sous `red-*`. La corruption est injectée dans des fixtures : aucune
attaque publique par un tiers n'est revendiquée.

**M27-I02 — producteur d'archive ignoré.** Le premier lot candidat exécute
60 noms PASS, zéro FAIL, et un SKIP : le chemin `SWARM_MEMORY_RECIPE_BUNDLE`
manque. Le lanceur strict garde donc FAIL malgré tests/statique/compilation à
code zéro. Ne pas masquer le SKIP : un chemin d'archive neuf est fourni, puis
le seul test est réellement exécuté (un PASS, aucun SKIP). Manifeste, événements,
RETEX vérifié et blobs sont inspectés. Le lot final utilise un autre chemin
neuf, avec environnement explicitement enregistré, et ne saute plus ce test.
Les deux archives sont des fixtures synthétiques, pas une preuve de navigateur
ni d'acceptation utilisateur. Aucun garde de destination n'est contourné.

**M27-I03 — erreur sans source identifiable.** La revue locale constate que
le diagnostic ne nomme pas la ligne lors d'un scan multi-missions. Le test
`TestMemoryWorkRefusalIdentifiesSafeStoredSource` échoue avant correction ;
source et sorties `diagnostic-red-*` conservées, entrées stables. Les refus
nomment désormais la clé SQL validée ; une variante vérifie qu'une clé invalide
ne publie pas sa valeur dans le message. Les causes SQL/JSON restent conservées.

Une première insertion documentaire a été refusée sur un contexte de ligne
inexact, sans mutation. Les paragraphes FR/EN ont été relus puis la modification
appliquée sur leur texte exact. La première recherche a aussi supposé plusieurs
chemins absents ; les symboles ont ensuite été retrouvés avec `rg` dans les
fichiers réels. Ces erreurs de méthode restent dans le plan et ne sont pas
présentées comme des défauts du moteur ou des vérifications réussies.

## Vérification et preuve

Préimages, branche/HEAD et état sale : `evidence/m27-20261017/baseline.json`,
`before/`, `status-before.txt`. Les lanceurs inventorient les dépendances locales
Go réellement compilées, ressources embarquées et recette ; ils conservent
noms, commandes, dates, sorties originales et empreintes avant/après. Aucun
timeout Python ; `go test -timeout=0`. La progression est suivie, sans modifier
les délais fonctionnels du produit ni introduire un watchdog global arbitraire.

Le lot final donne **63 noms PASS : 30 tests principaux et 33 sous-cas**, zéro
FAIL/SKIP. Il couvre le nouveau lecteur, les vrais parcours CLI/API en processus,
remplacement/retrait et concurrence, import, migration, guide/profil, héritage
de dialogue et cohérence du contexte effectivement reçu par un fournisseur
scripté. Statique et compilation passent, à entrées stables. Les tests qui
décrivent des contextes de décision ne constituent aucun appel de modèle.

Le contrôle race final donne **21 noms PASS : 11 tests principaux et dix
sous-cas**, zéro FAIL/SKIP, entrées stables. Il cible les lecteurs, refus/curseurs, remplacement concurrent,
retrait hérité, guide et deux parcours extractifs M26 avec sous-processus scripté.
Une réussite de race ne prouve pas un engagement atomique des usages M20.

| Critère | Résultat local et limite |
| --- | --- |
| M27-01 | PASS : lecteur de ligne et catalogue CLI rouges avant correction ; originaux conservés. |
| M27-02 | PASS : huit variantes invalides refusées, Work vide, source inchangée puis lecture valide ; identifiant dangereux, absence et panne SQL refusés avec cause. |
| M27-03 | PASS : catalogue/guide sans résultat partiel, ordre conservé, curseurs libérés et lectures transactionnelles sur une seule connexion. |
| M27-04 | PASS : CLI sans stdout et HTTP en erreur sur la ligne incohérente, pas de contenu divulgué ; état identique, catalogue valide après restauration de la seule fixture. Aucun outil public de réparation démontré. |
| M27-05 | PASS normal et race : versions/retraits/imports, guide, héritage et entrée fournisseur scriptée observés ; aucune identité réelle ni autorité protégée. |
| M27-06 | PASS local : statique/compilation, race, empreintes, formatage/diff et liens dans `audit-final.json`. Même auteur, aucune revue indépendante. |

Les reçus originaux figurent dans `red-*`, `candidate-*`, `recipe-*`,
`diagnostic-red-*`, `final-*`, puis `race-*`. Les archives ont une vérification
séparée de manifeste et blobs. `owned.diff` et `audit-final.json` décrivent le
contenu possédé et les limites ; aucun résultat d'un processus encore actif
n'est présenté comme un verdict final.

## Limites, suite et RETEX

Une incohérence arrête le scan entier du catalogue/guide, même si d'autres
missions sont valides : ignorer cette ligne pourrait masquer une leçon gérée
et permettre une source héritée sans sélection. Une quarantaine avec index
indépendant serait un autre contrat. Le refus conserve les originaux et exige
le traitement de l'incident de stockage ; aucune réparation privée ou migration
silencieuse n'est appliquée. Le retour au cas valide dans les tests répare la
fixture contrôlée ; il n'est pas un parcours opérateur de réparation de base.

La concordance SQL ne protège pas contre la réécriture conjointe d'une ligne,
des originaux et du contrôle par un acteur qui les possède. Elle n'atteste ni
propriétaire, auteur, permission, avant-plan ou réception par le bon agent. Les
dix conformités M20 restent ouvertes. Le chantier identité poursuit sa frontière
locale choisie, son checkpoint et son service ; aucune activation ne s'en déduit.

Découverte conservée : les lecteurs globaux du Store ont aussi une validation
limitée du corps. Modifier immédiatement tous leurs consommateurs aurait élargi
le contrat et les risques de migration. La correction mémoire est bornée ; la
suite moteur/stockage doit inventorier les appelants et leurs historiques, définir
le contrat partagé et ses gardes, puis coordonner `store.go`. Propriétaire de
cette suite : moteur/stockage ; effet global non vérifié, pas de message inter-chat.
Le plan conserve la provenance, la valeur, le critère observable et cette condition
de reprise. L'incident n'est pas réputé clos à l'échelle de tous les lecteurs.

Leçon : l'empreinte d'une mémoire et une source RETEX vérifiée ne remplacent pas
l'identité du conteneur qui les porte. Valider clé, version de schéma, révision et
encodage à chaque frontière mémoire ; ne pas cacher une ligne invalide dans un
catalogue partiel. Un test conditionnel non exécuté reste sans preuve jusqu'à
sa précondition explicitement fournie. Transmission : lecteur commun, gardes
nommées, docs FR/EN et point de reprise du plan, sans changer un contexte natif
ou un processus déjà démarré.

Aucun écran/asset n'est modifié ; les libellés du graphe FR/EN correspondent au
contrôle courant, rendu graphique non vérifié dans ce lot. Aucune recette
navigateur, compte fournisseur réel, mission native, délégation, interruption,
redémarrage, commit ou déploiement. Le binaire compilé est un candidat isolé ;
il n'est pas présenté comme le serveur utilisateur. RETEX de ce lot local,
conformité M20 et RETEX global toujours distincts.
