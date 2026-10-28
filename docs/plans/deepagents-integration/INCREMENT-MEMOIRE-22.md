# M26 — cohérence du reçu extractif avant départ

17 octobre 2026, reprise « next ». Codex mémoire seul, replan/APEX borné.
M25 conserve ses preuves. Lecture seule du chantier identité : décision IA3c.2f
adoptée, service local sous compte distinct pour le pilote, distant prévu ;
configuration privée en cours, principal initial et raccord public encore ouverts.
Aucune demande inter-chat ni orchestration native.

## Observation et décision

`checkMemorySummaryBeforeStart` relit les sources et leur empreinte de mise en
file, mais ne compare pas le reçu à la demande confirmée `agents.request`.
Une dérive du reçu ou du prompt est une hypothèse à reproduire : ne pas la
présenter comme une attaque publique démontrée. Les tests modifient uniquement
une fixture isolée via son stockage moteur ; aucune base utilisateur n'est touchée.

Deux voies : ajouter quelques comparaisons de champs sans reconstruire la pièce,
ou partager la construction extractive entre admission et contrôle avant départ.
Retenir la seconde avec la demande originale comme ancre de cohérence : même
sélection, octets attendus, sources reçues/omises et contexte confirmé. Ne pas
réinterpréter un reçu absent comme un lancement sans sélection si la demande
originale en comportait une. Un hash local reste une preuve de concordance,
pas une authentification face à un acteur capable de réécrire tous les originaux.

Périmètre : constructeur extractif existant, garde du reçu dans un fichier mémoire
dédié et tests ciblés ; docs mémoire FR/EN, contrat consommateur, plan et RETEX.
Sources identité, autres lanceurs, API/schéma de stockage, assets et serveur
utilisateur préservés. Aucun nouveau droit ni politique d'exécution.

## Critères observables

| ID | Observable et preuve attendue |
| --- | --- |
| M26-01 | Contre-exemple exécuté avant correction : reçu altéré accepté ou fournisseur appelé malgré la divergence ; original conservé. |
| M26-02 | Sélection d'origine, digest et métadonnées complètes reconstruits/contrôlés ; modification, suppression ou injection de reçu refusée avant fournisseur. |
| M26-03 | Préfixe préparé et prompt final concordent avec le contexte confirmé ; ajout de reprise moteur valide conservé, altération refusée. |
| M26-04 | Refus et erreur d'enregistrement ne marquent pas la vérification réussie ; entrées et preuves historiques conservées ; reprise valide possible. |
| M26-05 | Parcours réel Store/supervision/fournisseur scripté, réouverture et gardes CLI/API existantes passent ; M24/M25 restent compatibles. |
| M26-06 | Entrées de test stables, statique/compilation ciblées, revue locale, docs FR/EN et RETEX ; limites d'identité et de fraîcheur explicites. |

Séquence : reproducer, corriger le contrat commun, vérifier les refus et le
parcours valide, relire le diff puis clôturer ce lot local. Tests publics de
contrat écrits avant correction. `go test -timeout=0`, aucun timeout Python :
aucune borne globale n'est justifiée ici ; progression suivie et lenteur diagnostiquée.
Les délais fonctionnels du produit restent intacts. Élargir aux gardes extractives
et M24/M25 parce que le constructeur devient partagé ; pas de suite entière.

## Découverte de méthode

Une lecture a supposé le chemin inexistant `internal/engine/agent_row.go` ;
aucune donnée n'a été lue ou modifiée par cette sous-commande. Les symboles
nécessaires ont été retrouvés avec `rg` dans les vrais producteurs. Prévention :
chercher le chemin avant de le lire, y compris lors de la reprise. Propriétaire :
Codex ; effet local vérifié par les lectures ciblées suivantes, aucune consigne
embarquée ou preuve historique réécrite.

## M26-I01 — défaut reproduit et contrat corrigé

La série rouge exécute réellement les variantes `edited-selection` et
`missing-receipt` : les deux appellent le fournisseur scripté et terminent
`completed`. Trois noms de tests FAIL (parent et deux sous-cas), plus le paquet ;
zéro PASS, entrées stables. Les sorties et le source exact sous son empreinte
sont conservés dans `evidence/m26-20261017/red-*`. Aucun accès pirate réel
n'est revendiqué : l'altération est injectée dans le stockage d'une fixture.

