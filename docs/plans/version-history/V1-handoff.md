# Task result — plan-10b0e692d2-V1

## Outcome in two sentences

Le contrat proposé sépare strictement l’identité du binaire de l’état éventuel des sources locales, réserve les versions de release à des tags SemVer réellement constatés et retient un historique embarqué plutôt qu’un Git interrogé à l’exécution. Le cadrage est documenté et contrôlable, mais reste `PARTIAL` tant que l’URL GitHub canonique n’est pas fournie et qu’une revue indépendante n’a pas été enregistrée; aucune modification applicative n’a été faite.

## Identity and scope

- Work / task / attempt / producer: `w-10b0e692d202f36ee05a1b28` / `plan-10b0e692d2-V1` / `a-b179f710addb1eea9f15f625` (départ 1/3, révision du travail 2) / `auto-4825107401c936ce8bc7`
- Role and assigned scope: worker; lecture ciblée de `runtime_health.go`, `main.go`, `Makefile`, `install.sh`, `Dockerfile`, `.github`; comparaison de deux solutions; seul ce rapport est écrit
- Base / candidate revision: `8d1e223053d7c047bde94276ea3094bd748c82f5` sur `codex/version-history`
- État initial: `?? docs/plans/version-history/` préexistait et a été préservé; le présent rapport était absent
- State: cadrage et vérifications statiques du livrable réalisés; revue indépendante encore à consigner; ni acceptation ni modification de gate par ce worker

## État observé

1. `git tag --list` ne retourne aucun tag (exit 0) et `git describe --tags --exact-match HEAD` échoue avec « No names found » (exit 128). Il n’existe donc aucune release que ce périmètre permette d’affirmer. Le SHA de départ est bien `8d1e223053d7c047bde94276ea3094bd748c82f5`, daté par Git du `2026-10-02T19:22:42+02:00`; cette date de commit n’est pas une date de build ni une date de release.
2. `runtime_health.go` expose déjà `ServerVersion{Revision, Modified, Available, Source, Compared, Current}`. L’identité du binaire vient du marquage VCS automatique de Go (`debug.ReadBuildInfo`), tandis que `Source` appelle `gitState(s.root)`. Il manque la version de release, la date de build et une provenance explicite; la comparaison source est couplée à Git à l’exécution.
3. `main.go` ne déclare ni `swarm version` ni `--version`. Le parsing des options précède la résolution du root, mais toute commande non traitée tôt poursuit vers l’ouverture du store: la future commande doit donc retourner avant `filepath.Abs`, `filepath.EvalSymlinks` et `openStoreWithMigration` pour garantir l’absence d’initialisation SQLite et de dépendance au projet local.
4. Les quatre chemins de compilation observés utilisent tous `CGO_ENABLED=0 go build -trimpath` sans `-ldflags`: `make build`, installation native, build Docker et le `make smoke` de CI. Docker compile avec Go 1.26; CI configure Go 1.26.x; le texte d’installation annonce encore Go 1.24+. Aucun workflow `.github` de release ou de publication n’existe: seul `ci.yml` est présent avec les gabarits issue/PR.
5. Le runtime Docker installe `git`, mais cela ne garantit pas que le build ait accès aux métadonnées VCS ni qu’un binaire natif distribué s’exécute dans un checkout. Le remote `origin` de cette copie est un dépôt géré local, pas une URL GitHub publique; l’URL canonique de l’historique complet ne peut donc pas être figée honnêtement dans ce périmètre.

## Décision — deux solutions comparées

