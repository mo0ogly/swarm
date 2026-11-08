# Stockage des compilations locales

[English](BUILD-STORAGE.en.md)

Les futurs appels Cargo depuis ce checkout ou ses sous-dossiers chargent
`.cargo/config.toml`. Les profils `dev` et `test` utilisent `debug = 0` et
`incremental = false` : moins de symboles de débogage et aucun cache incrémental
rustc. Optimisation, assertions de débogage, contrôles de débordement, gestion des
paniques et features ne sont pas modifiés. Le profil `release` n'est pas modifié.
Les sorties précédentes ne sont ni supprimées, ni déplacées, ni réécrites par
cette configuration. Une nouvelle combinaison de paramètres entraîne une
recompilation ; les anciens artefacts ne disparaissent pas automatiquement.

## Admission avant compilation

Cargo utilise `tools/verification/cargo_space_guard.py` comme `rustc-wrapper`.
Avant chaque commande ayant `--out-dir`, le garde lit
`.cargo/storage-policy.json` et vérifie les octets disponibles pour l'utilisateur
sur le volume du checkout et celui des sorties. La décision utilisateur du
7 novembre 2026 fixe `minimum_free_bytes = 21474836480`, soit **20 Gio**
(environ 21,5 Go décimaux). Sous cette réserve, le compilateur n'est pas démarré,
le refus explique la mesure et le seuil, et Cargo termine en échec. Les sondes
de découverte du compilateur restent possibles. Une mesure impossible ou une
politique absente/invalide refuse la compilation, sans seuil de repli.

```sh
python3 tools/verification/cargo_space_guard.py --status
python3 tools/verification/cargo_space_guard.py --set-minimum-free-bytes 21474836480
```

Le fichier JSON est la seule source de ce seuil. L'écriture CLI est atomique et
synchronisée : une erreur avant remplacement conserve l'original ; une erreur
de synchronisation après remplacement n'implique pas un retour arrière.
Cette politique concerne le poste et son outillage de compilation, pas les
budgets, délais ou politiques d'exécution du moteur Swarm dans Admin/API/CLI.

Le garde ne supprime rien, ne surveille rien en arrière-plan et ne termine aucun
processus en cours. Il ne change ni les cibles choisies explicitement par les
recettes ni la séparation des builds de production et de fixtures. Les appels
hors de l'arborescence utilisant seulement `--manifest-path` doivent charger
explicitement ce fichier de configuration avec `--config`, ou être lancés depuis
le checkout. Les surcharges Cargo de profil/wrapper par environnement ou CLI
restent possibles : leur consommation doit être conservée dans les preuves.

## Limites et reprise

**Ce contrôle d'admission n'est ni un quota ni une réservation atomique.** Un
compilateur déjà admis, des compilations concurrentes, un build script, rustdoc,
un autre programme ou une commande directe contournant Cargo peuvent consommer
la réserve après la mesure. Les processus déjà démarrés restent inchangés.
Il ne garantit donc pas que le filesystem ne pourra jamais être plein.

Après refus, libérer uniquement des caches inutilisés avec le vrai verrou du
producteur, ou choisir un volume adapté. Reprendre avec les mêmes 20 Gio : ne
pas diminuer le seuil pour faire passer une preuve. Aucune purge automatique,
suppression de preuves ou modification du service actif n'est impliquée.
Pour un débogage source explicite, on peut choisir un profil avec symboles
(`CARGO_PROFILE_DEV_DEBUG=2` / `CARGO_PROFILE_TEST_DEBUG=2`), en conservant la
réserve et en assumant le coût disque. Ne pas changer un profil en cours de test.

Un plafond strict exige un filesystem/quota dédié aux sorties : préparer un
nouveau volume Unix de taille bornée, vérifier les verrous Cargo et les recettes,
puis raccorder seulement les nouvelles exécutions. Ne pas utiliser FAT32 pour
les builds ni SSHFS sans qualification des verrous. Ne pas déplacer les sorties
d'une exécution active. Sur ce poste, l'activation d'un montage dédié exige une
authentification administrateur non disponible à l'agent ; ce plafond n'est pas
installé. Réduire les symboles seul ou répéter des nettoyages ne prouve pas ce
plafond. Les chemins `debug/deps` de plusieurs consommateurs interdisent aussi
de modifier globalement `build.build-dir` sans adapter et vérifier ces parcours.

## Vérification et provenance

La recette permanente `recipes/build-storage.json` couvre le refus sans lancement,
la reprise, les frontières, les erreurs de mesure et d'écriture, la configuration
persistée, les paramètres réellement consommés par Cargo et les entrées de
qualification. Les seuils de fixture sont des données de test isolées ; ils ne
modifient jamais la réserve du checkout.

`verify_native_profile.py` pré-lie et archive aussi les octets du garde et de sa
politique : leur changement ou disparition invalide une réutilisation. Les
preuves historiques ne sont pas réécrites et une ancienne qualification n'est
pas une qualification des nouveaux paramètres. Le garde n'est pas une frontière
de sécurité et sa présence ne prouve ni une livraison native ni une acceptation.

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/build-storage.json --output .cache/verification/build-storage-NEW
python3 tools/verification/recipe.py verify --evidence .cache/verification/build-storage-NEW
```

Utiliser un dossier neuf sur filesystem Unix, puis archiver les preuves vérifiées
sur la clé USB avec un index local. Le protocole de publication de la recette
emploie des liens Unix, incompatibles avec un dossier de preuves direct sur FAT32.
Voir aussi [le guide de vérification](README.md) et
[la documentation Cargo](https://doc.rust-lang.org/cargo/reference/profiles.html).
