# V4 — Recette et documentation de la livraison

## Résultat

La documentation française et anglaise décrit désormais la version du binaire,
les états dégradés, la fenêtre web, les recettes de build/installation/Docker,
le redémarrage et le cache Go local, avec des liens vers les captures conservées.
La livraison reste `PARTIAL` : ce worker n’a exécuté ni suite Go complète, ni
Chrome, ni Docker, et aucune revue indépendante fraîche du candidat combiné n’est
encore enregistrée.

## Identité et périmètre

- Mission / tâche / tentative / producteur : `w-10b0e692d202f36ee05a1b28` / `plan-10b0e692d2-V4` / `a-b5618497a7a7148a4c1b0b49` / `auto-221a77760cdf88fdc959`
- Rôle : worker documentaire, départ 1/3, révision du travail 93.
- Base : branche `codex/version-history`, HEAD `8d1e223053d7c047bde94276ea3094bd748c82f5`.
- Candidat : diff non commité combiné ; `git diff --stat` observé avant les deux rapports V4 : 22 fichiers suivis, 509 insertions et 77 suppressions, plus les fichiers non suivis listés par `git status --short`.
- Fichiers modifiés par V4 : `README.md`, `README.en.md`, `INSTALL.md`, `docs/en/INSTALL.md`, `GUIDE-UTILISATEUR.md`, `docs/en/USER-GUIDE.md` et les deux rapports V4. Les rapports V1–V3, scripts, code version/web/cache et captures ont été laissés intacts.
- Observation tardive : `tools/verification/version_delivery.py` est apparu non suivi après l’inventaire initial. V4 ne l’a ni créé, ni lu, ni modifié ; le responsable doit l’inclure dans son empreinte finale si ce fichier appartient au candidat.
- État : documentation et contrôles statiques terminés ; ni revue indépendante, ni gate de validation, ni acceptation moteur revendiquée.

## Matrice R1–R5

| Exigence | Observable attendu | Contrôle / environnement | Révision / entrées | Résultat | Preuve / limite |
| --- | --- | --- | --- | --- | --- |
| R1 | Version explicite, commit/date/dirty fiables ; binaire distinct des sources ; aucune fausse release | Lecture ciblée de `version.go`, `runtime_health.go`, `version_history.json` et `web/runtime-health.js`; contrôle documentaire des six guides | HEAD `8d1e223…`, diff sale ; `version.go` `8c5963…`, manifeste vide `18d5ed…` | PARTIAL | Contrat et documentation concordent ; le contrôle runtime frais du candidat combiné reste au responsable. `devel`, `unknown`/`null` et liste vide sont explicités. |
| R2 | `version`, `--version`, JSON sans SQLite ; cohérence build local/install/Docker, sans Git au lancement | `bash -n install.sh`; `sh -n build.sh`; `./install.sh --help`; `make -n build`; lecture ciblée `main.go`, `Makefile`, `install.sh`, `Dockerfile` | `build.sh` `4a8982…`, Makefile `1c1271…`, install `d8dce1…`, Dockerfile `1a6217…` | PARTIAL | Exits 0 et `make -n build` produit `sh ./build.sh "bin/swarm"`. La recette native/Docker antérieure est liée aux mêmes quatre empreintes dans V2, mais Docker et les trois commandes n’ont pas été réexécutés par V4. |
| R3 | Bouton cockpit/préparation ; fenêtre accessible FR/EN, État/sombre, Échap et retour focus | Documentation liée aux captures ; `python3 tools/verification/version_captures.py` | Manifeste `bf2e66…`, archive `a1402d…` ; contrôle moteur `version-web-real` déclaré comme origine | PARTIAL | Exit 0 : dix PNG, SHA-256, dimensions, résultat natif et zéro erreur/échec réseau conservés. Vérification structurée Codex non visuelle ; inspection visuelle des captures déclarée par le responsable, pas par ce worker. Revue indépendante fraîche absente. |
| R4 | Historique embarqué borné et sûr ; vide explicite sans release inventée ; liens GitHub | `python3 -m json.tool version_history.json`; lecture ciblée du parseur et du rendu `textContent`/liens HTTPS | `version_history.json` `18d5ed…`, `version_history.go` `de546a…` | PARTIAL | JSON valide, `releases: []`, documentation FR/EN cohérente. Les tests comportementaux antérieurs sont consignés dans V2/V3 ; aucun test Go frais V4, conformément à la consigne. |
| R5 | Tests CLI/API/web, docs bilingues, preuves du candidat exact, revue indépendante | Assertions `rg` sur les trois commandes dans six documents ; existence des cibles de captures ; `python3 tools/agent-workflows/check.py` | SHA des docs : README FR `8d5a89…`, EN `af46e4…`, INSTALL FR `855fad…`, EN `bd8277…`, guide FR `d75e8e…`, EN `aa4c2b…` | PARTIAL | Tous les contrôles documentaires sortent 0 ; contrôle workflows : `OK: 9 shared Claude/Codex skills…`. Suite Go complète, CLI/API compilés, Chrome/Docker et revue indépendante restent à qualifier sur le candidat final. |

