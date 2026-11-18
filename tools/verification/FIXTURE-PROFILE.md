# Qualification de compilation du profil fixture

`qualify_fixture_profile.py` prépare une seule cible de test d’intégration Cargo
avec `--offline --locked --no-run`. Aucun corps de test n’est exécuté. Le cache
peut être modifié par Cargo ; les dépendances nécessaires peuvent être recompilées.
L’adaptateur ne programme pas de build d’image CLI standard gérée.

Le coordinateur autorise séparément chaque compilation réelle. Les tests inertes
ne constituent ni cette autorisation, ni une preuve de compilation native.

## Entrées explicites

La CLI exige `--config CHEMIN --config-sha256 EMPREINTE` ; aucun drapeau de
substitution des helpers n’est exposé. Le JSON version1 exige exactement :
`schema_version`, `root`, `frozen`, `workspace_root`, `target`, `output`,
`manifest_path`, `source_map`, `registry_map`, `cargo_lock`,
`external_targets_policy`, `space_policy`, `package`, `test`, `features`.
Les cinq bindings sont des objets `path`/`sha256`. La carte source associe les
chemins relatifs à `frozen` à leurs SHA-256 ; la carte registre utilise des
chemins absolus. Le verrou doit être `workspace_root/Cargo.lock`. Les politiques
persistantes restent sous `root/.cargo`. La cible externe doit être déjà admise ;
l’adaptateur n’enregistre aucune cible et n’abaisse pas la réserve.

Le répertoire de sortie doit être neuf, hors dépôt/source/cible, sur le montage
admis. Admission et réserve sont vérifiées avant sa création. Le descripteur du
support est détenu pendant le processus, les postcontrôles et la publication.
Préserver séparément les images standard déjà qualifiées et les anciens artefacts
référencés avant toute réutilisation séquentielle du cache. Le chemin du cache
n’est pas une réservation contre tous les producteurs externes.

## Preuves et erreurs

Les reçus lient configuration, cartes brutes, sources avant/après, politiques,
stdout/stderr, résultat du helper, artefacts Cargo et profil du test sélectionné.
Les chemins relatifs des depfiles sont résolus depuis le workspace Cargo.
L’inventaire reste l’union observée des depfiles du cache, pas une fermeture
exhaustive des entrées du binaire. Les fichiers générés observés restent distincts
des entrées préliées. L’ELF retenu est copié, relu et rendu exécutable au seul
propriétaire ; il n’est pas installé.

Une publication utilise un temporaire adjacent synchronisé puis un lien exclusif.
L’échec après création du lien conserve une observation d’application incertaine,
pas une affirmation de rollback. Les causes de lecture, processus, stockage et
publication restent conservées, y compris les échecs secondaires. Un code enfant
connu reste enregistré même si le helper lève une exception ; il ne constitue pas
le succès du contrôleur. Aucun timeout global, retry ou marge supplémentaire
n’est ajouté. Le mode offline de Cargo ne constitue pas un confinement réseau.

## Vérification et limites

La recette maintenue `recipes/fixture-profile-inert.json` vérifie les gardes avec
des fichiers synthétiques et des helpers injectés. Elle n’appelle aucun compilateur,
provider ni service. Son résultat doit être conservé puis vérifié par `recipe.py`.
Les refus de publication et les dérives ont des témoins valides dans les tests.

Cette preuve ne qualifie pas le graphe C/en-têtes/système, les effets de scripts de
build/proc-macros, un runtime, une installation ou une acceptation métier. Une
revue indépendante et une allocation séparée restent nécessaires avant l’usage
natif prévu. Voir [English](FIXTURE-PROFILE.en.md).

Les depfiles et fichiers générés sont archivés depuis le tampon exact utilisé
pour leur empreinte et leur analyse, sans seconde lecture de la source. Cela
ne garantit pas que la source reste immuable après cette observation.

Les composants `..` des depfiles sont résolus depuis le workspace Cargo avant
classification. Leur présence seule ne prouve pas une sortie du périmètre : la
cible canonique doit rester déclarée et hashée, ou être un généré du cache admis.
Les chemins externes non liés, y compris via lien symbolique, restent refusés.
