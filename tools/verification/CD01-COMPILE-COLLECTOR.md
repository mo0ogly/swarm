# Collecteur C-D01 instrumenté — collecte non qualifiée

`cd01_compile_collector.py` est l’unique copie maintenue du collecteur historique SHA256 `65a6f5806d7e5aca02950e184a4aac2d24d78af55116652c46795b78019c1030`, complétée pour observer des intentions rustc et pré-lier des intrants. L’original reste une preuve immuable. Cet outil ne qualifie ni binaire, ni profil, ni graphe complet, n’exécute aucun corps de test et ne crée aucun enregistrement d’admission C-D01.

## Entrées et autorité

Huit arguments explicites sont nécessaires : `--root`, `--output`, `--target`, `--inventory`, `--rustc-intent-producer`, `--rustc-intent-producer-sha256`, `--root-guard`, `--root-guard-sha256`. Le root est l’export privé sans `.git`/`.swarm`, avec inventaire original vérifié ; ce n’est pas le checkout partagé. Output est une nouvelle feuille de preuves. Target est une cible temporaire vide, déjà admise par la configuration de stockage ; l’outil ne l’enregistre pas et ne nettoie aucune cible.

Le chemin de montage vient de `.cargo/external-targets.json` via le lecteur strict du helper épinglé : inscription publique → validation de propriété/schéma → sélection de la cible → observation du vrai montage → FD/admission. L’identité SWARM/ext4 autorisée et la réserve minimale de 21 474 836 480 octets restent exigées sur hôte et sorties, sans repli sous montage absent. L’instance Swarm, son dépôt, ses données et son binaire installé ne sont pas déplacés.

Le garde doit être exactement `ROOT/tools/verification/cargo_space_guard.py`, car sa politique est relative à son emplacement. Producteur et garde sont contrôlés par empreintes explicites ; le producteur est copié exécutable, relu et archivé. Les shebangs et outils PATH sont des observations, pas une qualification complète des interpréteurs, proxies ou bibliothèques. Les arguments conservés exigent une revue/autorisation de l’appelant : ils peuvent contenir des données privées. Aucun dump d’environnement descendant ni garantie générique de confidentialité.

## Parcours

La commande Cargo reste `cargo test --manifest-path ROOT/rust/Cargo.toml -p swarm-engine --lib --no-run --locked --offline --message-format=json`, sans activation de features de fixture. L’outil refuse les overrides ambiants contrôlés avant de construire son propre `RUSTC_WRAPPER` et les deux variables `CD01_CAPTURE_DIR`/`CD01_SPACEGUARD_PATH`. Cette instrumentation remplace explicitement la valeur wrapper de configuration ; elle ne prouve pas des arguments rustc ou un codegen identiques. Aucun shadow PATH, CC ou CFLAGS ajouté.

Les candidats workspace sont archivés avant Cargo. `Cargo.lock` est hashé et analysé depuis le même tampon. Les paquets verrouillés sont rapprochés d’un seul namespace src/cache complet sous le HOME explicite : absence, ambiguïté, source incompatible, symlink ou checksum d’archive incorrect refuse. Les fichiers décompactés et archives sont épinglés séparément ; leur équivalence n’est pas affirmée. Les deux noms de configuration Cargo sont observés dans cwd/ancêtres/home, sans inventer de priorité. Seul FileNotFoundError établit l’absence ; les autres erreurs refusent. L’observateur de configuration n’ouvre pas de fichier credentials ; les lectures des descendants restent inconnues.

Après Cargo, les contrôles pré/post, événements/features et image gelée restent distincts des intentions. Les reçus v1 réels sont validés, y compris arguments vides/non-UTF8, lien partiel/final et délégation cohérente ; les sondes `--print`/version ne sont pas des compilations. Un nom de crate et `--test` donnent seulement un candidat d’intention correspondant au nom de l’artefact sélectionné : liaison source/sortie et issue individuelle restent UNKNOWN. Le statut final est toujours `CD01_COMPILE_ONLY_UNQUALIFIED`, `full_effective_inputs_unknown=true`, tests0. Même un exit0 ne permet ni les quinze corps C-D01 ni une installation.

## Erreurs et preuves

Le helper natif conservé retient les issues primaires avant publication ; Cargo0 ne signifie pas test PASS. Une erreur de postcontrôle peut suivre une compilation terminée. Les valeurs en cache, causes typées, états partiels et erreurs de mesure sont conservés. Si le dernier diagnostic échoue, le groupe d’erreurs porte `collector_failure` ; le consommateur terminal `_error_tree` expose ces faits sans fabriquer un frame `native_outcome`. Cela reste un secours en mémoire, jamais une garantie de livraison durable si tous les canaux échouent. Les archives d’instruments candidates restent conservées sur échec et succès. Aucune publication/écriture échouée ne signifie rollback.

La réserve, les FDs et les intrants exécutés doivent rester stables. Les hashes pré/post sous bail ne constituent ni snapshot atomique ni fenêtre de lecture démontrée. Restent notamment inconnus : environnement descendant, graphes natifs/headers/linker/procmacro, intrants générés/supprimés et graphe complet des outils. L’outil observe des chemins PATH et leurs symlinks finaux ; il ne prouve pas tous les composants parents ou le toolchain effectif.

## Vérification actuelle

Quinze gardes distincts sont maintenus, tous sans compilateur/provider/service : raccord d’arguments, vrais reçus, noms/configs et refus d’erreur, cible vide, archives, namespace registre, PATH relatif, tampon lock, causes/valeurs connues et portée des intentions. Ils ont été exécutés séparément sur les contrats modifiés ; les PASS inchangés sont réutilisés. La recette `recipes/cd01-collector-integration-guards.json` n’a pas été exécutée globalement : schéma et intrants seulement. Les véritables contre-exemples et corrections sont dans `root00-cd01-collector14-waja855_` sur SWARM ; leur index garde les empreintes exactes et les limites.

Aucun Cargo ni corps C-D01 exécuté dans cet incrément ; revue indépendante, allocation exacte et préconditions fraîches restent requises avant collecte réelle. Les tests de fixtures ne valent pas acceptation du plan global. Pas de changement de runtime produit : aucune installation/version_history justifiée par ces seuls outils de vérification.
