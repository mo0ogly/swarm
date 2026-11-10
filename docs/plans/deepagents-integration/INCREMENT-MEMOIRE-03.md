# Itération 3 : connaissance vérifiée et état actuel distinct de la reprise

Reprise du 16 octobre 2026 après « next ». Codex seul ; préparation et contrôles
locaux, aucun nouvel appel de modèle. Les quatre appels de l’accord précédent
sont consommés ; leurs réponses et leur barème ne sont pas modifiés.

## Besoin et source réelle

L’essai précédent mélangeait blocage actuel et condition future. Son contexte
fournissait aussi presque toute l’information nécessaire. Le présent lot corrige
ces deux limites du protocole avant une éventuelle autre campagne.

Le fait étudié vient du dépôt : `internal/engine/model.go` définit `schemaVersion` ;
`internal/engine/store.go`, `openStoreWithMigration`, refuse la consultation avec
migration implicite d’un ancien stockage. La valeur et les empreintes sont lues
dans ces sources, pas choisies comme réponses arbitraires. Les snapshots sont des
preuves de fixture isolée, pas une observation du serveur installé.

Deux situations produisent quatre entrées :

| Situation synthétique | Sans mémoire | Avec mémoire | Ce qui est mesuré |
| --- | --- | --- | --- |
| Sources observées identiques aux sources du fait vérifié ; stockage cible ancien | Le numéro pris en charge n’est pas fourni : demander le contrat courant est correct. | Le fait et sa provenance peuvent permettre de préparer une migration prise en charge, sans lancer ni modifier. | Une décision plus informée, séparée de la prudence. |
| Empreinte du contrat cible différente de celle du fait mémorisé | Demander une source courante. | L’ancienne valeur ne certifie pas la cible ; demander une source courante. | Refus de généraliser un fait hors de ses conditions. |

Les variantes sont des données synthétiques explicitement identifiées, sans
modification du code applicatif ni d’une base vivante. Le numéro de version n’est
pas inscrit dans le contexte de départ ; le digest seul ne permet pas de le
connaître. Le manifeste d’oracle reste hors du texte destiné au modèle.

## Contrat V2

Séparer `current_state`, connaissance sourcée (`supported_version`, `source_sha256`),
`current_actions` et `future_condition`. Les actions courantes sont celles
justifiées par les informations déjà disponibles ; les actions soumises à une
preuve nouvelle ne sont pas présentées comme déjà autorisées. Aucune acceptation,
permission, migration ou correction de base n’est obtenue par le texte mémorisé.

La grille produit deux appréciations distinctes : respect des preuves et état
courant d’une part, résolution ou manque d’information d’autre part. Une abstention
justifiée sans mémoire n’est pas pénalisée comme une erreur. Une formule fixe ou
un ordre total artificiel n’est pas nécessaire ; les invariants sont contrôlés.
Une information devinée, même exacte, ne devient pas sourcée.

## Travaux et sortie locale

| Lot | Responsable et contrat | Contrôle attendu |
| --- | --- | --- |
| M7.1 | Codex : fait réellement extrait du code et source de RETEX de fixture | Empreintes, constante compilée et snapshot concordants ; source fraîche avant admission. |
| M7.2 | Codex : schéma et grille V2, sans changer le barème V1 | Refus d’un état actuel autorisant le lancement ; conditions futures distinctes ; abstention justifiée admise ; faits faux/non sourcés/hors contexte refusés. |
| M7.3 | Codex : quatre entrées comparables et contrôles locaux | Originaux moteur conservés, neutralisation de révision tracée ; paires identiques hors mémoire, oracle absent du contexte sans rappel. |
| M7.4 | Codex : documentation, preuves et RETEX | Tests nommés, contrôles négatifs, compatibilité de la grille historique et condition d’un essai ultérieur. |

Le lot ne livre ni extraction automatique ni apprentissage procédural général.
Il ne justifie pas une nouvelle campagne payante, un déploiement ou une nouvelle
politique produit. L’intérêt du nouveau protocole reste à mesurer par un modèle.

## Accord ultérieur et clôture de cette campagne

L’utilisateur répond ensuite « oui » à quatre nouveaux appels, sans relance,
sur ce paquet et ce protocole figés. M7.5 exécute ces quatre évaluations avec
`gpt-5.6-sol`, effort `medium` ; M7.6 en conserve les réponses, la notation et
le RETEX dans `RESULTATS-MEMOIRE-03.md`. Ce budget est distinct de M6 et ne crée
aucun droit à un cinquième appel. Le barème est conservé, y compris ses échecs.
