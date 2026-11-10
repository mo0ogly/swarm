# PAEA — incrément 12 : vraie CLI sans nouvelle compilation

[English](../../../en/plans/actions-confiance/paea/INCREMENT-12.md) ·
[Plan](../PAEA-APEX.md) · [Contrat](../../../architecture/PAEA-CONTRACT.md)

## Reprise et choix

L’utilisateur demande « next ». Codex racine poursuit seul, dans le checkout
direct, sous la consigne **sans compilation ni redémarrage**. Les 25 entrées de
l’incrément 11 sont initialement inchangées. Les sources de l’écran/API sont
écrites, mais le service natif n’est pas qualifié ; les sept tests Rust du rapport
v2 et les deux tests HTTP restent non exécutés.

Précondition nouvelle : une autre session a construit le candidat **dev77** avec
les sources du rapport v2. Deux options : attendre un build autorisé dans cette
session, ou qualifier ce candidat existant puis l’écran avec un pont de fixture.
Choix : seconde option, sans installation ni confusion avec le service natif.
L’incrément modifie seulement tests, recettes et documentation, aucun code produit.

## Artefact et provenance

Preuves conservées sous `evidence/20261025-existing12-23lstywl/`.

- Binaire figé `existing-swarm`, SHA256
  `7e7563dd5e1bdb848fec42f6ae2e619c31b048889221191517553fe45e56f6d3`.
- Version conservée : `v0.1.1-dev.77`, commit déclaré
  `d566ed35c42a362f59e48a025d58b2b68a25b179`, modifié, build
  `2026-10-25T21:54:26Z`. Cela identifie l’artefact, pas le serveur en service.
- Résultat, argv, métadonnées, stdout/stderr Cargo, inventaires, version et archive
  du build d’origine copiés dans `retained-existing-build/`, sans lancer Cargo.
- **434 membres d’archive vérifiés** contre les hashes retenus ; sept sources
  PAEA actuelles correspondent (six fichiers du contrat et l’adaptateur CLI).
- Profil observé non test, aucune feature fixture ; contrat PAEA sans feature,
  Identité avec `serde`. Les quatre artefacts « core » du collecteur ne décrivent
  pas à eux seuls l’unification de toutes les dépendances : messages Cargo relus.
- L’archive ne contient pas la nouvelle route PAEA ni ses assets : aucune preuve
  d’API/UX native installée n’en est tirée.

Incident conservé : le chemin `artifact` de `deploy/local-build.json` désignait
une copie candidate déjà absente. Le résultat original du build nomme la copie
conservée par contenu, dont l’empreinte correspond ; cette copie a été figée dans
nos preuves. Aucun retry du chemin absent ni nouveau build. Une lecture de version
avec un chemin relatif depuis `/tmp` a aussi refusé avant exécution ; les tests
utilisent ensuite exclusivement le chemin absolu figé. Ces erreurs de méthode
n’effacent pas les observations originales et ne sont pas des défauts PAEA.

## Résultats

| Parcours | Résultat | Limite |
|---|---|---|
| CLI public `paea-public-existing-binary` | **18 tests PASS**, 973 invocations : 705 succès, 268 refus attendus ; binaire stable | Python sur artefact existant, pas Cargo ni test Rust exécuté. |
| Exactitude v2 | 331 appels `receipt-report` ; deux domaines, FR/EN, hashes exacts, refus/reprise, sortie `/dev/full` | Correspondance de données, aucune authentification du reçu ou observation d’effet. |
| Compatibilité v1 | 231 appels `check-receipt`, forme entière vérifiée ; sorties identiques sur variation d’observation | Limite historique v1 conservée intentionnellement, pas d’évolution implicite de son schéma. |
| Autres contrôles sur ce nouvel artefact | 232 canonicalisations, 163 comparaisons, 16 refus d’opérations actives | Réexécution justifiée par l’identité différente du binaire testé ; aucune reprise de mission. |
| Écran → vraie CLI | **174 assertions PASS, 48 appels** : 36 succès/12 refus attendus | Pont HTTP de fixture avec session de test, pas le service HTTP natif. |
| Trois recettes conservées | **PASS et intégrité VERIFIED** | Inventaires déclarés et archive retenue, pas acceptation indépendante. |
| Composant sur catalogue courant | **219 assertions PASS, 40 requêtes simulées** | Rejeu justifié par une modification concurrente de traduction, distinct des 174 assertions sur vraie CLI. |
| API native, installation, invariants de bout en bout | **NON TESTÉ / NON INSTALLÉ** | Deux tests HTTP Rust non exécutés ; aucun redémarrage autorisé. |