| Solution | Exactitude / récupération | Docker et binaire distribué | Sécurité / coût | Décision |
| --- | --- | --- | --- | --- |
| A. Git à la volée (`describe`, `log`, remote) | Reflète le checkout présent, mais confond facilement binaire installé et sources; devient inconnu sans `.git`, commande ou remote | Dépend de Git et du checkout au lancement; résultat variable entre native, Docker et archive | Processus externes et données non bornées; erreurs/latence à traiter | Rejetée: contraire au fonctionnement hors ligne et à l’absence de dépendance Git au lancement |
| B. Métadonnées injectées au build + manifeste d’historique embarqué, avec repli sur Go BuildInfo | Identité immuable du binaire; manifeste relu et validé en revue; champs inconnus explicites si injection absente | Identique en native et Docker si tous les chemins appellent la même recette; aucune commande Git ni requête réseau à l’exécution | Données bornées, liens validés, rendu échappé; exige de synchroniser la recette de build et le manifeste | Retenue |

Le repli `debug.ReadBuildInfo` conserve le commit et l’état modifié pour un `go build` direct quand Go sait les produire. Il ne doit jamais fabriquer une version ou une date de build, et ne remplace pas l’injection commune aux builds pris en charge.

## Contrat de version proposé

### Convention et provenance

- Une release existe uniquement si le commit compilé correspond exactement à un tag Git validé de forme `vMAJOR.MINOR.PATCH` avec suffixes SemVer 2 éventuels. Tant qu’aucun tel tag n’existe, `version` vaut littéralement `devel`; ne jamais utiliser `v0.0.0`, une date ou le nombre de commits comme fausse release.
- `commit` est le SHA Git complet de 40 caractères en minuscules, ou `null` s’il est indisponible. L’interface peut l’abréger visuellement, mais conserve le SHA complet dans l’API et le lien.
- `modified` vaut `true`, `false` ou `null` (inconnu). Un booléen `false` ne doit pas représenter l’absence de preuve.
- `build_date` est un instant UTC RFC 3339 injecté lors de la compilation, ou `null`. Il est distinct de `vcs.time` (date du commit) et de la date d’une release. Pour un build reproductible, la recette convertit `SOURCE_DATE_EPOCH` quand il est fourni; sinon elle utilise l’instant UTC du build pris en charge.
- `provenance` vaut `injected`, `go_build_info` ou `unknown`. Les champs injectés priment; le repli BuildInfo ne complète que `commit` et `modified`.
- L’identité du binaire ne lance jamais Git, ne lit pas SQLite et ne requiert pas de réseau. La comparaison facultative avec les sources est une projection séparée, non bloquante et tri-état; elle ne modifie jamais `runtime_health.state` ou `launch_allowed`.

### CLI texte et JSON figés

`swarm --version` et `swarm version` sont des alias exacts, acceptent les options globales dans l’ordre déjà pris en charge et sortent avec le code 0 sans résoudre `--root` ni ouvrir le store. En texte, une seule ligne stable:

```text
Swarm devel (commit unknown; modified=unknown; built=unknown; provenance=unknown)
```

Cette ligne dégradée fige le format sans inventer de valeur. Les valeurs absentes s’écrivent `unknown`; aucune localisation n’altère les clés ou valeurs machine.

Avec `--json`, stdout contient uniquement:

```json
{
  "schema_version": 1,
  "binary": {
    "version": "devel",
    "commit": null,
    "modified": null,
    "build_date": null,
    "provenance": "unknown"
  }
}
```

Les diagnostics restent sur stderr et une option invalide retourne 2 selon le contrat CLI existant. La commande de version elle-même reste exploitable même si le root n’existe pas, si `.swarm` est absent ou illisible, si SQLite est indisponible, ou si `git` n’est pas installé.

### Comparaison binaire / sources pour le web

La projection santé conserve une clé `version`, mais son schéma cible devient:

```json
{
  "binary": {
    "version": "devel",
    "commit": null,
    "modified": null,
    "build_date": null,
    "provenance": "unknown"
  },
  "source": {
    "available": false,
    "commit": null,
    "modified": null,
    "compared": false,
    "matches_binary": null
  }
}
```

`source` décrit uniquement le projet local explicitement servi. `matches_binary` n’est booléen que si les deux commits sont connus; un checkout sale reste signalé séparément. Une absence de Git, de checkout ou de permission produit l’état ci-dessus, sans faux écart et sans bloquer la santé du runtime.