Les préfixes ci-dessus sont des aides de lecture ; les empreintes SHA-256 complètes
figurent dans la section « Commandes et résultats ».

## Documentation livrée

Les six documents conservent leur contenu existant et ajoutent :

- `swarm version`, `swarm --version` et `swarm --json version`, utilisables sans base projet ;
- la signification non trompeuse de `devel`, `unknown` et `null` ;
- la séparation entre identité du binaire, sources locales et état sale ;
- le bouton **Version et nouveautés** / **Version and what's new** dans le cockpit et la préparation ;
- l’historique embarqué vide tant qu’aucune release réelle n’est déclarée, avec lien vers les commits GitHub ;
- les chemins communs `build.sh`, `make build`, installation native et Docker, puis le redémarrage obligatoire du processus ;
- les captures FR/EN, thèmes État/sombre et états chargement/indisponibilité ;
- le cache worker `.swarm/cache/go-build` déjà documenté dans les deux guides d’installation, sa persistance et ses limites face aux refus socket/réseau/Docker.

Captures liées :

- [Cockpit FR, État](../../screenshots/version-history/cockpit-fr-etat.png)
- [Cockpit EN, sombre](../../screenshots/version-history/cockpit-en-sombre.png)
- [Préparation FR, sombre](../../screenshots/version-history/prepare-fr-sombre.png)
- [Préparation EN, État](../../screenshots/version-history/prepare-en-etat.png)
- [Manifeste des dix captures](../../screenshots/version-history/manifest.json)

Ces images proviennent d’un parcours local isolé. Elles ne démontrent ni une
mission réelle, ni l’autonomie d’un fournisseur.

## Commandes et résultats exacts

1. État initial : `pwd`, `git rev-parse --show-toplevel`, `git rev-parse HEAD`, `git status --short` — exit 0 ; racine attendue et HEAD `8d1e223053d7c047bde94276ea3094bd748c82f5`, diff préexistant conservé.
2. Syntaxe et commandes documentées : `bash -n install.sh`, `sh -n build.sh`, `./install.sh --help`, `make -n build` — exit 0. Première ligne d’aide : `Installation de Swarm depuis ce dépôt (Linux).`; sortie make : `sh ./build.sh "bin/swarm"`.
3. Contrat documentaire : recherche des trois commandes dans chacun des six documents, validation JSON du manifeste d’historique et du manifeste de captures, vérification de l’existence non vide des cibles — exit 0, `document_command_triplets=PASS`, `json_and_capture_targets=PASS`.
4. Conservation des captures : `python3 tools/verification/version_captures.py` — exit 0, `PASS: 10 preserved real-journey PNGs, SHA-256, dimensions and native browser result; visual inspection remains supervisor evidence`.
5. Configuration partagée : `python3 tools/agent-workflows/check.py` — exit 0, `OK: 9 shared Claude/Codex skills, preparation commands and contract`.
6. Empreintes de distribution : `build.sh` `4a89827e8fe630364276beb90012af0456b40e042443aa1ce0269c49d6fac5a6`; `Makefile` `1c127104de3c2c5659728950ae69bf0c6e750f60d7c7844ab780032926aff2f6`; `install.sh` `d8dce1a19a5d22c02e8d9d6c55d92a97b081360273ad611379becd6819ce5418`; `Dockerfile` `1a621796a7e57643b413260fdc011157f0bc38da071958c6d33e2d7fbacfa379`.
7. Empreintes documentation : `README.md` `8d5a89c7a2766a9f042d3e0db162d6837d199bea04f306260eb2f8318c00613f`; `README.en.md` `af46e4f97db9c4a42eba862abae226f5eced6ad3bac1c55b67d3896f3e7eee88`; `INSTALL.md` `855fad0047f0a235ed5a0bdd2376f11ad87d416d88e6d4c9a7b04b05d30da762`; `docs/en/INSTALL.md` `bd82770c0019e083ac08726656c7d37700a6fafd98397fcdbb731eebe402a0f3`; `GUIDE-UTILISATEUR.md` `d75e8e4bb340ddba20e763e33dec590325be51a38ef966ef77835b45fc12004a`; `docs/en/USER-GUIDE.md` `aa4c2b17a8f49fb51c70d5c4c136eaef23d3902d56d2e59800a0e3a64c780658`.
8. Empreintes captures : manifeste `bf2e66ec8e3bb0ae6989b96f3b2bdd126c72ede5b293aefcfb1a9bd807a17291`; archive `a1402df71a96af84b515b301a6d0a16b11d3b4cba077398411a9cdb8148557d5`.
9. Contrôle final : `git diff --check`, absence d’espaces finaux dans les huit fichiers V4, assertions de matrice/gates et présence des captures liées — exit 0 ; `final_diff_check=PASS`, `reports_and_gate_matrix=PASS`, `linked_capture_targets=PASS`. Le `git status --short` final signale aussi le fichier externe tardif ci-dessus.

