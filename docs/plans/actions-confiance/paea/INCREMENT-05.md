# PAEA — incrément 05 : comparaison Rust, qualification différée

[English](../../../en/plans/actions-confiance/paea/INCREMENT-05.md) · [Plan](../PAEA-APEX.md)

## Résultat réel

`swarm-protection-contract` expose désormais dans ses sources `compare_documents`
et les types `DocumentComparison` / `DocumentIdentity`. Le module
[comparison.rs](../../../../rust/crates/swarm-protection-contract/src/comparison.rs)
consomme directement deux documents, réutilise les décodeurs et la canonicalisation
Rust existants et produit les chemins modifiés sans valeurs. Les états de contenu,
intention métier et transport sont distingués. Aucun accord, mandat ou effet n’est
validé ; les trois champs d’autorité restent `not-established`.

**Écrit et relu statiquement, non compilé, tests Rust non exécutés, non installé.**
La consigne utilisateur sans compilation ni redémarrage reste active. Le module
n’ajoute pas d’opération CLI/API ; le parcours opérateur PA-08 reste à raccorder.
La commande historique `protection canonicalize` n’est pas changée. Les preuves
01…04 ne qualifient pas ces nouveaux octets. PA-01/08/09 restent partiels.

## Replan après erreur de méthode

Codex avait choisi un nouveau comparateur Python parce que le vérificateur de
correspondance existait et qu’il pouvait l’exécuter sans compilation. L’utilisateur
conteste deux fois ce choix : « pourquoi python ? », puis « oui cest quoi cette
histoire de python??? ». **Interdire de compiler n’interdit pas d’écrire du Rust.**
Multiplier les outils périphériques ne satisfait pas le raccord du moteur.

Décision : retirer les cinq nouveaux fichiers Python/test/recette/documentation
des sources maintenues et préserver leurs originaux dans `withdrawn-python/`.
Les trois recettes Python PASS restent des preuves historiques du candidat retiré,
sans valeur de qualification pour Rust. Le vérificateur Python de l’incrément 04
reste inchangé : cette reprise ne supprime pas les réalisations antérieures.
Le module de comparaison est repris en Rust, dans la bibliothèque existante.
Le [replan original](evidence/20261025-glm-compare-0af66401/replan-language.json)
conserve cause, décision et correction du suivi documentaire : le chemin EN de la
file APEX est `docs/en/APEX-QUEUE.md`, pas le chemin supposé initialement.

La méthode à transmettre : contraintes d’exécution et choix de langage sont
indépendants. Les prochains raccords fonctionnels restent dans le moteur Rust ;
un code non compilé garde ce statut, sans substitution de tests Python.

## Contrat et revue statique

`compare_documents(before_raw, after_raw)` valide les deux documents selon les
invariants Rust existants, y compris i64/OIDs/sources, puis compare les octets
canoniques réellement calculés. Contrairement au diagnostic Python retiré, il
ne consomme pas d’export ni de schéma JSON externe ; il ne duplique pas l’admission.
Les SHA256 lient les documents réellement fournis ; les chemins viennent uniquement
des champs des types sérialisés par le programme. Les tableaux sources sont
comparés en entier, sans équivalence implicite d’indice. Les labels sont normalisés
par le producteur existant ; les sources gardent leur ordre.

Aucune nouvelle dépendance Cargo, limite, politique opérateur, attente, IO,
clé, horloge ou effet. Les constantes sont des champs du protocole ou marqueurs
de non-autorité. Les échecs des décodeurs/canonicaliseurs conservent `Error` et son
code sûr ; les incohérences internes de forme ont le code statique
`protection-comparison-shape-invalid`. Aucun résultat partiel n’est retourné.
Le code n’effectue aucune publication de sortie à réconcilier. Les digests et
chemins restent des métadonnées à distribuer selon les droits du destinataire.

Trois tests Rust sont écrits, **aucun lancé** : deux domaines, transport/labels et
ordre des sources ; refus sur chaque côté et reprise. Ils utilisent les vecteurs
gelés et les vraies validations. La revue des fonctions et erreurs liée aux octets
est conservée dans `source-review.json` ; elle ne remplace pas Cargo/Rustdoc ni
une revue indépendante. Aucune qualification globale n’est déclarée.

## GLM et preuves conservées

Un appel SKYNET autorisé finit `stop`, sans plafond client ni effort imposé :
7 656 jetons entrée, 30 301 sortie, 37 957 total déclarés, 162,217 secondes.
Modèle demandé/déclaré `glm-5.2`, sans preuve de version backend. La proposition
Python complète est historique après la correction utilisateur ; elle n’est pas
une revue du Rust final. Les distinctions de contenu/intention/transport, tableaux
sources entiers, vocabulaire des chemins et cas des deux domaines sont réutilisés
par Codex dans le contrat Rust ; aucune proposition n’est appliquée aveuglément.
Les requêtes, originaux et résultats restent dans le dossier de preuves.

## Reprise du besoin principal

Codex porte la prochaine qualification : lorsque la compilation sera autorisée,
exécuter le seul target d’intégration `comparison` de ce crate, puis contrôles
ciblés concernés, Rustdoc et raccord CLI/API/opérateur selon le contrat retenu.
Les remises protégées IA3 et AC-02 restent nécessaires à PA-02/03. L’autre session
et ses travaux SCALING-S2 sont préservés. Pas de build/restart, installation,
commit, push, publication ou nouvelle mission. Évaluateur/équipe tierce E3 restent
à désigner par le commanditaire. Le programme PA-00…13 et le RETEX restent ouverts.
