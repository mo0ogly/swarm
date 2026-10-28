# PAEA — incrément 16 : résolution de l’objet de dette

## Reprise et décision

Le 26 octobre, `next` reprend PA-02/03 après la livraison dev80, commit
`462abbeb9bcf58e828c2fc9db5ef69b43c020bbc`. Instance observée : PID 2612428,
SHA256 `f55654100c7b14684e5de2a855e9ed660d82cdb881ae35384789beba6cf95299`.
192 entrées suivies étrangères sont conservées (106 suppressions et deux ajouts
indexés, 84 modifications). Codex racine reste seul propriétaire de cet incrément.

PAEA-GUIDE-03 a déjà un contre-exemple retenu : deux aliases bruts créent deux
effets dans le laboratoire. L’enrôlement signé existant fournit des données et
exige lui-même un contrôle courant protégé ; Trust utilise une ressource libre.
Aucun producteur de registre de dettes protégé n’est présent dans ces interfaces.

Deux options : modifier l’intention v1 en ajoutant fournisseur/référence (migration
et collisions d’aliases encore possibles), ou conserver v1 et résoudre d’abord
un objet stable depuis un registre explicite. Choix : deuxième option, bibliothèque
pure réutilisable et CLI hors ligne, puis observation d’un seul effet de fixture.
Ni cette CLI ni un fichier fourni ne deviennent une source d’autorité.

## Contrat et critères

1. Registre fermé/versionné : projet, révision et dettes ; identifiant de dette
   unique, aliases uniques dans le couple fournisseur/référence. Pas de fusion
   automatique, normalisation implicite ou correspondance approximative.
2. Demande fermée/versionnée : projet, fournisseur, référence, révision attendue,
   version de facture attendue et identifiant de transport. Le registre fournit
   montant entier exact, devise et compte ; `invoice_id` v1 reçoit `debt_id`.
3. Mauvais projet, révision/version périmée, référence absente, ambiguïté,
   doublons JSON ou valeurs invalides : refus sans proposition ni effet.
4. Deux aliases d’une dette produisent la même intention ; même référence chez
   deux fournisseurs peut désigner deux dettes. Correction de version/montant/
   compte conserve l’intention et invalide le contenu antérieur.
5. Rapports liés aux octets bruts et explicitement non autorisants. Erreurs de
   lecture, écriture et flush propagées ; aucune ouverture de Root ou mutation.
6. Tests nommés, recette permanente fraîche et intégrité ; profil normal,
   construction/installation locale et parcours public réels avant annonce.
   Revue politiques/fonctions/erreurs liée aux octets finaux.

## Limites et reprise

Le producteur protégé devra authentifier le registre, assurer fraîcheur et
continuité des identifiants entre révisions, lier son observation au contexte
avant accord, puis revalider sous l’ordre d’engagement. Des fichiers isolés ne
détectent pas deux dettes sémantiquement identiques avec références disjointes,
ni une réattribution malveillante entre snapshots. Responsables métier/risque,
approbateur, comptes séparés, IA3/AC-02 et évaluateur restent ouverts ; aucun
paiement réel, activation protégée, E2/E3 ou invariant de bout en bout acquis.

État initial : décidé ; implémentation et vérification à produire ci-dessous.

## Vérifications intermédiaires et incidents de méthode

La recette `paea-payment-registry` réussit : 3 tests de schéma, 6 du résolveur,
3 CLI, 17 de compatibilité v1 et 8 CLI existants, soit **37 tests nommés**.
Intégrité retenue VERIFIED. Clippy `swarm-protection-contract --lib -D warnings`
et Rustdoc sans dépendances réussissent. Pas de revendication Clippy globale :
la compilation CLI conserve des avertissements du travail Rust étranger
(`managed_repository`/`managed_review_reply`), non modifié ici. Propriétaire :
migration Rust ; reprise : vérifier usage/retraits avec ses tests d’origine.

Le premier contrôle a sélectionné `bin/swarm-local` depuis un résumé périmé,
alors que `/proc/2612428/exe` désigne `bin/swarm`. L’ancien exécutable a refusé
la version de stockage 37. Aucun succès métier ni absence d’effet déduits de ce
refus. Le lanceur de preuve affirmait avant d’écrire les flux : original non
retenu, défaut de méthode déclaré dans `method-incident.json`. Correction :
identité observée du processus, hash vérifié, cwd isolé et flux conservés AVANT
assertions. Témoin dev80 valide puis commande manquante reproduite (sorties 0/2).
La recette de livraison permanente compare désormais le contenu des évolutions
au catalogue courant, au lieu du nombre fixe de notes du script ponctuel 15.
Ces apprentissages sont transmis via recette/source maintenues et leur README.

## Livraison vérifiée, preuves et RETEX de l’incrément

