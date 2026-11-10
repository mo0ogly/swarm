# IA2.3b — registre durable et publication récupérable

17 octobre 2026, Codex seul, `replan` + APEX, auto-revue locale `code-reviewer`.
Branche `codex/clear-product-method-names`, HEAD
`4e58f7cbf8200a58f69f1f4d03d657c3313fb642` avec changements locaux conservés.
**Module interne vérifié ; aucun lanceur, endpoint ou témoin externe déployé.**

## Résultat et frontière de preuve

Le cadre d'autorité peut maintenant être produit depuis un journal SQLite et
une tête Trust distincte, sous un checkpoint courant fourni par le contrôle.
Un consommateur interne relie cette source à l'attribution réelle d'instance
IA2.3a et au noyau IA2.2. La révocation durable d'un octroi fait refuser le
message suivant de la cage testée, sans révoquer sa liaison runtime en mémoire.
Cette observation prouve le passage par la source persistée ; elle ne prouve
ni arrêt du processus, ni admission publique ou effet métier.

Le permis de publication et le checkpoint sont des entrées privées du contrôle,
**émises par les fixtures dans ces essais**. Le jeton du publiant est réellement
résolu dans Trust ; cela ne transforme pas son principal local en humain ou
son rôle métier en droit de création. L'émission humaine de ces entrées, la
fraîcheur d'un témoin externe et la protection hôte des fichiers restent ouvertes.

## Choix, fichiers et compatibilité

Options comparées : lire la plus grande révision SQLite et en déduire les racines,
ou authentifier une tête indépendante avant d'en lire l'instantané exact. La
première ferait d'un import une autorité. La seconde est retenue, avec intention
durable et récupération explicite entre les deux stockages.

Six nouveaux fichiers `agent_identity_registry_{schema,types,store,publish,test,atomic_test}.go`.
Sources préexistantes modifiées : `model.go` (version 36 et écriture durable),
`store.go` (migration), `trust_actions.go` (champ/validation et encodeur commun),
`agent_identity_authority.go` (validation des enregistrements extraite sans changer
les invariants de transition). Contrats FR/EN, plan et dossier scientifique mis
à jour. [État initial](baseline.json), préimages et sources avant/après conservés.
Aucun fichier mémoire ou lanceur préexistant modifié par cette tranche.

La migration crée `agent_identity_snapshots` et `agent_identity_publications`
dans `state.db`, avec FK vers le travail et unicité des événements/têtes.
Une sauvegarde `state-pre-v36-*.db` est créée pour un stockage existant. Aucun
snapshot, octroi ou principal n'est ajouté par la migration. Les consultations
refusent une mise à niveau implicite ; la voie existante `swarm init` est celle
de la migration, à utiliser dans le parcours de livraison autorisé. Aucun init
ou changement du stockage vivant du dépôt n'a été exécuté ici.

## Protocole et données

1. Copier/décoder la proposition et vérifier le permis privé exact : projet,
   portée, digest, racines, principal publiant, révision Trust et événement.
2. Prendre Trust exclusif sans attente cachée ; un verrou occupé retourne
   `workspace_wait`. Prendre une transaction SQLite avec le contexte du consommateur
   et réserver l'écrivain avant lecture. Vérifier version et existence du travail.
3. Résoudre le jeton courant après cette attente, vérifier le checkpoint précédent,
   la chaîne, les racines, les liens et l'histoire non réécrite. Revalider le
   publiant juste avant la décision durable.
4. Persister dans Trust une intention contenant la tête et l'instantané exacts.
   Fichier et entrée de répertoire sont synchronisés. Toute lecture normale
   d'identité refuse une intention en attente.
5. Insérer snapshot et reçu en une transaction SQLite, puis COMMIT.
6. Finaliser la tête Trust et supprimer le marqueur en attente ; retourner le
   checkpoint cible seulement après succès de cette écriture.

La tête lie contexte de projet, portée, révisions/digest, racines, publiant,
événement et digest de la tête précédente. Le lecteur ne fait pas de
`SELECT max(revision)` pour choisir son autorité. Il exige le checkpoint,
vérifie la tête et la chaîne, lit le snapshot canonique dans une transaction
avec garde de version et en possède la copie. La connexion SQLite est libérée
avant le verrou runtime, puis le temps d'évaluation est échantillonné après
les attentes. Le message d'agent ne fournit ni le snapshot ni le cadre.

`recoverAgentIdentityRegistry` exige le permis initial et son contenu exact.
Il termine une intention existante ou vérifie sa finalisation déjà effectuée ;
il ne prépare pas une nouvelle publication. Après COMMIT, une reprise n'insère
pas une seconde fois les lignes. Si la configuration Trust a changé, elle est
conservée : la publication ancienne est historique, et son usage reste refusé
pour révision périmée. Aucun processus, consommation ou budget n'est recréé.