Les quatre nouveaux tests publics couvrent : rapport complet v2 et v1 conservé,
distinction document/reçu/observation/format, entrées malformées/manquantes/doublons,
liaison exacte, états `UNKNOWN` et terminaux, options runtime/stdin/arity refusées,
écriture en échec et reprise. Chaque défaut est précédé d’un témoin valide ; chaque
invocation conserve argv, entrées et flux bruts, et vérifie l’inventaire/octets des
fixtures après appel. Aucun Root réel, fournisseur, politique ou effet consulté.

`tests/paea_cli_ui_test.cjs` sert les assets du workspace sur une adresse locale
éphémère et transmet une seule opération fixe à la CLI. La réponse est celle du
binaire, sans remplacement métier. Dans les deux langues et domaines, l’écran
exerce : rapport exact, service incorrect/refus/reprise, observation distincte,
`UNKNOWN`, réconciliation, transition interdite, reçu JSON invalide et document
reformaté. Les rapports sont rapprochés des vecteurs indépendants et octets
originaux. La réutilisation initiale de la preuve des 219 assertions de 11 a été
refusée en contrôle final : une autre session avait changé le catalogue partagé
après les tests du pont. Le diff conserve une aide de préparation remplacée,
aucune valeur PAEA modifiée. Le composant a donc été **rejoué : 219 assertions,
40 requêtes simulées PASS** sur le catalogue courant. Ce résultat couvre rendu,
FR/EN, thèmes/mobile/clavier et pannes simulées ; il reste distinct des 174
assertions avec la vraie CLI. Originaux et refus de réutilisation conservés.
Le pont reste lié au catalogue archivé précédent ; ses libellés PAEA sont
identiques, le composant courant complète cette preuve sans refaire les 48 appels.

Recettes réutilisables : `paea-public-existing-binary.json` (18 tests),
`paea-cli-ux.json` et `paea-ux.json` (composant simulé). Exiger le binaire et son hash explicites ; fournir un transcript
neuf pour la première et `SWARM_PAEA_UI_EVIDENCE` neuf pour la seconde. Parents
existants, `recipe.py run --output` neuf, puis `recipe.py verify`. Aucune borne
globale, aucun shell implicite, aucun retry ni buffer maximal caché du sous-processus
dans le pont ; les flux sont lus en continu.

## Revue et apprentissage

`source-review.json`, `static-checks.json`, `owned.diff` et l’inventaire final lient
la revue aux octets. Revue du même auteur ; aucune délégation ni revue indépendante.

- Politiques : comptage 18, noms de cas, port éphémère, CSRF de test, codes
  0/2, HTTP 200/400 et domaines sont des paramètres/attentes du banc. Le pont
  n’institue aucune politique produit et n’accepte aucune commande arbitraire.
  Il reçoit un exécutable explicitement identifié, une opération fixe, des fichiers
  qu’il possède et des données de service/état ; pas d’option shell ou de Root.
- Fonctions : `invoke_files` autorise seulement la troisième opération hors ligne ;
  nouveau helper `reported` et quatre méthodes documentés. Helpers du pont et
  assert complet documentés ; syntaxe Python/JS et contrats consommés revus.
- Erreurs : cause de sous-processus conservée ; sortie 2 et stderr JSON deviennent
  refus HTTP de fixture, jamais succès vide. Une erreur du pont produit 500 et
  un échec du test. Après refus, l’UX supprime tout ancien rapport, garde les
  saisies et permet la reprise explicite. Aucun effet n’a lieu à remettre.
- Documentation FR/EN actualisée : le v2 n’est plus décrit comme non exécuté
  partout ; sept tests Rust restent non lancés, et preuve CLI/pont/session native
  sont distinguées. **Pas de note de livraison ajoutée à `version_history.json`** :
  tests/documentation seulement, aucun changement produit construit ou livré ici.

Apprentissage : un chemin candidat périssable ne suffit pas à retrouver un
artefact ; utiliser le résultat retenu, vérifier le hash, copier les entrées de
preuve avant exécution. Propriétaire vérification ; appliqué ici, sans modifier
la métadonnée d’une autre session ni prétendre garantir sa durée de conservation.

PAEA-OBS-01 v2 est **vérifié localement** pour l’identité des octets : des reçus
distincts qui peuvent donner le même v1 donnent des rapports v2 distincts. Cela
ne transforme ni hash en signature ni cohérence en effet établi. La limite v1 et
ses originaux restent conservés. PAEA-UX-02 reste dans le suivi de 11.

Reprise restante : levée explicite de la consigne pour qualifier/typer le service
HTTP natif et ses assets réellement servis, puis livraison locale vérifiée ;
PA-02…07 pour les producteurs protégés et accords réels, tiers pour E3. Aucun
des **17 invariants n’est démontré de bout en bout** par ce nouveau banc ; PA-08,
installation, E2/E3 et RETEX restent ouverts. Les travaux autorisés indépendants
restent possibles, sans simuler une acceptation ou étendre les droits.
