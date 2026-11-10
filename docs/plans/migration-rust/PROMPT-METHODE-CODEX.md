# Prompt pour Codex — méthode de migration plus efficace

Rédigé par Claude à la demande de l'utilisateur (22 octobre 2026). Les sections
marquées **[décision]** changent des décisions consignées en tête de `PLAN.md` :
elles ne valent que si l'utilisateur envoie le prompt avec elles.

---

Codex, la migration Go → Rust avance trop lentement par rapport au travail
restant : environ 25 routes API sur 91 et 17 commandes CLI sur 40 sont portées
(mesure par nom, `CHEMIN-CRITIQUE-SPEC.md` § 3 bis et 3 ter). Aucun lot R0–R7
n'est clos, et R6 et R7 ne sont pas commencés. Le coût vient moins de la
traduction que de la méthode : parité octet par octet partout, preuves
maximales pour chaque micro-étape, durcissement au-delà du Go, et un `PLAN.md`
de 18 000 lignes. Le lot 201 compte à lui seul 377 éléments de preuve. Applique
désormais la méthode suivante. Les règles communes d'`AGENTS.md` restent en
vigueur : replan, gestion des erreurs, documentation, aucun réglage caché.
Cette méthode les applique, sans les assouplir.

## 1. Classer chaque surface avant de la porter [décision]

- **Contrat — parité stricte (RM-02 inchangée)** :
  - données sur disque (`state.db`, JSON du Work, migrations) ;
  - sorties JSON de la CLI consommées par des agents ou des scripts ;
  - refus et codes de sécurité, d'identité, de budget et de droits ;
  - contrat de lancement et de supervision des agents.
- **Réécriture par parcours** : cockpit web et pages, préparations,
  automatisation, aide, console, rapports. Tu écris la fonction depuis le
  besoin utilisateur, pas depuis le code Go. L'acceptation se fait par tests
  de parcours (web, CLI, API), plus une inspection visuelle pour l'interface.
  Un écart de forme avec le Go est permis s'il est documenté ; un écart de
  sens, de droits ou de données ne l'est pas.
- Consigne ce classement pour chacun des groupes G1–G12 dans
  `CHEMIN-CRITIQUE-SPEC.md`, avant le premier lot.

## 2. Automatiser la parité au lieu de la démontrer à la main

Pour les surfaces « contrat », construis un banc différentiel :
- un corpus de requêtes et de Roots de test ;
- le même scénario exécuté par le binaire Go et par le binaire Rust ;
- les sorties, refus, codes et l'état de la base comparés automatiquement,
  avec une liste explicite et justifiée des différences admises.

Un écart non listé fait échouer le banc. Ce banc remplace les contre-exemples
écrits un par un pour prouver la parité. Chaque commande ajoutée au corpus
devient une garde de régression.

## 3. Lots verticaux, par parcours utilisateur

Un lot porte un parcours complet : route, commande CLI, interface, données et
test du parcours, ensemble. Il est fini quand ce parcours fonctionne en Rust
de bout en bout. Plus de lot consacré à une seule garde interne, sauf défaut
réel constaté. Ordre : les groupes dont dépendent les autres d'abord, puis les
plus gros en volume (préparations, automatisation, cockpit).

## 4. Preuves proportionnées au risque

- **Niveau A** (sécurité, identité, budget, données, processus agents) :
  recette sur instantané figé, témoin valide avant chaque refus, revue
  indépendante.
- **Niveau B** (logique moteur sans effet de sécurité) : tests unitaires et
  d'intégration ciblés, puis une recette de groupe en fin de lot.
- **Niveau C** (interface, lecture, aide) : test de parcours et inspection
  visuelle.

**Une recette complète par lot, à la fin**, pas une par micro-étape. Entre-temps,
seulement les tests ciblés des fichiers modifiés. Un même point en échec trois
fois de suite déclenche `replan` avec réduction du périmètre, pas une
quatrième recette identique.

## 5. Durcissement au-delà du Go : différé

Un durcissement qui n'existe pas dans le Go n'entre dans un lot que s'il
corrige un défaut réel reproduit, ou s'il est exigé par une surface
« contrat ». Toute autre idée de durcissement va dans un registre
« après bascule » (origine, valeur, condition de reprise). Rien ne disparaît.

## 6. Plan court, preuves dans les fichiers

- `PLAN.md` garde, par lot : objectif, parcours, niveau de preuve, état, lien
  vers le dossier de preuves, blocages. Une quinzaine de lignes, pas plus.
- Le détail (sorties, hachages, contre-exemples) vit dans
  `evidence/<lot>/`.
- Tiens à jour un tableau d'avancement : par groupe G1–G12, routes et
  commandes « portées en parité vérifiée » (banc différentiel) ou « acceptées
  par parcours ». Ce tableau est complété par les taux et l'estimation du § 9.