## Contrat d’historique embarqué et de modale

### Données

- Un manifeste public embarqué, versionné avec les sources, porte `schema_version: 1` et une liste `releases`. Une entrée n’est admise que pour un tag de release vérifié; aujourd’hui la liste doit rester vide et l’UI doit dire qu’aucune version publiée n’est répertoriée.
- Chaque future entrée contient exactement: `version` (tag validé), `date` (`YYYY-MM-DD` de publication vérifiée), `summary.fr`, `summary.en`, `commits[]` et `release_url`. Chaque commit contient `sha` complet et `url`; les libellés éditoriaux éventuels sont bilingues.
- Limites proposées: 20 releases embarquées, 50 commits par release, 8 Kio par résumé localisé. Le parseur rejette le manifeste entier si son schéma, ses types ou ses bornes sont invalides; l’UI affiche alors l’indisponibilité sans retomber sur Git ou le réseau.
- Les liens sont des URL HTTPS prévalidées sous l’origine GitHub canonique du projet, avec chemins `/commit/<sha>` ou `/releases/tag/<tag>`. Cette origine reste une donnée à fournir par le responsable: le remote courant ne la prouve pas. Un lien séparé « Historique complet » est affiché seulement après configuration de cette URL.
- Le rendu utilise du texte échappé (`textContent` ou équivalent), jamais du HTML issu du manifeste. Aucun `git log`, aucune commande arbitraire, aucune récupération périodique et aucune mutation du manifeste au runtime.

### Interaction accessible bilingue

- Deux déclencheurs visibles, cockpit et préparation, portent « Version et nouveautés » / “Version and what’s new”. Ils ouvrent le même composant et le même jeu de données.
- La fenêtre a `role="dialog"`, `aria-modal="true"`, un nom accessible localisé, place le focus initial sur le titre ou le bouton de fermeture, enferme Tab/Maj+Tab, se ferme par bouton et Échap, puis restitue le focus au déclencheur exact. La fermeture par clic extérieur n’est permise que si elle ne remplace pas ces mécanismes.
- Le fond est rendu inerte pendant l’ouverture. Tous les états utilisent les jetons sémantiques existants et doivent être contrôlés en thèmes clair/sombre, FR/EN et au clavier.
- Ordre du contenu: identité du binaire; comparaison facultative aux sources locales; statut `modified`; date de build; nouveautés publiées; lien vers l’historique complet. La modale n’interprète jamais une version de sources comme la version installée.

### Cas dégradés obligatoires

| Cas | Effet attendu |
| --- | --- |
| Métadonnées de build absentes | `devel`, champs inconnus et provenance `unknown`; commande toujours exit 0 |
| Git absent / root non-checkout / permission refusée | Identité binaire intacte; source indisponible, non comparée, santé inchangée |
| Binaire et sources divergent | Deux commits affichés, statut explicite « différent »; aucune mise à jour automatique |
| Sources sales | Badge modifié distinct d’un écart de commit |
| Aucun tag/release vérifié | Message « Aucune version publiée répertoriée » / “No published version is listed”; aucune release synthétique |
| Manifeste absent, malformé ou hors bornes | Message « Historique indisponible » / “Version history unavailable”; aucun fallback Git/réseau |
| URL non canonique ou commit invalide | Lien masqué/rejet du manifeste selon validation; jamais de lien construit à partir d’entrée libre |
| JSON demandé | Clés et valeurs machine canoniques, aucun texte localisé ni sortie parasite |
| SQLite indisponible | `swarm version` et `swarm --version` restent fonctionnels car traités avant le store |

## Inventaire des builds et modifications attendues par la suite

