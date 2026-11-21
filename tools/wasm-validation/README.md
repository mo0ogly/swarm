# Contrôle de validation Wasmtime / Wasmtime validation control

## Français

Cet exécutable autonome se raccorde à un **contrôle de validation existant**
dans Swarm. L'opérateur attribue la commande et les entrées à la tâche avec
la politique de validation. Après qualification du binaire, autoriser son nom
dans les réglages de validation et le rendre résolvable par le `PATH` du
processus Swarm. Placer le module dans le candidat géré, puis enregistrer
`["swarm-wasm-validation", "rule.wasm", "7", "100"]`.
Inscrire les intrants concernés dans le contrôle. Les nombres de l'exemple
appartiennent à sa règle et à son budget de calcul.

Le module Wasm core exporte la seule fonction `check(i32) -> i32`. Le module ne
peut déclarer ni import, ni mémoire ou table, ni fonction de démarrage. Le fichier
est limité à 65 536 octets par le protocole de ce contrôle. Le troisième argument
fixe le fuel, en unités Wasmtime, pour cette invocation et doit être positif.
La limite de temps du contrôle Swarm continue de régir le processus complet,
notamment le chargement et la compilation.

La valeur 1 rend le contrôle positif. La valeur 0 rend un refus métier. Les
autres valeurs et les erreurs de chargement, de profil ou d'exécution rendent
un échec. La décision d'adoption reste dans le parcours de validation Swarm.

## English

This standalone executable plugs into an **existing Swarm validation control**.
An operator assigns the command and its inputs through the task's validation
policy. After qualifying the executable, authorize its basename in the
validation settings and make it resolvable through the Swarm process `PATH`.
Store the module in the managed candidate and record an argument array such as
`["swarm-wasm-validation", "rule.wasm", "7", "100"]`. Declare the relevant
inputs in the control. The example numbers belong to its rule and fuel budget.

A core Wasm module exports one function, `check(i32) -> i32`. The module can
declare zero imports, memories, tables, or start functions. This control protocol
caps the file at 65,536 bytes. The third argument sets positive Wasmtime fuel
units for this invocation. The Swarm control deadline still applies to the
whole process, including loading and compilation.

A return value of 1 passes the control and 0 gives a policy refusal. Other
values and load, profile, or runtime errors fail. Swarm's validation workflow
retains the adoption decision.

## Build and verification / Construction et vérification

Build with `cargo build --locked --manifest-path tools/wasm-validation/Cargo.toml`.
The public journey is `tests/rust_wasm_validation_control.py` with an identified
Swarm binary (`--binary`), its native profile receipt (`--profile`), the
compiled control (`--control`), and a fresh evidence directory (`--output`).
The isolated Python fixture dispatches two case inputs to the same executable
based on the candidate marker. It records the real receipts, checks for
preserved attempts and leaves the independent review pending.