## 7. Agents en parallèle sans collision

Attribue à chaque agent Codex un groupe et ses fichiers propriétaires. Les
fichiers partagés (`lib.rs`, `root_store.rs`, `engine_settings.rs`,
`service.rs`, catalogue de réglages) sont modifiés de façon additive, annoncés
dans `PLAN.md` avant écriture, et gelés pendant la recette d'un autre agent.
Les zones LIVE-UX de Claude (`telemetry_*.rs`, `live*.rs`, `web/live*`) restent
coordonnées dans `PLAN.md`.

## 8. Bascule [décision]

Garde la bascule finale unique pour toutes les écritures (pas de double
écrivain). Prépare R6 et R7 dès maintenant, en parallèle des groupes :
- outil de migration des données, avec restauration vérifiée ;
- procédure de retour arrière ;
- installation.

Sans les achever : leur retard ne doit pas devenir le dernier goulot.
Optionnel, à confirmer par l'utilisateur : servir en lecture seule les
surfaces Rust acceptées sur une Root de test, pour obtenir un retour d'usage
avant la bascule.

## 9. Avancement visible à chaque lot (ajout du 23 octobre, demande utilisateur)

L'utilisateur doit connaître l'avancement sans le demander. À la fin de chaque
lot, en tête de `PLAN.md` :
- **trois taux** recalculés depuis la matrice (`build_behavior_matrix.py`,
  régénérée) : *existe en Rust*, *vérifié localement*, *parité acceptée*, pour
  la CLI (opérations), le web (routes et pages), les données (migrations) et la
  bascule ;
- **le découpage restant** : chaque groupe G1–G12 et R6/R7 découpé en tranches
  verticales, avec leur niveau de risque et une estimation en tranches ;
- **la vitesse observée** : tranches livrées par jour sur les derniers lots, et
  une fourchette de durée restante qui en découle, marquée comme estimation.

Outil fourni : `tools/migration-progress/avancement.py --record` calcule les
trois taux depuis les aides Go/Rust, `route()` et la matrice, écrit
`AVANCEMENT.md` et un historique daté d'où sort la vitesse. Il refuse
d'enregistrer avec un binaire Rust plus ancien que les sources. Il mesure
aussi la taille du code Rust/Go par domaine (hors tests et commentaires) :
un rapport nettement supérieur à 1,5 sur un domaine porté signale du
durcissement ou du code défensif à reporter dans AFTER-CUTOVER. À toi d'y
déclarer chaque preuve PASS (`preuves.json`) et les équivalents renommés
(matrice), sans quoi le taux reste sous-estimé.

Si une estimation n'est pas défendable pour un groupe, dis-le pour ce groupe
et donne la condition qui la rendra possible. Ne la remplace pas par un
silence global.

## 10. Cadence et consommation — décision du 23 octobre 2026

L’utilisateur confirme l’inscription de ces règles dans le nouveau plan actif :

1. Assembler les composants existants pour livrer une capacité Rust utilisable.
   Prochain résultat : `mission status WORK --json` sur un parcours ordinaire
   défini, avec les champs du contrat historique réellement produits.
2. Pendant le développement, exécuter les vérifications ciblées et réutiliser
   les preuves dont les entrées pertinentes restent inchangées.
3. Regrouper la recette complète, la qualification finale du binaire, la revue
   indépendante requise et le bilan à la fin du parcours. Une nouvelle erreur
   ou un contrat affecté impose les vérifications nécessaires, sans mini-projet
   documentaire systématique pour chaque retouche.
4. Déléguer seulement des tâches indépendantes à fichiers réservés, avec un
   contexte court et les références utiles ; ne pas multiplier les agents par défaut.
5. Conserver les améliorations non bloquantes pour après la bascule, avec
   origine, propriétaire et condition de reprise. Corriger les défauts réels et
   préserver les contrats de données, sécurité, droits, budgets et erreurs.
6. Mesurer les capacités utilisables vérifiées et le coût observé. Les dossiers
   de preuves et le nombre de tests ne sont pas de l’avancement fonctionnel.
   Déclarer les tokens indisponibles et ne pas annoncer d’accélération ou de
   durée restante sans mesure défendable.

Les docstrings et la documentation FR/EN restent incluses dans le même incrément.
Le parcours est clos après son exécution publique et ses contrôles requis ; un
SDK ou une garde interne seuls ne suffisent pas. Cette décision ne change ni
les critères de sécurité ni les autorisations de publication ou de bascule.

## Ce que j'attends en retour

1. Le classement contrat / réécriture des groupes G1–G12, et ton désaccord
   motivé si un classement te paraît dangereux.
