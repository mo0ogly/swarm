# Résultats M31 — erreurs du conducteur

17 octobre 2026. Incrément local MEM-P1, Codex seul. Critères et arbitrage :
[INCREMENT-MEMOIRE-27.md](INCREMENT-MEMOIRE-27.md). Preuves :
[evidence/m31-20261017](evidence/m31-20261017/). Aucun commit, redémarrage du
serveur existant, déploiement, appel fournisseur réel ou mission native.

## Résultat observé

L’erreur d’inventaire/occupation quitte le cycle en conservant sa cause. Les
baux déjà possédés reçoivent un diagnostic d’erreur ; aucun bail n’est acquis
pour produire ce diagnostic. Le watch d’une mission utilise sa lecture ciblée.
Le chemin global reste fermé si l’inventaire est incomplet ou incohérent.
Les empreintes éphémères sont invalidées pour la reprise, sans modifier tâches,
agents, réservations, consommation ou politiques.

Le contrôle de supervision réserve l’écriture avant de vérifier son propriétaire.
Un bail arrêté/remplacé ne peut pas être réactivé par ce contrôle. Premier échec :
pas de fausse réconciliation. Erreur vide/contrôles : message explicite, cause
retournée conservée. Les transitions `cycle-error`/`cycle-recovered` sont ajoutées
au journal existant, atomiquement avec le diagnostic et sans répétition à chaque
poll. Une erreur d’écriture annule le contrôle. Ces traces ne deviennent pas des
actions métier dans LastAction. Trace d’acquisition et arrêt propagent leurs
échecs au lieu de les ignorer.

La boucle transmet ses transitions sur stderr et le callback historique reçoit
également les erreurs globales. Un retour normal après erreur signifie que le
cycle a repris, pas qu’une mission est acceptée ou terminée. Le stockage bloqué
interdit les écritures de diagnostic du cycle ; une persistance impossible peut
laisser un état ancien jusqu’à expiration. Si le Work lui-même est illisible,
les lectures publiques de cette mission retournent son erreur, pas un statut
fabriqué. Un snapshot d’une mission saine expose bien l’erreur globale reçue.

## Preuves conservées

| Critère | Résultat et frontière |
| --- | --- |
| M31-01 | Six cas précis rouges initiaux, puis deux cas précis vide/contrôles : 8 au total, sept scénarios. Reçus originaux FAIL et sources exactes archivés, empreintes conformes ; aucune protection moteur retirée. Le bail remplacé passait déjà avant correction. |
| M31-02 | Inventaire défaillant, callback, HTTP authentifié et reprise ; comparaison de toutes les tables hors diagnostics. La cause d’origine traverse errors.Join/Is. |
| M31-03 | Premier échec, erreur vide/contrôles, baux arrêtés/remplacés/inconnus, deux Stores concurrents, refus du journal sans effet partiel puis reprise. Garde stockage sans aucune écriture. |
| M31-04 | Vraie CLI compilée : 4 assertions sur 8 commandes finies et un watch. Work étranger incohérent, deux contrôles en erreur sans spam, réparation, état CLI JSON/plain actif, arrêt SIGINT puis réouverture absente. Aucun agent créé. Handler snapshot HTTP local authentifié vérifié en Go ; pas de navigateur ni serveur réel modifié. |
| M31-05 | Candidat final : 112 noms PASS, 80 tests parents + 32 sous-cas ; 23 sous race, 17 parents + 6 sous-cas, tous compris dans les 112. Onze nouveaux tests parents/17 noms attendus réellement exécutés. vet/build passent. Entrées Go/module/embed stables pendant les séries. Docs FR/EN et deux rendus Mermaid lisibles, diff contrôlé et auto-revue. |

Les résultats `candidate` et `delivery` restent historiques : le premier a 105
noms passants mais un refus de bail dont le libellé n’est plus compatible avec
un test existant ; le second passe 109 noms avant les dernières gardes et la
normalisation commune du message stderr. `final` et `race` qualifient ces sources Go, mais huit assets web embarqués ont
changé après leurs séries stables. Les reçus `refresh`/`race-refresh` qualifient le checkout courant avec ces
assets : mêmes 112/23 noms, entrées stables, vet/build. La CLI du nouveau binaire repasse ses quatre assertions. Le refus original et les huit empreintes modifiées restent
dans `verification-concurrent-assets.json` ; aucun travail concurrent écrasé. Le préfixe historique a été restauré sans modifier le test ni
réduire son refus. Aucun lot interrompu ou à zéro test n’est compté PASS.

Rejouer sur une nouvelle copie de reçus pour éviter tout écrasement : Go
`-run '^Test(Mission.*|DurableCoordinator.*|RuntimeHealthBlocksWithoutChargeAndResumesConductor|ManagedReviewDoesNotBlockConductorOrOtherMissions|WorkRowStore.*|WorkRowReadEnvelope.*)$' -count=1 -timeout=0` ; la recette
[run_checks.py](evidence/m31-20261017/run_checks.py) conserve logs et entrées.
[verify_cli.py](evidence/m31-20261017/verify_cli.py) attend les contrôles persistés,
sans watchdog global ni délai de succès arbitraire. Ses seules écritures directes
affectent les révisions de sa propre fixture pour reproduire l’erreur ; aucune
base de mission réelle n’est utilisée. Le seul trigger retiré est le défaut de
journal injecté par son test Go, jamais une protection de production.

## RETEX et suite réelle

La présence récente du conducteur n’établissait pas la réussite de sa lecture.
Le diagnostic était après un retour silencieux, et un contrôle d’erreur pouvait
produire sa première réconciliation ou réveiller un bail arrêté. La correction
porte sur ces contrats partagés, sans exception par mission et sans changer les
délais de poll/expiration, le schéma ou les droits.

La revue élargie a été utile : conserver un refus ne suffit pas si un consommateur
attend son préfixe existant ; une erreur non nulle doit aussi rester une erreur
lorsque son texte est vide ou neutralisé. Les contre-exemples restent dans les
preuves. La récupération d’une source rouge a elle-même été vérifiée par son
empreinte, après diagnostic d’un commentaire omis ; conserver désormais le
nouveau test au premier reçu, avant son enrichissement. Un contrôle final a
aussi refusé huit assets embarqués modifiés après les tests : préserver ce refus
et recompiler/requalifier le candidat courant. Le contrôle des entrées vérifie
maintenant aussi leur inventaire pour détecter un fichier ajouté ou supprimé,
pas seulement les empreintes des fichiers déjà connus.

M31-01 à M31-05 sont vérifiés localement ; aucune acceptation indépendante ou
qualification fournisseur/distribuée réelle n’est revendiquée. La documentation
et les schémas sont persistants, aucun contexte agent ou asset embarqué n’est
modifié. La référence et le README gardent leurs commandes et champs existants ;
les détails de ce comportement sont dans RUNTIME-RECOVERY et PROJECT-MEMORY.
Le serveur déjà démarré ne reçoit pas ces sources automatiquement.

MEM-P1 demeure **en cours** : callbacks de fraîcheur, vues de repli, enveloppes
liées et injection pendant préflight gardent propriétaire/critère de reprise dans
[AUDIT-REPRISE.md](evidence/m31-20261017/AUDIT-REPRISE.md). La prochaine tranche
porte sur la cause des annulations de fraîcheur avant d’aborder les droits.
P2–P5 et M20 ne sont pas livrés par cet incrément ; la session identité reste
propriétaire de son autorité protégée. Sources Go hors M31 et 37 fichiers
identité observés restent inchangés depuis la série rouge.
