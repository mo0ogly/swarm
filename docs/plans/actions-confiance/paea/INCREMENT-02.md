# PAEA — incrément 02 avec GLM, recette publique sans compilation

[English](../../../en/plans/actions-confiance/paea/INCREMENT-02.md) ·
[Plan](../PAEA-APEX.md) · [Contrat](../../../architecture/PAEA-CONTRACT.md)

Le 25 octobre 2026, l’utilisateur autorise la suite « avec glm ». La consigne
précédente d’éviter de compiler pendant son autre session est conservée. Codex
applique APEX/replan et la skill personnelle `skynet-agents` ; la délégation HTTP
ne donne au modèle ni accès dépôt, ni outil, ni droit d’intégration ou de clôture.

## Décision et résultat

Deux voies ont été considérées : reconstruire un candidat Rust pour étendre les
contrats actifs, ou renforcer la vérification publique du binaire déjà disponible.
La première reste fermée par la consigne utilisateur et les dépendances du plan.
La seconde est réalisée : nouvelle recette maintenue, entrée/sortie réelle de la
CLI hors ligne, données des deux domaines, racines temporaires et oracles conservés.
Aucun fichier Rust, Cargo, serveur, base de mission ou réglage actif n’est modifié.

La [recette finale](evidence/20261025-glm-contract-bf9e66eb/public-recipe-02/result.json)
réussit : **4 tests nommés, 88 invocations**, 17 refus attendus et 71 succès.
Les refus comprennent 16 mutations de document et l’écriture sur `/dev/full`.
Chaque faute possède un témoin valide puis une reprise dans la même racine.
L’intégrité de la recette est VERIFIED. Le SHA256 du binaire reste
`addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`.

| Exigence locale | Observation et limite | Preuve |
|---|---|---|
| Deux domaines, FR/EN, montant exact et Git Unicode | Les 14 vecteurs donnent exactement les octets, empreintes, intentions et marqueurs d’absence d’autorité attendus, via fichier et stdin. | [Transcript original](evidence/20261025-glm-contract-bf9e66eb/public-transcript-02.json), test `test_all_frozen_vectors_match_public_bytes_in_both_languages`. |
| Ambiguïtés et autorité inventée refusées | Doublons racine/imbriqués/Unicode, flottants/exposants, dépassement i64, montant ambigu, UTF-8/Unicode invalides, second document, champ privé inventé, alias Git, OIDs incompatibles et no-op. Sortie vide, exit2, code attendu dans `failure.code`, canari privé absent. | Test `test_raw_ambiguities_refuse_between_valid_control_and_recovery`, 16 cas avec contrôles. |
| Encodages équivalents | Espaces, ordre des propriétés/labels et échappement Unicode légitime conservent les octets exacts attendus. | Test `test_equivalent_input_encodings_preserve_exact_content`. |
| Écriture échouée | `/dev/full` donne exit2 et `failure.code=storage_unavailable`, puis le même document valide réussit. Aucun fichier projet créé dans la racine de test. | Test `test_output_failure_is_nonzero_after_valid_control_then_recovers`. |
| Binaire existant seulement | Identité explicitement fournie, hash avant/après stable ; quelques métadonnées locales de build conservées, sans reconstituer un graphe complet. | [Observation du binaire](evidence/20261025-glm-contract-bf9e66eb/binary-observation.json). |

Les 26 tests de l’incrément 01 gardent leur preuve historique ; ces quatre tests
complètent le parcours public. Les 88 invocations ne deviennent ni 88 tests
indépendants, ni des campagnes T01…T22, ni une installation ou un verdict E2/E3.

## Contribution GLM effectivement utilisée

La route locale vérifiée associe l’alias `glm-5.3-flash` à `openai/glm-5.2` ;
l’identifiant demandé et renvoyé est `glm-5.2`. Cela n’atteste pas la version réelle
du backend. Aucune configuration de proxy n’est modifiée.

| Appel | Résultat déclaré par le service | Décision du principal |
|---|---|---|
| [Revue complète](evidence/20261025-glm-contract-bf9e66eb/glm-review/result.json) | 13 316 jetons d’entrée, 4 096 de sortie ; fin `length`, aucun texte final. L’effort demandé `low` ne prouve pas son application. | Réponse inutilisable, aucun verdict retenu. Original conservé, pas de retry automatique ni hausse du plafond. |
| [Reprise ciblée](evidence/20261025-glm-contract-bf9e66eb/glm-focused/result.json) | Décodeur brut/visiteur complet seul ; 1 100 jetons d’entrée, 1 032 de sortie, fin `stop`, texte final reçu. | Avis statique « aucun défaut démontré » dans ce seul code ; trois régressions proposées, adaptées et exécutées par Codex. |