2. Le plan du banc différentiel : premier corpus, formats comparés,
   différences admises.
3. L'ordre des prochains lots verticaux, avec leur niveau de preuve.
4. Le lot 201 clos ou réduit sous cette méthode, avec la raison.

L'estimation de durée demandée au § 9 reste une estimation, jamais un
engagement. N'élargis pas le périmètre et n'accorde aucun droit implicite.

## Correction de cadence — délégation GLM (23 octobre 2026)

Une vague de migration doit produire une modification intégrée du parcours choisi et sa vérification, ou un blocage concret diagnostiqué. Huit réponses de conseil ne sont pas huit tâches migrées. Réserver les tâches indépendantes de code/tests/docs, fournir les fonctions/types nécessaires, demander un livrable applicable ; Root relit, intègre et exécute le parcours. Mesurer séparément appels complets, propositions retenues, code intégré et effet public vérifié. Sans accès outils, GLM propose seulement les octets : Root porte édition, tests et clôture. Ne pas multiplier analyses ou appels pour occuper huit rôles ; réutiliser les preuves et propositions valides. Contextes ciblés, jamais un fichier entier par commodité ; transmettre les corps utiles sans tronquer leur contrat.

Pour une proposition de code, joindre aussi les définitions réelles de chaque DTO/enum et les signatures des helpers consommés ; contrôler ce paquet avant l’appel. Un contrat en prose n’est pas un substitut aux types. Rejeter code inventé/placeholder, compléter le contexte ou intégrer directement dans les droits existants ; ne pas compter une réponse « complète » comme une tâche intégrée. Retenir le dénominateur : appels, propositions applicables, intégrations et parcours vérifiés sont quatre mesures distinctes.

### Apprentissage473 : contexte par livrable
Une petite extension de recette a répété environ6,4ktokens de contexte sur huit appels. Fournir seulement les extraits nécessaires au rôle ; inclure le vrai appelant pour une proposition d'intégration. Les contextes de documentation/comptage n'ont pas besoin de tout le producteur Rust. Conserver séparément réponses complètes, propositions adoptées, fonctions produit ajoutées et parcours prouvés ; une extension de preuve n'est pas une nouvelle fonction migrée. Amélioration décidée, effet sur prochaine campagne non encore mesuré.

### Précondition477 — vérifier les extraits avant HTTP
Avant une délégation de code/revue, vérifier que le contexte produit contient les corps et ancres attendus et n'est pas vide ; conserver tailles/empreintes par rôle. Une extraction vide refuse le lancement local, pas une demande payée au modèle pour redécouvrir ce manque. Origine : result-review477, contexte vide constaté après appel ; avis non intégré, lecture source reprise par Root sans retry payé. Condition future, pas preuve rétroactive ni économie démontrée.

### Non-régression485 — ajouter sans retirer les lectures antérieures
Quand une garde de statut est élargie, couvrir un ancien état accepté avant les nouveaux cas. Un refus lié à une capacité non portée ne doit pas retirer par accident une lecture de métadonnées qui n’utilise pas cette capacité. Origine : predicate repository479 trop large ; RED485 conserve le contre-exemple, correction486 borne le refus aux historiques avec Agents. Distinguer preuve du garde pur et support public d’initialisation/exécution managed, toujours non établi.

### Contrats de fonctions498 — fournir aussi les préconditions documentées
Pour un producteur/consommateur partagé, fournir son Rustdoc/docstring avec le corps et vérifier les branches maintenant admises. Une extraction qui commence à pub fn peut omettre une précondition devenue fausse. Origine : result_inputs498, ligne de Rustdoc complétée pour running/queued ; diff exact conservé, corps fonctionnels identiques à494. La règle est persistante pour les prochaines préparations, ne réécrit pas les contextes GLM490 déjà terminés.

### Apprentissage532 — déplacer un target sans recopier
Le reproducteur permanent démontre que shutil.move peut recopier tout Cargo
si le parent destination manque. Utiliser move_build_target.py après arrêt de
nos writers : parent créé, rename seul, inode identique, EXDEV sans fallback.
Ne pas contourner refus/existing destination, ne pas effacer preuves/autrui.
Cette amélioration est vérifiée sur fixtures ; prochain transfert réel doit
conserver son témoin inode. Aucun temps gagné chiffré sans mesure.

Vague520 : contextes entiers trop larges et deux réponses tronquées ; ne pas
relancer ni augmenter simplement la sortie. Fournir une fonction/module borné,
les types/helpers/appelants exacts, et demander un seul livrable applicable.
Un helper privé forgeable depuis son propre code n’est pas une vulnérabilité
produit sans entrée réelle : rejeter le durcissement sans contre-exemple.
Égalité de deux lectures n’établit pas leur fraîcheur ; assert champ/effet réel.