Ce protocole rend la publication récupérable ; il ne rend pas les deux fichiers
simultanément atomiques pour tous les usages. SQLite décrit ses propres
transactions et hypothèses de synchronisation, qui ne couvrent pas à elles seules
le fichier Trust. [Atomicité SQLite](https://www.sqlite.org/atomiccommit.html),
[transactions](https://www.sqlite.org/lang_transaction.html).

Schémas du contrat courant : [séquence FR](contract-fr-3.svg),
[EN](contract-en-3.svg), [commandes de rendu](diagrams.json). Les six diagrammes
FR/EN du contrat ont été rendus et inspectés visuellement dans cette zone.
Une note rognée a été répartie sur deux lignes puis revérifiée ; les
[quatre nouveaux rendus](diagrams-layout-fix.json) gardent la trace de cette
correction de présentation, sans changement des sources Go ni effacement des
commandes historiques.

## Contrôles effectivement exécutés

| Contrôle | Résultat | Originaux |
| --- | --- | --- |
| Sélection ciblée complète | **181 noms PASS / 181**, 42 tests racine, aucun FAIL/SKIP | [Commande/dates](tests-final-run.json), [JSONL](tests-final.jsonl), [résumé](tests-final-summary.json) |
| Nouveau registre | **33 noms PASS / 33**, dont invalides, reprise, restauration et cage réelle | Inclus dans la même exécution ; ne pas additionner |
| Concurrence registre/canal/runtime | **52 noms PASS / 52 sous `-race`**, aucun signalement | [Commande](race-final-run.json), [JSONL](race-final.jsonl), [résumé](race-final-summary.json) |
| Statique / compilation | `go vet ./internal/engine`, `go build ./cmd/swarm` : codes zéro | [Sorties](static-build-final.json), [binaire isolé non déployé](binary-final.json) |
| Candidat | **1 052 entrées** locales Go/tests/ressources embarquées, aucune modifiée pendant les checks | [Inventaire](inputs-final.json), `source-final/` |

La sélection est élargie pour une raison concrète : le contrat commun
`atomicWrite` et la version 36 touchent Trust, lancement, inspection/migration,
reprise et contexte mémoire. Les 133 contrôles de l'étape précédente sont rejoués,
avec les nouveaux tests et dépendances pertinentes. Les 52 sous `-race` répètent
un sous-ensemble, pas 52 attaques supplémentaires. Pas de suite globale,
fournisseur distant, revue indépendante ou déploiement revendiqué.

Cas observés : publication, réouverture et lecture ; copie sans alias ; rejeu
authentifié sans doublon ; refus du JSON forgeant un checkpoint, permis absent,
jeton faux, publiant différent même admin, contenu changé, racine invalide,
projet différent, événement réutilisé, effacement de révocation, cycle, contexte
absent/annulé et travail absent. Quatre reprises couvrent bootstrap/mise à jour
après intention et après COMMIT. Configuration modifiée préservée et droits
anciens refusés. Restauration SQLite seule ou conjointe Trust/SQLite refusée
**face au checkpoint courant gardé hors des fichiers restaurés**. Altérations
du snapshot/journal, ligne trop grande et version changée refusées. Deux écrivains
sur la même base n'obtiennent qu'une publication. Une attente annulée sur la
connexion libère Trust sans préparer d'intention. Une cage réelle reçoit une
décision, puis un refus d'octroi révoqué depuis la source durable ; aucun agent
métier n'est créé par ces appels.

## Défauts et améliorations conservés

| ID | Cause observée | Correction / preuve |
| --- | --- | --- |
| IA2-D01 | `atomicWrite` synchronisait le fichier, pas le répertoire renommé. Une création pouvait être annoncée réussie sans capacité d'ouvrir/synchroniser son parent. | [Échec avant](atomic-before.jsonl), [passage après](atomic-after.jsonl). Ouverture du parent avant mutation, synchronisation après rename ; tests contenu, mode 0600 et temporaires. Une erreur post-rename reste incertaine. Aucune coupure électrique simulée. |
| IA2-D02 | Décoder le précédent dans l'objet courant gardait son champ `previous_head_digest` lorsqu'il était absent du JSON précédent. Dès la révision 2, la chaîne était mal relue. | [Échecs initiaux](registry-initial.jsonl), [empreintes](registry-initial-run.json), `source-before-chain-fix/`. Décodage dans un objet neuf pour chaque enregistrement ; suites suivantes réussies. Le contrat générique de `strict` n'est pas changé en remplacement implicite. |
| IA2-D03 | Le FK révélait un travail absent seulement après l'intention durable, laissant du Trust modifié malgré le refus. | [Échec avant](missing-work-before.jsonl), [sources/commande](missing-work-before-run.json). Vérifier existence sous réservation SQLite avant intention ; refus sans mutation et motif exact vérifiés. |
| IA2-D04 | Un lecteur sur connexion ouverte acceptait encore un snapshot après passage du stockage à une version incompatible. | [Échec avant](version-before.jsonl), [sources/commande](version-before-run.json). Garde de version dans la transaction de lecture, motif `storage_version_conflict` vérifié. |

Les checkpoints intermédiaires 30 et 180 PASS restent historiques, pas les
preuves du candidat final. Les préimages originales et chaque source rouge
sont conservées en `.txt` ; aucun faux package Go n'est créé dans les preuves.

Le scan AST initial confondait des receveurs homonymes et des appels locaux avec
le CLI, et ignorait les frontières COMMIT. Les [versions et limites](lock-inventory-summary.json)
conservent l'amélioration : fichiers réellement compilés, receveurs/champs et
fonctions locales distingués, puis examen manuel. Le résultat final inventorie
1 938 fonctions et 83 fonctions à verrou/transaction, sans candidat restant
dans ce modèle limité. **Ce n'est pas une preuve globale d'absence d'inversion** :
callbacks et flot typé complet restent hors analyse. Ne pas appeler le nouveau
publiant depuis une transaction SQLite déjà tenue ; le raccord des exécuteurs
doit vérifier cette précondition.

Le pilote d'affichage du scan a aussi rencontré `null` pour une slice Go vide
et tenté `len(None)` ; JSON original et code de sortie Go conservés, résumé
normalisé sans relancer le moteur. Des recherches initiales visaient des noms
de fichiers absents ; l'inventaire/symbole réel a remplacé ces suppositions.
Ces erreurs de méthode ne sont pas des tests de sécurité.

L'index final a également révélé un comptage manuel erroné : le vérificateur
attendait 16 archives, l'inventaire exact en contient 17, toutes identiques aux
sources courantes. L'[échec original](verify-final-before-count-fix.json) et son
script sont conservés. Le vérificateur compare désormais l'ensemble des chemins
attendus et leurs octets, puis les 1 052 entrées du candidat. Propriétaire Codex,
valeur : éviter qu'un dénominateur recopié invalide ou masque une preuve ; critère
observable : concordance des chemins et empreintes, réalisée et vérifiée par
l'index final. Aucun test Go ni résultat produit n'a été modifié pour ce correctif.
Un ajout documentaire a d'abord été refusé pour un contexte de patch tronqué ;
la ligne réelle a été relue avant application, sans mutation partielle.

## Limites, droits et prochaine frontière

Le checkpoint courant est une **précondition fournie par le contrôle**. Options
examinées : signer un fichier voisin (n'empêche pas une restauration conjointe),
utiliser un témoin monotone indépendant (enrôlement/protection à déployer), ou
préparer une interface privée qui refuse l'absence de témoin sans activer les
parcours publics. Cette troisième voie progressive est réalisée ici ; les
fixtures fournissent le témoin. Restaurer tous les fichiers et aussi un ancien
témoin n'est pas détecté par magie. Propriétaire Codex pour le contrat, opérateur
pour l'enrôlement/déploiement ultérieur ; reprise après preuve de l'émetteur et
de sa fraîcheur, pas après simple relecture d'un fichier local.

Le permis exact n'a pas encore d'émetteur humain raccordé. La portée vient du
permis : le producteur du vrai plan adopté, l'accord humain lié au lancement et
sa consommation restent requis. La confidentialité des types Go n'est pas une
frontière contre un compte contrôleur hôte compromis. Le confinement fournisseur
et toutes les activations restent à raccorder. Les essais d'altération portent
sur des lignes/états de fixtures, pas une certification d'une image SQLite
hostile arbitraire, du matériel ou de tous les systèmes de fichiers.

IA2.3 reste partiel pour l'admission : réservation/départ incertain, effets,
processus redémarré et récupération complète non réalisés. Les consommateurs
bas niveau IA2.2/IA2.3a prenant encore des snapshots/cadres restent des primitives
à précondition de fraîcheur, pas des endpoints publics. IA3 devra employer le
parcours durable et couvrir CLI/API, autres fournisseurs et activation hors plan.
IA4 droits mémoire, IA5 qualification IDA et RETEX restent ouverts. F01–F07 ne
sont pas requalifiés, aucun scénario global IDA ne devient PASS par ces tests.

Index des contrôles et empreintes courantes : [vérification finale](checks-final.json).
[Auto-revue finale locale, non indépendante](review-local-final.md).