La correction partage la construction des extraits, retrouve la demande
originale avant de décider l'absence de sélection, compare les métadonnées
complètes reconstruites et le préfixe du contexte confirmé. La reprise moteur
reste ajoutable après ce préfixe ; le manifeste courant doit correspondre au
prompt complet. La copie appelante n'est mise à jour qu'après l'écriture réussie
du contrôle. Contrôles de panne et retour au cas valide ajoutés. Identité et
engagement atomique demeurent des dépendances distinctes, pas des garanties
déduites d'un hash ou d'un reçu local.

## Replan de la première vérification

M26-I02 : l'ancienne garde modifiait le reçu appelant avant une sauvegarde en
échec ; `finishAgent` tentait ensuite de sauver ce même reçu estampillé et
échouait à nouveau sur le trigger de fixture. La nouvelle garde conserve
l'appelant, ce qui permet d'enregistrer normalement un état `failed`. Le test
ancien doit vérifier cet état, la cause originale, l'absence d'estampille et
l'absence d'appel fournisseur, puis rouvrir le stockage ; le seul retour nil
du superviseur ne prouve pas un succès métier. Source initiale conservée.

M26-I03 : la fixture UTF-8 malformée est désormais refusée par le trigger
`agent_identity_plan_update`, avant la CLI d'export. Ne pas supprimer ce trigger
pour atteindre le lecteur : vérifier le refus sans mutation à cette frontière,
puis exercer le garde UTF-8 du lecteur sur une fixture SQL minimale dédiée,
sans mission migrée. La première série conserve ses 167 noms PASS et ses trois
noms FAIL, plus le paquet, avec entrées stables. Diagnostic et sources initiales
dans `evidence/m26-20261017/candidate-*` et `before/`.

La sélection de lectures a encore supposé un chemin `store_test.go` inexistant
et demandé le reçu `final-run.json` avant la fin du processus. Aucun résultat
n'en a été inféré. Correction de méthode : rechercher les symboles dans les
chemins inventoriés, attendre la fin réelle de la session avant de lire son
verdict ; les sorties de progression restent distinctes du reçu final. Ces
erreurs de méthode sont conservées avec leurs limites, pas présentées comme
des défauts du moteur.

## M26-I04 — revue des observations historiques

La relecture locale découvre une omission liée : la comparaison du reçu efface
les champs d'observation du contexte préparé avant de les comparer. Cela peut
conserver une fausse estampille historique dans ce contexte, alors qu'il doit
rester antérieur au contrôle de départ. Propriétaire : Codex mémoire ; valeur :
conserver l'état réel des preuves historiques. Ajouter le contre-exemple puis
exiger l'absence de ces champs dans `prepared_context`, sans empêcher une
réobservation valide du seul reçu courant. Attendre la fin du contrôle race en
cours avant toute modification des sources ; conserver cette première preuve
sur son candidat, puis vérifier la correction et ses parcours dépendants.

M26-I04 est reproduit : le nouveau test échoue avant correction parce que le
contexte préparé peut prétendre avoir été contrôlé au départ. Le source testé
et les sorties `review-red-*` sont conservés, entrées stables. Correction :
refuser ces deux champs dans le seul contexte préparé ; un second contrôle
valide du reçu courant reste permis. Les parcours affectés (sélection, contexte
préparé/reprise) sont revérifiés ; les gardes source et usages M24/M25 de la
série verte précédente restent réutilisables, leurs producteurs et appelants
n'ayant pas changé. Le premier contrôle race (31 noms PASS) reste lié au
candidat précédent ; un nouveau lot couvre la garde finale et la concurrence.

## Résultat du lot

Le parcours final donne 81 noms PASS (21 principaux et 60 sous-cas), statique et
compilation réussies, entrées stables ; le lot final race donne 33 noms PASS
(sept principaux et 26 sous-cas), zéro FAIL/SKIP. Les 92 gardes source/usages
antérieures sont réutilisées sur leurs dépendances inchangées, pas présentées
comme rejouées après le dernier ajout. Sources possédées, diff, liens et
empreintes dans `audit-final.json` ; limites et RETEX dans
[RESULTATS-MEMOIRE-22.md](RESULTATS-MEMOIRE-22.md). Aucun droit M20 public activé.