| Chemin observé | Commande actuelle | Risque de provenance | Contrat d’alignement recommandé |
| --- | --- | --- | --- |
| `Makefile` `build` | `CGO_ENABLED=0 go build -trimpath -o bin/swarm .` | seulement marquage Go implicite | recette canonique injectant les cinq champs |
| `install.sh` natif | même build direct vers un fichier temporaire | peut diverger du Makefile | appeler la même recette/variables, puis installation atomique existante |
| `Dockerfile` | build direct sous Go 1.26 | présence de `.git` au build non prouvée; aucune injection | recevoir des `ARG` non secrets et appliquer exactement la recette canonique |
| CI `make smoke` | passe par `make build` | valide actuellement un binaire sans version explicite | fournir des valeurs de test déterministes et vérifier `version` texte/JSON |
| CI `test-install` | script d’installation déclenché séparément | couverture exacte hors du périmètre de lecture | ajouter assertions native/Docker liées au SHA candidat dans la tâche d’implémentation |

La recette commune doit échouer en release si version, commit ou date exigée manque; un build développeur direct reste permis avec les valeurs dégradées ci-dessus. Ne pas mettre un horodatage variable dans `go generate` ou le manifeste versionné.

## Findings the responsible planner must know

- L’URL GitHub publique est une décision/preuve manquante; ne pas transformer le remote local géré en lien utilisateur.
- La forme actuelle de `ServerVersion` n’exprime pas les inconnues pour `Modified`/`Current` et mélange identité binaire et source. Une migration JSON est à traiter explicitement par les tâches API/web et leurs tests.
- L’installation documente Go 1.24+ tandis que Docker/CI utilisent 1.26; cette compatibilité doit être décidée avant de figer une promesse d’installation, sans être corrigée par cette tâche en lecture seule.
- Il faudra examiner `.dockerignore` dans une tâche autorisée avant d’affirmer le comportement du marquage VCS automatique pendant le build Docker.
- Le chemin contractuel de livrable mentionnait `docs/plans/version-history/V1-handoff.md`, mais l’instruction d’exécution courante impose un unique rapport `docs/plan-10b0e692d2-V1.md`; ce worker suit cette dernière et n’écrit pas un second rapport ambigu.

## Commandes et résultats observés

- `pwd && git rev-parse --show-toplevel && git rev-parse HEAD && git branch --show-current && git status --short`: exit 0; racine attendue, SHA `8d1e223053d7c047bde94276ea3094bd748c82f5`, branche `codex/version-history`, changement initial `?? docs/plans/version-history/`.
- `rg --files -g 'runtime_health.go' -g 'main.go' -g 'Makefile' -g 'install.sh' -g 'Dockerfile' -g '.github/**'`: exit 0; cinq fichiers racine. La reprise avec `rg --files --hidden .github` a trouvé les trois fichiers `.github` listés plus haut.
- Recherche ciblée `rg -n -i 'version|commit|build|ldflags|git|docker|release|health|json|flag|os\\.Args' ...`: exit 0; points d’entrée cités dans l’état observé.
- `git tag --list`: exit 0, stdout vide. `git describe --tags --exact-match HEAD`: exit 128, diagnostic attendu « No names found, cannot describe anything. »; aucune répétition.
- Recherche exacte des entrées `--version`, `pos[0] == "version"`, `buildVersion`, `buildDate`: exit 1, interprété explicitement comme « aucune correspondance ». Les recherches exactes des recettes de build et de `ServerVersion` ont retourné exit 0 avec les lignes citées.
- Une première enveloppe de recherche des workflows de release a elle-même retourné exit 1 parce qu’une chaîne `&&` arrêtait l’enveloppe sur l’absence attendue de correspondance. Après correction de cette précondition (capture explicite du statut), `git ls-files .github` a retourné exit 0 et la recherche `release|upload-artifact|goreleaser|docker/build-push` exit 1 attendu; aucun workflow de release trouvé.
- Assertions de structure sur le rapport (`test -s` et `rg -q` pour les contrats, exigences, deux solutions et cinq checks obligatoires): exit 0, `report_contract_assertions=PASS`; taille observée 190 lignes, 2 742 mots, 19 084 octets avant la présente mise à jour.
- Recherche d’espaces finaux: `rg -n '[[:blank:]]+$ docs/plan-10b0e692d2-V1.md`, exit 1 attendu, aucune correspondance. `git diff --check`: exit 0. `git diff --name-only`: exit 0, aucune modification suivie. `git status --short`: le présent rapport et le répertoire préexistant `docs/plans/version-history/` sont les seuls chemins non suivis.
- Aucun build, test applicatif, serveur, base SQLite ou fixture de mission n’a été lancé: la tâche est un cadrage en lecture ciblée, pas une implémentation.

