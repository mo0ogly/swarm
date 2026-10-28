# Carte du dépôt Swarm

Swarm utilise un moteur, une CLI et un serveur HTTP Rust avec une interface locale HTML/CSS/JavaScript. Le portage fonctionnel reste en cours ; voir [le plan actif](plans/migration-rust/PLAN.md).

| Chemin | Rôle |
| --- | --- |
| `rust/crates/swarm-cli/` | Entrée `swarm-rust` et adaptateurs CLI |
| `rust/crates/swarm-engine/` | RootStore, WorkEngine, état et politiques persistés |
| `rust/crates/swarm-web/` | Routes HTTP protégées et inventaire des ressources embarquées |
| `rust/crates/swarm-*/` | Contrats d’identité, enrôlement et JSON |
| `rust/Cargo.toml`, `rust/Cargo.lock`, `.cargo/config.toml` | Workspace, dépendances verrouillées, capacité SQLite requise |
| `web/`, `frontend/`, `locales/` | Sources canoniques de l’interface et traductions |
| `contracts/`, `tests/` | Schémas, fixtures et recettes CLI/API/navigateur |
| `tools/verification/`, `tools/rust-migration/` | Recettes pérennes et builds Rust qualifiés |
| `docs/`, `docs/en/` | Documentation FR/EN ; plans et preuves datés conservés |
| `.claude/skills/` | Méthodes canoniques ; `.agents/skills/` les référence |
| `deploy/`, `Dockerfile`, `compose.yaml` | Livraison locale conteneurisée |
| `swarm`, `swarm.sh`, `build.sh`, `install.sh` | Relais CLI installé, cycle serveur, build Rust, installateur |

Les `include_bytes!`/`include_str!` Rust embarquent les seules ressources déclarées. Le JavaScript est réutilisé pendant le portage ; sa présence ne prouve pas qu’une route Rust existe. Les sources Go ne sont plus suivies ; les copies locales ignorées et preuves datées sont des références, jamais un repli d’exécution.

Utiliser `./swarm.sh build` pour un candidat local identifié, puis le redémarrage pris en charge. Un build de développement simple est `cargo build --manifest-path rust/Cargo.toml --locked -p swarm-cli`. Aucun des deux ne migre automatiquement les anciennes missions ni ne valide toutes les fonctions.

`.swarm`, secrets, bases, configurations locales de livraison et caches sont privés. Préserver les missions réelles ; utiliser des racines isolées pour les tests. Compiler ne remplace pas un processus déjà lancé. Voir [l’installation](../INSTALL.md) et [la contribution](../CONTRIBUTING.md).
