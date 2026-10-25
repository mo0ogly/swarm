# PAEA — incrément 08 : consommateur CLI Rust du contrôle de reçu

[English](../../../en/plans/actions-confiance/paea/INCREMENT-08.md) · [Plan](../PAEA-APEX.md)

## Besoin, décision et état réel

Reprise du programme demandée le 25 octobre 2026 dans le chat « Retrouver le projet
APEX APAEA ». L’utilisateur reconfirme explicitement **sans compilation ni
redémarrage**. Codex racine possède cette tranche ; aucun agent, appel GLM,
service, mission ou déploiement n’est démarré. Les travaux concurrents restent
à leurs propriétaires ; les preuves des incréments 01…07 ne sont pas réécrites.

Le contrôle Rust de l’incrément 07 n’a aucun consommateur CLI. Deux options :
ajouter un service actif, ce qui exigerait l’autorité protégée PA-02 encore
indisponible ; ou raccorder un consommateur hors ligne aux contrats existants.
La seconde est retenue pour PA-01/03/09, sans prétendre avancer l’activation.

`swarm-rust protection check-receipt ACTION RECU SERVICE ETAT_PRECEDENT --json`
est écrit en sources : deux fichiers réguliers, état explicite, décodage strict,
recalcul canonique et appel de `check_receipt_transition`. Les neuf champs de
sortie sont décrits dans `schemas/paea/receipt-check-v1.schema.json`. Autorité,
authenticité du reçu et effet restent `not-established`. Aucun état ni reçu n’est
enregistré. Lecture séquentielle, sans instantané ni fraîcheur. Même un reçu
forgé cohérent peut passer ce contrôle de correspondance.

**Code et quatre tests écrits, non compilés/non exécutés/non installés.**
L’aide FR/EN, le contrat FR/EN et la recette permanente sont actualisés. Les autres
targets restent inchangés : 5 tests historiques de schéma, 17 de contrat, 3 de
comparaison, 4 de reçu, 8 de CLI sur Linux ; le nouveau target `protection_receipt`
sélectionne 4 tests supplémentaires. Ce sont des sélections, pas un bilan PASS.

## Revue du changement

| Axe | Examen et limite |
|---|---|
| Politiques | Arity 6, refus du marqueur stdin, sérialisation `State`, version et marqueurs : invariants de protocole/framing. Lecture/transport et sortie : mécanismes existants. Aucun délai, budget, retry ou politique opérateur ajouté. La variable d’environnement du seul test enfant sert à son isolation, sans consommateur produit. |
| Fonctions | `execute` du module protection, nouveau `check_receipt`, aide dans `execute` racine ; 10 fonctions/méthodes de test documentées. Préconditions, erreurs, sortie partielle et absence d’autorité explicites. Rustdoc différé, commentaires examinés en sources. |
| Erreurs | Syntaxe : `command_failed` existant ; état invalide : `protection-document-invalid` avec cause serde ; lectures : codes/cause du lecteur commun ; liaison/cycle : erreurs existantes. Sérialisation complète avant sortie ; écriture/flush gardent la cause et peuvent suivre des octets visibles. Aucun effet projet déduit d’une erreur. |
| Documentation | Contrat/aide/schéma/recette et suivi FR/EN ; notes locales signalent le code non qualifié. Aucun nouveau format d’entrée ou dépendance Cargo. Les descriptions historiques et preuves exécutées gardent leur périmètre. |

La revue automatique fournit seulement des candidats lexicaux ; l’examen
sémantique est lié aux empreintes dans
[source-review.json](evidence/20261025-receipt-cli-xnl00wiz/source-review.json).
Ce document n’est pas la qualification des entrées effectives de compilation,
un verdict indépendant ou une preuve fonctionnelle.

## Tests préparés et amélioration de la méthode

Quatre tests nommés : deux domaines/FR/EN/toutes issues permises ; entrées
invalides, doublons Unicode, liaisons incompatibles, UNKNOWN et terminaux avec
témoins et reprises ; routage/framing avant lecture ; erreurs d’écriture et
flush avec cause originale, sortie partielle/complète et reprise.
Les refus comparent le champ JSON `code` exact, sans accepter sa simple présence
dans un message.

La revue a corrigé deux hypothèses de test avant toute exécution : le code commun
de syntaxe est `command_failed`, pas `usage`, et BrokenPipe ne signifie pas
`storage_unavailable` (réservé ici à ENOSPC). Les originaux avant correction sont
dans l’historique de ce chat ; aucun échec d’exécution n’est inventé.

Amélioration découverte : un test `run_detailed` appelé depuis le checkout
laisserait son Root par défaut viser ce checkout si une régression ouvrait un
projet. Origine : revue de cette tranche ; valeur : protéger les données réelles ;
owner : Codex/tests CLI. Décision réalisée : même exécutable de test lancé dans
un répertoire temporaire vide, sans changer le cwd global du parent. Critère
préparé : témoin valide, erreurs injectées, causes conservées, aucun `.swarm`
créé dans les deux répertoires. **Effet non vérifié**, compilation interdite.
Les prochains contextes reçoivent cette contrainte par la recette et cette note ;
aucun contexte embarqué ou processus existant n’est modifié.

## Preuves et reprise

Originaux propres avant modification, plan de reprise, commandes de contrôle,
diff et empreintes :
[dossier de preuve](evidence/20261025-receipt-cli-xnl00wiz/).
Les contrôles effectivement réalisés et leurs limites sont dans
[static-checks.json](evidence/20261025-receipt-cli-xnl00wiz/static-checks.json).
Le formatage/analyse syntaxique Rust n’établit ni typage, ni exécution, ni graphe
de features. Aucun Cargo, build, Rustdoc ou redémarrage n’est autorisé ici.
Format/analyse syntaxique, cohérence schéma/champs/états, sélection des quatre
tests, 128 liens locaux et absence d’erreur d’espacement ont été contrôlés.
Les empreintes finales et la portée de leur réexamen restent dans ces fichiers
de preuve ; aucun de ces contrôles n’exécute le parcours CLI Rust.

Reprise owner Codex : après levée explicite de cette consigne, figer les entrées,
exécuter les targets affectés et Rustdoc, puis la recette permanente dans un
répertoire neuf et son `verify`. Qualifier ensuite profil/binaire et livraison
locale par le chemin supporté, en préservant les travaux concurrents. PA-02 et
AC-02 gardent l’autorité et le confinement ; aucun raccord protégé n’est inventé.
PA-00…13, E2/E3 et le RETEX final restent ouverts ; évaluateur et équipe tierce
E3 restent à fournir par le commanditaire. Cette tranche a un état statique
non qualifié, sans acceptation globale.