## Gates obligatoires — constat du worker

| Check | Phase | Mandatory | Résultat | Preuve / limite |
| --- | --- | --- | --- | --- |
| `plan-entry` | entry | true | PASS | Racine, rôle, identité, dépendance V3 et périmètre confirmés ; aucune écriture hors périmètre V4. |
| `plan-criterion-1` | validation | true | PARTIAL | Matrice R1–R5, SHA/diff, résultats et limites fournis ; revue indépendante fraîche absente. |
| `plan-criterion-2` | validation | true | PARTIAL | Documentation bilingue et captures liées ; contrôles statiques build/install réussis, Docker non exécuté. |
| `plan-validation` | validation | true | NOT TESTED | Les recettes natives moteur et la revue indépendante doivent porter sur le candidat final après V4. Ce contrôle personnel ne les remplace pas. |
| `plan-delivery` | delivery | true | NOT TESTED | Le relais du rapport est automatique après fin de tentative ; il ne prouve ni lecture, ni acceptation. |

## Revue, limites et qualification restante

- La revue Codex disponible sur les captures est structurée et non visuelle : elle vérifie manifestes, SHA, dimensions, résultat navigateur et diagnostics. Le responsable déclare avoir inspecté visuellement les captures ; ce worker ne transforme pas cette déclaration en revue indépendante fraîche.
- Les preuves V2/V3 documentent une suite Go native, un runtime Docker et le parcours navigateur réel à des étapes antérieures. Elles sont utiles et leurs artefacts restent intacts, mais l’ajout du cache worker et des documents V4 impose des contrôles frais sur le candidat final pour la gate.
- Conformément à la consigne, V4 n’a pas relancé `go test ./...`, Chrome ou Docker dans cette sandbox. Les refus socket/Chrome/Docker historiques n’ont pas été répétés.
- Les trois commandes de version n’ont pas été rejouées sur un binaire fraîchement compilé par V4 ; leur exécution, l’API réelle et les deux pages seront couvertes par les contrôles natifs du responsable.
- L’apparition tardive d’un script non suivi empêche V4 de prétendre figer seule l’intégralité du candidat ; le responsable doit prendre l’empreinte après stabilisation du diff.
- Aucun push, merge, tag, image ou paquet : aucune publication n’a été effectuée.

## RETEX court

La séparation binaire/sources évite qu’un checkout local fasse passer un ancien
serveur pour le candidat courant ; la documentation insiste donc sur le
redémarrage et la relecture de `version`. Un manifeste vide et `devel` sont plus
fiables qu’une release synthétique. Enfin, conserver les captures avec SHA et
résultat navigateur améliore la traçabilité, mais ne remplace ni inspection
visuelle, ni test du candidat final, ni revue indépendante.

## APEX / PDCA et prochaine action

- Analyse / PLAN : inventorier les six documents, les recettes communes, le contrat CLI/web, les captures et les limites de qualification.
- Exécution / DO : mettre à jour uniquement la documentation et ce handoff ; préserver code, scripts et rapports V1–V3.
- Vérification / CHECK : syntaxe shell, dry-run Make, aide installateur, assertions documentaires, JSON, artefacts de captures et configuration partagée, tous exit 0.
- Ajustement / ACT : laisser R1–R5 `PARTIAL` et validation/delivery `NOT TESTED` jusqu’aux preuves fraîches et à la revue distincte.
- OODA : observation — tests natifs historiques disponibles mais candidat enrichi ; orientation — ne pas réutiliser un vert ancien comme gate fraîche ; décision — contrôler statiquement V4 et transmettre les recettes exactes ; résultat — documentation prête, qualification finale attribuée au responsable.

