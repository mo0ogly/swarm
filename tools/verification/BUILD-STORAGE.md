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

## Cibles externes temporaires enregistrées (STOCK-EXT-01)

Le disque externe reçoit uniquement preuves, fixtures et caches/sorties temporaires
de compilation. Le checkout, `.git`, configurations, données de missions, service
et binaire installé restent locaux. `verify_native_profile.py` copie et vérifie
toujours l’image gelée dans `.cache/rust-migration-binaries` locale ; le builder et
le lanceur qualifient cette copie locale. Aucun service n’est installé depuis le
cache externe.

Par défaut la cible reste dans le checkout. Une cible externe exacte doit être
enregistrée localement dans `.cargo/external-targets.json` (fichier opérateur non
versionné, sans secret). Sélection du chemin : `deploy/local-web.json`
`rust_target_dir`, surcharge historique `SWARM_RUST_TARGET_DIR`, ou argument
explicite du builder. La sélection ne confère aucune admission. Enregistrement :

```sh
python3 tools/verification/build_storage.py --root /CHECKOUT register --target /MONTAGE/cache/cible --mountpoint /MONTAGE --uuid UUID_OBSERVE --fstype ext4
```

Le registre contient `schema_version:1` et `targets`, avec `path`, `mountpoint`,
`uuid`, `fstype`. Chemins absolus canoniques, clés inconnues/doublons, propriétaire,
droits du fichier, liens, montage réel et identité sont validés. Le writer prend
un verrou coopératif non bloquant ; refus sur concurrence, sans effacement du
verrou. L’écriture adjacente est relue/synchronisée puis remplacée atomiquement.
Un échec avant remplacement préserve le précédent registre et conserve le
temporaire pour diagnostic ; un échec après remplacement ne signifie pas rollback.
Les modes des répertoires existants sont conservés ; ce dispositif ne garantit
pas la disponibilité face à un acteur pouvant modifier le checkout lui-même.

`build_storage.admitted()` est l’unique entrée publique de durée de vie. Builder
et collecteur tiennent leurs descripteurs jusqu’à la fin du sous-processus :
UUID/fstype, `findmnt` exact, rw/sans noexec, major/minor, dev/inode de l’ancre et
de la cible, chemin et octets du registre/méthode sont contrôlés avant Cargo puis
après exécution. Traversée par descripteurs avec O_NOFOLLOW ; composant d’un autre
périphérique refusé avant Cargo. Aucun repli sous un montage absent. Le FD empêche
un démontage ordinaire ; ce n’est ni un confinement contre le même UID ni une
garantie contre détachement forcé/lazy privilégié. Un refus après exécution
conserve des sorties non qualifiées et ne prouve pas absence d’écriture.

Python 3.11+ est requis pour conserver les groupes de causes. ext4 est la portée
technique qualifiée par les témoins Unix ; un autre filesystem exige sa propre
qualification, ce n’est pas un plafond de politique opérateur. Le garde de réserve
hôte/sorties reste inchangé. Les empreintes du registre et du helper sont liées
aux entrées effectives/archives ; invocation et résultat conservent aussi les faits
d’admission. `deploy/local-web.json` complet n’est pas archivé par ce mécanisme.

Recette permanente : `recipes/external-build-storage.json`. Ses simulations ne
prouvent pas un vrai montage ou un vrai build ; les témoins réels et le profil
Cargo effectivement exercé restent des preuves séparées. Les historiques locaux
sans registre externe restent compatibles ; aucune configuration absente
n’autorise une cible externe.

### Issue du sous-processus et refus postérieur

`native_build_outcome.execute` est commun au builder et au collecteur. Il ferme
et relit les canaux sans masquer sortie/signal, tente le reçu immuable
`collector-result.json` ou `cargo-result.json` **avant** le contrôle postérieur,
puis écrit un reçu distinct `*-admission-after.json` : issue du processus,
admission et état de la première publication. Le `failure.json` final contient
`native_outcome` structuré et les causes imbriquées, même si la publication d’un
reçu échoue. Un lancement sans issue connue conserve returncode:null ; aucun zéro
n’est inventé. Un échec interdit la qualification ; erreur de canal/fermeture/hash
ne devient pas succès vide.

Recette : `recipes/native-build-outcomes.json`, cinq cas simulés refus/reprise et
deux consommateurs de métadonnées ciblés, sans processus Cargo. Refus originaux et
profils de production antérieurs restent distincts ; une méthode modifiée exige
sa qualification fraîche. La recette unitaire d’admission externe n’exige pas le
registre réel du poste : montage/enregistrement sont isolés. Les vrais builds
lient toujours le registre optionnel et les empreintes des méthodes de contrôle.
