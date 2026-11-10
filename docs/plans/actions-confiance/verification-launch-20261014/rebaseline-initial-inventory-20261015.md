# Reprise du cadrage — Actions de confiance

État vérifié le 15 octobre 2026, révision publique 468.
Demande utilisateur : reprendre depuis le début et examiner ce qui a été fait.
La planification est suspendue par l’opération publique pendant cette reprise.
Les sources, preuves, identités, deux tentatives T2 et 90 appels de revue sont
conservés. Ce bilan remplace les anciens résumés comme point de départ courant.
Il ne supprime aucun relevé historique et ne crée ni nouvelle mission ni acceptation.

## Besoin repris

Permettre à un opérateur de définir un mandat, authentifier une demande, approuver
les paramètres exacts, engager un effet autorisé et retrouver ou réconcilier son
résultat avec une preuve vérifiable. Biba est le modèle d’intégrité retenu ; eBPF
est une observation système distincte. Le pilote reste une simulation sans paiement
réel. Le périmètre convenu reste de 14 stories, R01–R20 et huit lots, extensions
d’entreprise et RETEX compris.

## Réalisations et limites

| Lot | État établi | Ce que cet état prouve |
| --- | --- | --- |
| T1 — Architecture | Accepté | Contrat de 20 exigences, 14 stories, frontières et interfaces ; documentation et revue de spécification. Cela ne démontre pas une fonctionnalité exécutée. |
| T2 — Pilote Biba | Produit, contrôlé, non accepté | Sources, CLI/API, interface FR/EN, mandats, accord exact, effet simulé, réconciliation et preuves locales dans une copie gérée. Huit contrôles T2 et sept contrôles T1 passent. |
| T3 — Identités d’entreprise | À réaliser | Aucun lot accepté pour OIDC, cycle de vie et délégations. |
| T4 — Données et secrets | À réaliser | Classification, destinations autorisées et secrets par action restent à livrer. |
| T5 — Processus métier | À réaliser | Définitions versionnées, dossiers, validations et UX indépendante restent à livrer. |
| T6 — Preuves et contrôles avancés | À réaliser | Ancrage externe, rotation, conservation, simulations adversariales, second connecteur et référentiels restent à livrer. |
| T7 — Observation eBPF | À réaliser | Pas de collecteur intégré accepté ; collecte réelle et autorité système restent à démontrer. |
| T8 — Recette globale et RETEX | À réaliser | Aucune qualification globale ni fin du programme. |

**Un lot accepté sur huit. Le programme n’est pas qualifié pour diffusion.**
Le code principal du checkout et le binaire opérateur ne contiennent pas le pilote
T2. Celui-ci est conservé dans
`.swarm/managed/w-68b50cc82ec3fe55624f9549/copies/plan-68b50cc82e-T2-2`.
Son arbre Git correspond exactement à la remise `08ede6843a08ffec564a4c009da7161478f60f0b`.
Les mécanismes génériques déjà présents dans Swarm ne valent pas livraison des
stories de ce programme ; leur réutilisation doit être vérifiée au point d’effet.

## Preuves réutilisables

T1 est accepté sur `a2d6410389c03e16270fa43d3f50b5deeb04f313`.
Son contrat est `docs/architecture/actions-confiance-contract.json` dans cet objet
Git, avec 20 exigences et 14 stories.

Le résultat T2 conservé est `bdb4d220a97220fd47f51787d9bb0a5e5b856706`.
Les reçus natifs du candidat `a6486106403b64c2c344568576479dc605742ac1`
contiennent les 126 tests Go nommés passés (33 principaux, 93 sous-cas), les sept
cas publics, les contrôles navigateur et les trois cas de portabilité 386.
Les sorties de validation ne sont pas tronquées. Les défauts précédents et leurs
contre-exemples restent conservés : mandat périmé, publication interrompue et
reprise avant authentification. Les tests passés restent attribués à leur candidat,
sans devenir une nouvelle acceptation après la panne.

## Blocage présent

Après le redémarrage récent du système, le conducteur ancien est absent et un
objet Git candidat est vide dans le dépôt d’intégration : `a648610...`.
Le refus public est enregistré en révision 465. Le résultat T2, son arbre et le
commit T1 se lisent dans le dépôt et dans la copie gérée ; le commit candidat
ne se lit pas. La cause exacte de la perte de contenu n’est pas établie.
Les reçus et le checkpoint ne remplacent pas cet objet manquant.

Le conducteur unique rétabli est identifié par
[son reçu](verification-launch-20261014/restart-20261015-120502-process.json).
La pause de cadrage en 468 empêche les relances pendant l’inventaire.
Propriétaire du traitement : Codex, côté moteur et stockage d’intégration.
Condition de reprise : restaurer l’objet exact depuis une preuve intègre ou
fournir une récupération publique traçable du candidat, puis vérifier son graphe
Git et obtenir la revue indépendante sans modifier les anciennes preuves.
Ne pas effacer l’objet vide, réécrire le checkpoint ou accepter sur les seuls tests.

## Plan repris

1. Conserver ce bilan comme base et relier chaque story aux sources et preuves.
2. Traiter le stockage du candidat et sa durabilité, avec un contre-exemple isolé
   et une récupération publique. Cette réparation du moteur reste distincte du pilote.
3. Obtenir une revue indépendante de T2 et vérifier son intégration effective ;
   conserver ouverts les critères refusés ou non démontrés.
4. Livrer T3 puis T4 ; reprendre T5, T6 et T7 sous leurs dépendances et autorités
   réelles, sans les présenter comme existants à partir du seul cadrage.
5. Exécuter la recette complète du candidat final puis le RETEX.

Chaque étape garde ses critères observables, son propriétaire et ses preuves.
L’interdiction des délais de réponse longs reste active : messages courts,
lectures et sorties bornées, point utile au moins toutes les 60 secondes en travail
actif. Le réveil programmé ne constitue pas une preuve de réparation.

## Couverture des stories à la reprise

| Story | Lot | État de livraison |
| --- | --- | --- |
| s01-mandat — Définir un mandat vérifiable | T2 | Produit dans le pilote, sans acceptation |
| s02-reglement — Autoriser un règlement simulé | T2 | Produit dans le pilote, sans acceptation |
| s03-reprise — Reprendre sans double règlement | T2 | Produit dans le pilote, sans acceptation |
| s04-preuve — Vérifier les preuves du règlement | T2 | Produit dans le pilote, sans acceptation |
| s05-pilotage — Traiter les demandes du pilote | T2 | Produit dans le pilote, sans acceptation |
| s06-recette — Disposer d’un pilote qualifié | T2 | Produit dans le pilote, sans acceptation |
| s07-identites — Relier les identités d’entreprise | T3 | À réaliser dans ce programme |
| s08-donnees — Limiter données et secrets transmis | T4 | À réaliser dans ce programme |
| s09-processus — Suivre des processus et dossiers versionnés | T5 | À réaliser dans ce programme |
| s10-conservation — Renforcer la conservation et les clés | T6 | À réaliser dans ce programme |
| s11-simulation — Éprouver une politique sans effet | T6 | À réaliser dans ce programme |
| s12-connecteurs — Ajouter un autre connecteur sensible | T6 | À réaliser dans ce programme |
| s13-referentiels — Évaluer les référentiels applicables | T6 | À réaliser dans ce programme |
| s14-ebpf — Observer les agents avec eBPF | T7 | À réaliser dans ce programme |

[État public et pause](verification-launch-20261014/rebaseline-20261015-pause-applied.json).
[Observation du redémarrage et des reçus](verification-launch-20261014/restart-20261015-120502-observation.json).
