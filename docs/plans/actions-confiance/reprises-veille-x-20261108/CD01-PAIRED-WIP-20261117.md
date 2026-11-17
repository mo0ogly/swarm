# C-D01 — sauvegarde WIP du harnais apparié, 17 novembre 2026

Base source : `6a4785c76f1ce72eaa03452b2d9b638cea8c0285`. Ce lot sauvegarde le harnais Rust apparié/adaptateur legacy, son raccordement `cfg(test)`, le contrôleur Python, les gardes inertes, recettes et contrats FR/EN précédemment relus dans le paquet privé. Les postimages déjà identiques au parent ne sont pas recopiées. Les plans partagés étrangers ne sont pas remplacés par leurs anciennes copies.

Les deux probes nécessitent une fixture explicitement admise : `#[ignore]` protège la sélection ordinaire et le contrôleur utilise le nom exact avec `--ignored`. Le garde du nouvel argv a d’abord échoué puis passé (un seul healthy inerte, aucun vrai enfant) ; les cinq anciennes campagnes de faute ne sont pas rejouées. L’image initiale listait deux probes, zéro dans `--list --ignored` : constat de métadonnées, aucun panic/corps observé. La nouvelle sélection doit être contrôlée sur l’image recompilée avant clôture de ce critère.

## Preuves et portée

Sources et preuves : `/run/media/fpizzi/SWARM/swarm-evidence/root00-cd01-compile-collector-vuh6r78q` ; composition initiale dans `root00-cd01-build-admission-i9e_veh_`. Le snapshot compilateur reste local sous `.swarm-fixtures/cd01-paired-source-dk710gp7`, sans `.git` ni `.swarm`. Le dépôt et le service ne sont pas déplacés. SWARM ne porte que preuves, fixtures/cache temporaires et images de test non installées. Les cibles sont enregistrées par le parcours pris en charge ; garde20Gio conservée, FD volume retenu pendant enfants et publication.

Première compilation : un seul `cargo test -p swarm-engine --lib --no-run --locked --offline --message-format=json`, retour0,949candidats préliés/archivés relus,117artefacts, `swarm_engine` libtest unique `profile.test=true`, features vides, aucune feature fixture observée. Image `1df8482ba285a5583ef5e3c02191020a6fdef68008a892fea93d817f0536ef0b`, reçue indépendamment09 ; zéro corps de test. Une seconde compilation est justifiée par le contrat Rust de sélection modifié, pas par un rejeu historique. Son résultat est conservé séparément dans `compile-only-r2`.

## Revue et erreurs

Source paire initialement revue04 ; correction de sélection revue09 (`APPROVE_SOURCE_ONLY`, tour `01a1e816-4072-7ee3-852c-19751ac45473`). Le collecteur de compilation GLM était défectueux : suppléance root bornée, callbacks de cache avant publication, causes imbriquées, holds, garde avant écritures et erreurs de nettoyage conservées avec contre-exemples inertes. Originaux, refus de revue et défauts des validateurs v1/v2 sont préservés ; ne pas transformer ces preuves en tests Rust.

Trois axes : noms/profil/schéma et sélection manuelle sont contrats de fixture/protocole, pas politiques d’exécution produit ; docstrings/Rustdoc et FR/EN décrivent les préconditions et limites ; le contrôleur refuse zéro test, sortie non nulle, canaux/record/observations incohérents, sans réussite vide. Aucune borne temporelle, budget, droit ou mécanisme d’acceptation n’est changé.

## Reprise

WIP seulement : ni merge main, installation, publication produit ni acceptation globale. Pas de `version_history.json` modifié : aucun produit n’est livré. La compilation et l’intégrité workspace ne qualifient pas toute la fermeture compiler/registry/générés/toolchain. Propriétaire root00 : compléter cette admission et le record exact avant d’allouer les douze probes appariés/trois globaux ; les gardes R1 et campagnes historiques ne sont pas à rejouer par défaut. C-D01/BF-L0 restent ouverts. Les questions de quota managed-review et de Docker PR40 restent séparées, sans autorité déduite.

[Version anglaise](../../../en/plans/CD01-PAIRED-WIP-20261117.md).


### Réception R2 : compilation et métadonnées vérifiées

Image `91ac8c1913e38b38aba0c287c58344ec7c37dc3bb0ec535f6d6d8ef9ca3626b1`, Cargo0,949candidats archivés relus et117artefacts. Les candidats diffèrent de R1 seulement par paired_harness.rs et l’enregistrement de la nouvelle cible. Le listing sain contient les deux probes ; le listing `--ignored` les contient désormais toutes deux. Aucun corps exécuté. Code/source du contrôleur et garde modifiés : un healthy inerte ciblé1PASS ; pas de rejeu des cinq fautes historiques. Ce critère de sélection est vérifié ; les quinze probes comportementales, la fermeture effective complète et l’admission du record restent ouverts. Les résultats originaux et toutes les erreurs sont conservés.

Apprentissage : une compilation de libtest ne prouve pas l’isolation d’une probe manuelle. Rapprocher les attributs de sélection de l’image et les options effectives du consommateur exact avant d’allouer les corps ; un zéro-test ne devient jamais PASS. Le contrôleur exige toujours nom/compteur/rapport réels et refuse l’ancienne image sous `--ignored`. Cette consigne documentée n’est pas une garantie sur un passage futur.