**Installé localement : v0.1.1-dev.82**, commit de base
`462abbeb9bcf58e828c2fc9db5ef69b43c020bbc`, modifications non commitées affichées.
SHA256 `2a328c56775fad8c9715d1c0bc2ca22b9d60fdbf258a25484f1b3f0cf2154e0d`.
Le PID 3868897 a été remplacé par 3871902 pendant la première vérification UI ;
l’initiateur n’est pas attribué. Même binaire, mais chargement de scripts refusé
pendant l’interruption : preuve UI 01 incomplète conservée, jamais PASS.
La première hypothèse de sélecteur périmé est réfutée : `#version-dialog` est
correct, ses scripts n’avaient pas chargé. Le test est désormais interrompu par
un échec réseau/console/page, avec cause conservée, sans watchdog ni retry.
Une tentative de thème sombre a ensuite cliqué un bouton dans un volet fermé
(UI 03 FAIL conservé) ; correction : ouvrir le volet et attendre ses animations
finies avant interaction. Aucune correction produit nécessaire pour ces défauts
de méthode. Les sources des recettes et ce dossier transmettent la reprise.

| Contrat | Résultat réel | Preuve locale sous `evidence/20261026-debt16-_la9qpcs/` |
|---|---|---|
| Schéma/résolveur/CLI/compatibilité v1 | 37 tests PASS, intégrité VERIFIED | `payment-contract-01` |
| Profil normal livré | VERIFIED_DEFAULT_NATIVE_PROFILE ; features core vides, test=false, aucune feature de fixture | `production-profile/profile/result.json` et archive effective |
| CLI du binaire final | 2 tests, 33 appels PASS, hash stable ; exemple documentaire exécuté | `public-recipe-02`, `payment-public-observations-02`, `documented-example.*` |
| Effets fictifs réels | 8 tests PASS ; 151 processus, 28 racines ; aliases résolus : un effet exact, aliases bruts : contre-exemple deux effets conservé | `pilot-final-recipe-01`, `pilot-observations-02` |
| Instance identifiée et Version | 26 assertions PASS, FR clair/EN sombre, 9 assets uniques, notes exactes, lien Git, modifications affichées, focus, zéro erreur inattendue | `live-recipe-04`, `live-browser-04` |
| Garde de vérification | Témoin valide puis script volontairement refusé : sortie 1 attendue, attente interrompue et cause retenue, sans toucher au service | étape 2 de `live-recipe-04`, `live-browser-04-fault-script` |
| Politiques/fonctions/erreurs et diff | Revue sémantique de notre delta, liée aux hashes finaux ; aucun verdict indépendant/global | `source-review.json`, `scoped-diff-check.*` |

Le candidat dev81 et dev82 ont 446 entrées effectives de livraison comparées ;
seul `tools/rust-migration/build_native.py` change (conservation de diagnostic du
collecteur par une autre session). Les sources runtime restent identiques entre
ces candidats ; les tests publics/laboratoire sont néanmoins joués sur dev82.
Le résultat de profil marque `installed:false` car le collecteur précède
l’installation ; les observations PID/binaire/HTTP/Version établissent séparément
l’activation. Aucun commit/push nouveau de cet incrément ; l’index étranger est
préservé. La livraison locale reste distincte de la publication précédente.

Exemple exécuté, sans effet ni droit :

```sh
./bin/swarm protection resolve-payment tests/fixtures/paea/payment-registry-v1.json tests/fixtures/paea/payment-request-v1.json --json --lang fr
```

Le menu existant **Vérifications PAEA** reste le contrôle action/reçu ; cette
nouvelle résolution s’utilise par CLI et bibliothèque, sans nouvel écran.
PAEA-GUIDE-03 est traité pour les mappings explicites coopérants, pas clos pour
un registre protégé. **0/17 invariants de bout en bout** ; PA-02/03, IA3/AC-02,
accords humains, campagnes UX, E2/E3 et RETEX final du programme restent ouverts.
Reprise : producteur protégé lié au projet ouvert et identité de dette persistée,
continuité/fraîcheur sous ordre d’engagement, puis qualification avec rôles
réels. La désignation responsable métier/risque et approbateur est demandée ;
aucun nom n’est inventé et aucun rôle n’est attribué par ce dossier.

Amélioration à transmettre au propriétaire Vérification : réexaminer la même
attente après erreur dans les autres recettes UI, avec témoin puis script refusé.
Le script ponctuel 15 est historique ; son successeur maintenu est corrigé et
vérifié ici. La recette native 15 utilise un serveur de fixture détenu, donc
son contexte de concurrence diffère ; son scénario de perte de script reste à
qualifier avant de reprendre cette garantie. Pas de revendication étendue.

Le `git diff --check` global retrouve les deux blancs de fin de fichier déjà
ouverts à l’incrément 15 : `docs/plans/migration-rust/DELIVERY-PREPARATIONS-20261025.md:85`
et `rust/crates/swarm-engine/src/root_store/task_validation/managed_receipt.rs:293`.
Le contrôle de nos 31 chemins passe. Propriétaire de ces deltas étrangers :
migration Rust ; reprise avec son lot. Aucun de ces fichiers n’est corrigé ni
inclus dans notre delta ; aucune conformité globale annoncée.