## Changes and verification

| Requirement | Change / file | Exact check and environment | Observed effect | Result | Evidence / limit |
| --- | --- | --- | --- | --- | --- |
| `req-1` | Rapport uniquement | `rg -n -i 'version|commit|build|ldflags|git|docker|release|health|json|flag|os\\.Args' runtime_health.go main.go Makefile install.sh Dockerfile .github` (exit 0) | Contrat CLI/JSON, version, commit, dirty, date et provenance cadrés à partir des entrées réelles | PARTIAL | Revue indépendante absente; aucune exécution du futur comportement |
| `req-2` | Rapport uniquement | lecture bornée des quatre recettes et de `.github/workflows/ci.yml`; `git tag --list` (exit 0, vide) | Inventaire, choix embarqué, contrat modal et dégradations documentés sans release inventée | PARTIAL | URL GitHub canonique non prouvée; tests UI hors de cette tâche |

## Cadre de gates (constat du worker, sans décision de gate)

| Check obligatoire | Phase | Mandatory | État observé | Preuve / limite |
| --- | --- | --- | --- | --- |
| `plan-entry` | entry | true | PARTIAL | Racine, SHA, rôle et périmètre confirmés; état sale préexistant préservé. Le moteur/évaluateur reste seul juge du gate. |
| `plan-criterion-1` | validation | true | PARTIAL | Contrat écrit et fondé sur les sources; comportement non implémenté et non rejoué. |
| `plan-criterion-2` | validation | true | PARTIAL | Inventaire et dégradations écrits; URL canonique et revue manquent. |
| `plan-validation` | validation | true | NOT TESTED | Revue indépendante non fournie à cette tentative; aucun auto-PASS. |
| `plan-delivery` | delivery | true | NOT TESTED | Le relais moteur intervient après la fin; présence du rapport ne prouve ni remise ni acceptation. |

## APEX / PDCA checkpoint

- Analysis / PLAN: état du checkout, entrées CLI/version et quatre chemins de build inventoriés; hypothèse « aucun tag » confirmée
- Execution / DO: rédaction du contrat uniquement; aucune modification applicative ni runtime
- Verification / CHECK: contrôles statiques ciblés, assertions de structure, espaces finaux, diff et état de copie vérifiés avec les résultats ci-dessus
- Adjustment / ACT: proposer l’historique embarqué avec injection commune; transmettre les deux préconditions (URL canonique, revue indépendante)
- Recovery limits: tentative 1/3; plafond 100 appels, jalons respectés; aucun retry identique ni hausse de budget

## OODA / état courant

- Observation: métadonnées Go partielles déjà présentes, zéro tag, quatre recettes non injectées, origine Git locale
- Orientation: une solution runtime-Git ne donne ni fidélité du binaire ni portabilité; une solution embarquée rend l’échec observable et borné
- Décision: retenir injection build + BuildInfo en repli + manifeste embarqué; garder `devel` tant qu’aucune release réelle n’existe
- Résultat: contrat proposé; manque d’URL canonique rendu explicite plutôt que contourné

## Next action and limits

Le responsable doit fournir/valider l’URL GitHub canonique et faire réaliser l’implémentation séquentielle sur la base de ce contrat, puis demander une revue indépendante du même SHA/diff. Cette vérification personnelle ne vaut pas revue; l’acceptation et les gates restent exclusivement au moteur.
