# PAEA — incrément 24 : révocation et engagement concurrents en laboratoire

[English](../../../en/plans/actions-confiance/paea/INCREMENT-24.md) · [Plan](../PAEA-APEX.md)

## Reprise et choix

Lot 23 commité `97933d8b7790fa5ee67956ac96351fd24fca2a72`, 27 fichiers ; les
108 entrées préparées étrangères restent intactes. La reprise « reprends » conserve
le programme PA-00…13, Codex seul sans GLM, les autorités et budgets existants.
Preuves neuves : `evidence/20261026-contention24-efknxkuf/`. Le collecteur initial a omis le
chemin d'import du lanceur ; ajout explicite de `tools` puis lecture d'identité
réussie, sans mutation du service (`replan-resume-import.json`).

Manque observé PA-03/I05/I17 : T05/T17 ne couvrait que des ordres successifs.
Deux options : attente/retry SQL nouveau, ou contention immédiate existante avec
reprise explicite. Choix : conserver SQLite `timeout=0` et observer les vrais
processus sur chacun des deux ordres, sans nouvelle politique. Un helper de test
suspend le processus après lecture d'autorité sous sa vraie transaction SQLite ;
il ne remplace ni les requêtes, ni les effets Git/SQL, ni l'oracle séparé. La CLI
native reste celle du candidat normal dev109, épinglée par empreinte/profil du
lot 23 ; elle ne prétend pas représenter les sources concurrentes courantes.

## Critères et limites

1. Témoin avec le même worker et la même barrière avant chaque contention.
2. Commit détenteur : révocation concurrente refuse avec cause de lock, sans
mutation ; effet exact puis révocation explicite, ordre durable conservé.
3. Révocation détentrice : commit concurrent UNKNOWN sans effet ; après release,
première utilisation de l'ancien grant refusée ; nouveau grant puis effet unique.
4. Deux domaines, oracles bruts et réconciliation, sorties/PID/IPC conservés.
5. Substitution de test libérant trop tôt la transaction : mêmes assertions doivent
échouer après leurs témoins valides, jamais déclarer la propriété démontrée.
6. Recette huit tests et rapport v1 existants conservés ; garde de sortie et
intégrité des recettes vérifiées ; revue liée aux octets politiques/fonctions/erreurs.

Les tests et leurs recettes sont exécutables depuis le checkout utilisé par la
commande du menu ; aucun asset ni source compilée modifié. Une reconstruction ou
un redémarrage ne livrerait aucun de ces octets : aucune nouvelle installation
serveur ni note de version embarquée n'est donc attribuée à cette qualification.
Même UID, autorité simulée, barrières instrumentées et participants coopérants :
ni course distribuée, ni confinement, ni autorité protégée. PA-02/03 protégés,
IA3/AC-02, humains, E2/E3, 0/17 invariants et RETEX final restent ouverts.

## Vérification et suite

À compléter sur exécution effective ; propriétaire Codex de la recette. Une
contention ne prouve pas l'application d'une révocation : seule sa reprise
explicite réussie puis la lecture de l'epoch et du sink établissent l'ordre local.
Les preuves historiques du lot 23 restent inchangées et ne couvrent pas ce helper.


## Résultats, revue et RETEX de tranche

`final-recipe-03` : **trois étapes PASS, intégrité VERIFIED**, sans watchdog.
Le cas ciblé T05/T17 conserve ses ordres historiques et passe les **quatre
contentions**, avec quatre témoins worker/barrière avant injection. La garde
produit **quatre échecs d'assertion attendus**, un par domaine/détenteur ; ses
quatre témoins et pairs réussissent, ses quatre détenteurs sont avortés/recueillis
avec cause originale. Elle ne déclare jamais cette campagne négative PASS.
La commande publique exécute ensuite les **huit tests nommés** dans `campaign-02`,
vérifie/exporte le rapport v1 puis refuse mauvais hash, sortie existante et preuve
altérée sans remplacer le rapport. Recette interne également PASS/VERIFIED.
Ce sont des comptes de scénarios, pas une évaluation humaine indépendante.

Première recette ciblée PASS conservée. `final-recipe-01` échoue dans le nouveau
contrôleur : résumé recherché sur stderr au lieu du stdout de PilotLab.
Correction du consommateur ; `final-recipe-02` exécute tous les tests avec succès
mais refuse le manifeste dérivé qui avait fait l'erreur inverse pour LabCommand.
Causes et sorties originales conservées. Correction générale de la nouvelle
recette d'après ses trois producteurs réels et précontrôle contre les sorties
originales ; inventaires des deux recettes lecteur/composant complétés par le
helper transitif. Leur inventaire/schéma est contrôlé ; leurs navigateurs ne sont
pas rejoués ni requalifiés dans cette tranche. Aucun asset/rendu n'a changé.

`effective-production-inputs.json` rapproche les **dix sources de protection**
consommées avec l'inventaire et l'archive effective du build normal dev109 : mêmes
octets, features de cœur vides, test=false, aucune feature fixture. Empreinte du
binaire exercé : `fb1dcac07bd0794a0f0d45c8b1f379c685ac2a1953e830f4bf784e647e1adc6e`.
Ce réemploi ne qualifie pas le moteur global courant. Lecture protégée distincte :
service dev112/PID1327375, commit embarqué97933d8b7, modifié=true, observé pendant
le travail concurrent (`runtime-observation.json`). Aucun redémarrage/installation
attribué à24 ; configuration source conservée et aucun processus d'autrui signalé.

Revue de même contexte `source-review.json`, liée aux **14 fichiers** : politique
de test/format, contrats des fonctions et erreurs. Pause IPC et libération anticipée
sont des instruments de vérification explicites, pas des politiques opérateur.
Pas de nouveau délai, retry ou plafond de concurrence ; le `timeout=0` préexistant
reste limité à la fixture. EOF/abort annule au point suspendu ; connexion fermée,
processus recueilli, cause et état inconnus conservés. L'assertion examine l'effet
du verrou et le sink, pas seulement son drapeau `in_transaction`.

Apprentissage : vérifier tous les producteurs/flux d'une recette dérivée avant
lancement et transmettre sa nouvelle dépendance à chaque inventaire appelant.
L'erreur du premier contrôleur avait été corrigée trop localement ; le précontrôle
transitif et les trois flux réels traitent cette récidive de méthode. Les documents
persistants et le README des recettes transmettent cet apprentissage ; aucun
contexte d'agent embarqué ou processus déjà lancé n'est réputé actualisé.

Le programme reste ouvert : ces verrous coopérants du même UID ne sont pas une
frontière protégée. IA3/PA-02 (service/bootstrap/principal humain et confinement),
PA-03 protégé, campagnes web et PA-10…13/E2/E3/RETEX global conservent leurs
propriétaires et préconditions du plan. Aucun commit24/push ni acceptation fabriquée.
