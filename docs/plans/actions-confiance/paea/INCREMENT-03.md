# PAEA — incrément 03 : refus de routage avec GLM

[English](../../../en/plans/actions-confiance/paea/INCREMENT-03.md) · [Plan](../PAEA-APEX.md)

Le 25 octobre 2026, « next avec glm » autorise une contribution HTTP ciblée.
La consigne sans compilation reste active. Choix : étendre la recette publique
sur le binaire existant plutôt qu’écrire du Rust impossible à qualifier ici.
Aucun fichier Rust/Cargo, service, mission ou réglage actif n’est modifié.

## Résultat observé

La [recette](evidence/20261025-glm-routing-99e363e5/public-recipe/result.json) passe : **6 tests nommés,
248 invocations, 179 succès et 69 refus attendus**. Deux tests sont ajoutés aux
quatre existants ; les comptes ne sont pas six tests supplémentaires. Le binaire
SHA256 `addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`
est stable avant/après ; il reste le candidat de test historique, non le serveur installé.
Le [transcript](evidence/20261025-glm-routing-99e363e5/transcript.json) conserve argv et octets observés.

| Contrat | Résultat et portée |
|---|---|
| Opérations actives | commit, approve, config-apply, mandate-create refusent avec `failure.code=command_failed`, stdout vide, exit2. |
| Options incompatibles | root différent de `.`, server avec valeur, output, task et phase audit refusent ; aucun fichier de sortie ni racine projet créé. |
| Compatibilité | `--root . --phase delivery` explicites donnent exactement le contenu canonique attendu. |
| Entrées | Fichier absent : `input-document-io` ; répertoire : `input-regular-file-required` ; stdin/fichier vides : `protection-document-invalid`. |
| Reprise | Chaque faute dans les deux domaines et langues FR/EN possède un témoin valide et une reprise exacte dans la même racine temporaire. |

Les nouveaux refus exigent aussi `retryable=false` et un diagnostic sans canari
privé. Les quatre tests antérieurs restent présents ; 88 invocations historiques
sont complétées par 160 invocations. Aucun test ne prouve confinement OS, effet
protégé, graphe de compilation, interface servie, installation ou qualification E2/E3.

## GLM, diagnostic et intégration

Un seul [appel](evidence/20261025-glm-routing-99e363e5/glm/result.json), sans retry ni watchdog global :
route SKYNET, modèle demandé/déclaré `glm-5.2`, effort demandé low ; version backend
non établie. Usage déclaré : 4 218 jetons entrée, 1 682 sortie, total 5 900 ; fin
stop, réponse complète en 9,147 secondes. Aucun prix n’est inventé.

Le modèle propose serveur/phase/fichier absent et la structure de contrôle/reprise.
Sa proposition n’est pas directement intégrable : un seul test pour un compteur
passant de quatre à six, appel `accepted(root=...)` inexistant, variante
`input_override` non définie et assertion sans fichiers incompatible avec le
nouveau fichier vide. Il omet les opérations actives demandées. Le
[replan](evidence/20261025-glm-routing-99e363e5/replan.json) conserve ces défauts avant toute exécution.
Codex réutilise les cas utiles, implémente deux tests complets et un helper de
refus commun, garde le seul slot `document.json`, et vérifie les codes exacts dans
le producteur réel `command_input.rs`. Aucun patch produit n’est déduit du texte GLM.
Le modèle n’a aucun accès outil/dépôt et ne fournit pas de revue indépendante E3.

## Revue des fichiers et transmission

La [revue liée aux octets](evidence/20261025-glm-routing-99e363e5/source-review.json) inventorie les sources et
preuves finales. Auto-revue ciblée, sans qualification globale de migration.

- Politiques : aucune nouvelle politique produit. Cas, langues et nombres de tests
  sont inventaires de recette ; exit2/codes JSON et valeurs root/phase sont les
  contrats existants. Pas de délai, budget, retry ou serveur ajouté. Les overrides
  restent limités au namespace hors ligne et aux chemins temporaires de cette recette.
- Fonctions : `invoke` accepte des variantes contrôlées ; `accepted` transmet les
  options ; `refused` vérifie code/sortie/retryabilité ; les deux nouveaux tests
  documentent préconditions, effets et limites. `main` attend six tests nommés.
  Les docstrings sont relues ; aucune Rustdoc compilée sous cette consigne.
- Erreurs : lecture/lancement/publication conservent leur propagation, erreurs
  CLI observées jusqu’à stderr et exit, reprise vérifiée. La preuve originale
  reste exclusive et atomique via le publieur existant. Une absence de fichiers
  n’est pas une démonstration de médiation des effets.
- Documentation : contrat, plan et file FR/EN raccordés. Pas de nouvelle note de
  version runtime : seuls tests/recette/docs changent, aucune livraison construite.
  Aucun contexte embarqué ou processus déjà lancé n’est actualisé par ces fichiers.

Apprentissage appliqué : une réponse GLM complète doit être vérifiée comme du code
non fiable, notamment appels, nombre de méthodes et invariants du harness ; fin
stop ne vaut pas intégrabilité. Les erreurs ont été corrigées avant exécution,
puis le parcours réel de recette a été vérifié. Les historiques restent inchangés.

## Suite et critères ouverts

PA-00/01 restent partiels ; l’incrément ne termine aucun lot du programme.
PA-02/03 exigent les producteurs d’autorité IA3 et un candidat Rust qualifiable.
Reprendre leur intégration après disponibilité de ces interfaces et levée explicite
de la consigne sans compilation ; Codex conserve l’intégration, les propriétaires
Identité/AC-02 conservent leurs contrats. L’autre session et ses modifications
sont préservées. Évaluateur indépendant et équipe tierce E3 restent à désigner
par le commanditaire. La recette ne les remplace pas ; PA-11…13 et RETEX restent ouverts.