Usage total déclaré : 19 544 jetons sur deux appels. Aucun coût monétaire n’est
déduit sans tarif connu. Le premier appel a consommé sa sortie ; il ne vaut pas
zéro consommation. Le second n’est ni une revue globale des contrats ni un rôle
d’évaluateur indépendant. GLM n’exécute aucun test.

Adaptation vérifiée : les exemples abstraits avec clés `a`/`v` auraient également
échoué pour schéma inconnu. Ils sont remplacés par une clé `kind` légitime dupliquée
avec Unicode, un `schema_version` ambigu et un second document après un témoin
PAEA valide. Le code public attendu est `protection-document-invalid` ; les détails
internes du visiteur ne sont pas exposés comme contrat CLI.

## Replan et erreurs conservées

La [première recette](evidence/20261025-glm-contract-bf9e66eb/public-recipe-01/result.json)
est FAIL : deux tests positifs passent, les assertions de refus lisent par erreur
`code` à la racine et produisent 17 `KeyError`. Le produit émet déjà l’enveloppe
`failure.code`. Lecture du producteur `report()` et du transcript → correction
des deux accès du consommateur de test → nouvelle recette dans un dossier neuf,
sur le même binaire. Les originaux ne sont pas réécrits ; aucun correctif produit
n’est déduit de cet incident de recette.

Les [décisions de reprise](evidence/20261025-glm-contract-bf9e66eb/public-recipe-replan.json)
conservent aussi un patch refusé sur une ancre documentaire périmée. L’outil
n’avait appliqué aucune édition partielle ; le texte réel a été relu avant reprise.

Le contrôle final conserve la distinction entre intégrité et réussite :
`recipe.py verify` rend exit1 pour une preuve intègre portant le statut FAIL,
et exit0 pour une preuve intègre PASS. Une première assertion du collecteur
attendait à tort exit0 pour l’essai échoué ; son interprétation est corrigée
après lecture du vérificateur, sans changer le verdict ni les originaux.

## Revue liée aux octets et transmission

Fichiers réalisés : `tests/paea_public_contract_test.py`, nouvelle recette
`tools/verification/recipes/paea-public-existing-binary.json`, contrats et suivis
FR/EN. L’inventaire final est dans le
[dossier de revue](evidence/20261025-glm-contract-bf9e66eb/source-review.json).
Il s’agit d’une auto-revue ciblée, pas du collecteur de qualification migration.

- Politiques : aucun nouveau choix opérateur. Les 14 vecteurs, 16 fautes et 4 noms
  de tests sont l’inventaire de cette recette ; exit2 et l’enveloppe sont des
  contrats CLI existants ; SHA256 est une représentation technique. Aucun délai,
  retry, appel natif ou budget produit ajouté. Les trois variables d’environnement
  désignent des artefacts de test, pas une configuration active de Swarm.
- Fonctions : helpers d’encodage/substitution, allocation, invocation, assertions,
  quatre tests et `main` documentent entrées, préconditions, effets, erreurs et
  limites. La substitution vérifie une occurrence unique pour refuser un cas
  devenu inopérant. `main` refuse avant collecte sans identité explicite et sans
  destination neuve ; le hash après exécution invalide un artefact changé.
- Erreurs : conservation des octets originaux, propagation des erreurs de lecture/
  lancement/publication, sortie non nulle sur échec. Le publieur atomique exclusif
  existant préserve les preuves. L’absence de `.swarm` n’est pas une preuve de
  confinement ; aucun service d’effet n’est ouvert par cette recette.
- Documentation : FR/EN, plan et file APEX raccordés. Aucune note de version locale
  supplémentaire : ce changement de recette/docs ne modifie aucun code livré et
  aucune construction/installation n’est autorisée ici. Aucun contexte embarqué
  ou processus démarré n’est modifié par cette transmission documentaire.

Apprentissage vérifié : consommer l’enveloppe du producteur réel, conserver le
témoin valide avant les mutations, et réduire une revue GLM épuisée à une frontière
complète plutôt qu’augmenter seulement son budget. Le succès de cette reprise
étroite ne garantit pas les prochains appels.

## Conditions de suite

PA-00/PA-01 restent partiels : mandats et reçus authentifiés, API/CLI actives et
preuves au service d’effet restent à raccorder aux producteurs IA3. Le chantier
Rust concurrent est préservé. Une réalisation Rust à reconstruire ou installer
attend la levée de la consigne sans compilation et un candidat partagé disponible.
L’activation conserve les dépendances Rust/IA3/AC-02. Le commanditaire ne dispose
toujours pas des deux rôles externes PA-11 ; protocole, campagnes et RETEX global
restent ouverts. Aucune campagne payante ou délégation supplémentaire n’est lancée
automatiquement par cet incrément.