Prochaine action du responsable : sur exactement ce HEAD et ce diff sale, exécuter
les contrôles moteur CLI/API/web, suite Go et Docker prévus, constater le
redémarrage et le cache avec un worker réel, puis demander une revue indépendante
du même candidat. La remise par le moteur et l’acceptation restent distinctes.


## Qualification native du responsable — postérieure au rapport worker

Les limites PARTIAL/NOT TESTED ci-dessus décrivent exclusivement la tentative documentaire du worker. Le responsable a qualifié le candidat combiné après intégration du correctif cache et du web, sans modifier le rapport canonique docs/plan-10b0e692d2-V4.md. La décision humaine et la revue restent distinctes.

| Exigence | Qualification native réellement exécutée | Résultat technique |
|---|---|---|
| R1 | Alias version/--version et JSON du binaire compilé ; devel, commit 8d1e223053d7c047bde94276ea3094bd748c82f5, modified=true, date et provenance injected. La date du binaire réellement lancé est 2026-10-02T19:07:06Z. | PASS technique |
| R2 | make build et install.sh --mode native dans répertoires temporaires, métadonnées identiques grâce aux paramètres injectés ; version --root sur dossier inexistant ne crée aucun stockage. Construction Docker du candidat combiné terminée exit 0 et exécution --network none, aucune base montée. | PASS technique |
| R3 | Contrôle moteur version-web-real courant : serveur réel isolé, 10 parcours FR/EN/État/sombre, version/commit, historique vide, liens sûrs, contraste, clavier/Échap/focus, chargement et indisponibilité ; tous réussis sans erreurs. Serveur de la vraie mission redémarré et bouton/fenêtre ouverts dans le navigateur utilisateur. Capture live-cockpit-fr-sombre.png conservée ; aucun agent interrompu par ce redémarrage. | PASS technique |
| R4 | Suite complète du candidat combiné : 926 tests/exemples/targets fuzz découverts et répartis dans 4 groupes disjoints, chacun exit 0. Les contrôles version/historique et entrées hostiles en font partie. | PASS technique |
| R5 | Suite complète, cache avec race detector, npm test (y compris i18n), go vet : tous exit 0. Six guides FR/EN vérifiés avec version/--version/--json version et bouton réel ; archive des dix captures conservée. Revue indépendante finale à obtenir avant l’acceptation moteur. | PASS technique ; décision encore attendue |

Preuves natives : dossier `docs/plans/version-history/combined-qualification/`, reçu receipt.json, full-go-suite.log, worker-cache-race.log, frontend.log, vet.log, docker-build.log, docker-version.json, delivery.log. Le reçu inclut les empreintes des sources Go/web/distribution et des journaux ; le snapshot des sources a été capturé après exécution réussie, sans modification Go/web durant le contrôle. Le vérificateur version_delivery.py refuse toute dérive depuis ce snapshot. Il vérifie les preuves de suite complète préservées, mais ne prétend pas réexécuter cette suite ; il exécute réellement la compilation Make, l’installation native et le runtime Docker sur le candidat courant.

Image Docker locale de qualification : `sha256:7f40f2b3c5b788473e5f02da92461c86afad7cef991350f0c0c3daf1c16bd178`. Aucune image, release ou commit n’a été publié. La vérification Codex reste structurée non visuelle ; le responsable a inspecté visuellement les vraies pages et captures. La recette qualifie cette fonction et le cache worker, pas l’autonomie universelle des fournisseurs ni leurs accès socket/Docker.

RETEX complémentaire : la première recherche web trop large a inclus des bundles et produit un événement supérieur à 1 Mio. Un silence fournisseur a ensuite interrompu la tentative, sans preuve de causalité entre les deux. La reprise ciblée a abouti. Le cache Go partagé hors workspace était incompatible avec certaines sandboxes ; un cache local réutilisable est désormais préparé avant le lancement et transmis explicitement à Codex. Les refus de socket et de Docker nécessitent une qualification native distincte. Enfin, déclarer les contrôles et lier les captures avant la revue évite les avis répétés faute de preuve, sans modifier ni effacer les rapports initiaux.
