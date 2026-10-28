# PAEA — incrément 07 : liaison reçu/action/cycle en Rust

[English](../../../en/plans/actions-confiance/paea/INCREMENT-07.md) · [Plan](../PAEA-APEX.md)

## Résultat réel et décision de reprise

Reprise autorisée « go next », méthode APEX/replan et contribution GLM SKYNET.
La comparaison des incréments 05/06 ne rattache pas une observation à son action.
Le décodeur `decode_receipt` valide seulement la représentation. Deux approches :
créer un nouveau reçu/service autonome, ou réutiliser action canonique, reçu et
cycle existants. La seconde évite formats parallèles et fausse autorité.

Le module Rust `receipt.rs` ajoute `check_receipt_transition(action, receipt,
expected_sink, previous)`. Il valide le reçu, le service attendu et l’égalité
exacte des digests d’action/intention/service, puis demande au cycle existant la
transition correspondant à l’issue déclarée. La bibliothèque exporte la fonction.
Aucun nouveau composant Python, aucune nouvelle dépendance Cargo.

**Sources écrites, non compilées, non installées ; quatre tests écrits, aucun
exécuté.** La consigne sans compilation ni redémarrage reste active. Pas de nouvelle
commande CLI/API ni de mutation du runtime. PA-01 est complété en sources pour ce
sous-contrat ; PA-03/09 restent ouverts, sans service d’effet ou preuve authentifiée.

## Contrat, erreurs et limites

Le service attendu doit respecter la grammaire d’identifiant existante. Ce n’est
pas une politique de destination. `CanonicalAction` est produit par les validations
existantes ; `EffectReceipt::validate` s’applique même si le reçu a été construit
directement en Rust plutôt que décodé. Aucun montant, état, droit ou effet n’est
modifié. Aucun paramètre opérateur, délai, retry ou plafond n’est ajouté.

| Issue | Résultat proposé par le checker, sous les préconditions du cycle existant |
|---|---|
| COMMITTED / UNKNOWN depuis COMMIT_REQUESTED | Transition déclarative vers l’issue correspondante. |
| DENIED depuis une phase antérieure non terminale | Refus déclaré selon `State::transition`. |
| RECONCILED_COMMITTED / RECONCILED_NOT_COMMITTED depuis UNKNOWN | Transition de réconciliation déclarative seulement. |
| DENIED, COMMITTED brut ou UNKNOWN depuis UNKNOWN | Erreur de transition ; aucune mutation. |
| Toute issue depuis un état terminal | Refus d’avancement/rejeu ; lire un résultat historique est une opération distincte. |

Erreur du reçu : `protection-effect-receipt`. Service attendu invalide :
`protection-receipt-expected-sink`. Contenu/intention/service incompatibles :
`protection-receipt-binding`. Cycle impossible : `protection-transition-invalid`.
Les diagnostics sont statiques, sans entrée privée ni valeur de service. Le type
`Error` existant est conservé. Il n’y a ni IO ni publication/flush dans cette
fonction pure, aucun état partiel à annuler ni effet à réconcilier par elle.

**Le succès n’authentifie rien.** Action, reçu, état précédent et service attendu
peuvent tous être des données fournies. Même un reçu forgé cohérent peut passer.
La fonction ne vérifie ni observation indépendante, ni signature, ni digeste d’un
artefact réellement observé, ni actualité de l’autorité, ni ordre protégé, ni
consommation/unicité. Le propriétaire protégé devra obtenir ces preuves avant de
persister ou agir sur l’état retourné. Aucun booléen « vérifié » fourni par l’agent
ne remplace ce raccord. Le schéma reçu existant n’est pas changé.

## Tests et preuve différée

Quatre tests Rust dans le target `receipt`, non lancés :

- Les deux domaines et toutes les issues admises, avec reçu de fixture explicitement
  non authentifié : le test borne le résultat à une correspondance.
- Mauvais digest d’action, intention ou service ; contenu alternatif sous la même
  intention ; témoins avant/après et entrées inchangées.
- UNKNOWN et états terminaux : pas de refus inventé, réussite brute ou rejeu.
- Issue non recevable dans un reçu et service attendu invalide ; reprise valide.

La recette `paea-contract` comprend désormais ce target exact et ses quatre noms,
sans lancement. Les cibles antérieures restent inchangées (17 contrats, 3 comparaison,
8 CLI sur Linux, plus 5 tests historiques schéma/oracle). Ces nombres décrivent
les sélections, **pas des tests passés sur ce candidat**. La revue liée aux octets
couvre fonctions, politiques, erreurs et dépendances ; elle ne remplace pas une
compilation ou une acceptation indépendante.

## Dépendances et suite

L’examen des vrais producteurs Identité a confirmé que l’enrôlement humain et
ses permissions ont déjà leurs propres validations ; un ancien `Statement` avec
un ancien `Expected` peut encore satisfaire la vérification après révocation,
comme le documente `decode_active`. Aucune fraîcheur n’en est déduite. PA-02 garde
son propriétaire Identité/intégration ; AC-02 sa frontière. Cette tranche prépare
le consommateur de preuve sans prétendre livrer ces préconditions.

Codex doit compiler et exécuter les targets affectés, Rustdoc et recette dès
levée de la consigne utilisateur ; ensuite raccorder les services et observations
protégés. Le commanditaire doit fournir évaluateur et équipe tierce E3 ; ils sont
toujours indisponibles. Les autres sessions, missions suspendues, données et
processus sont préservés. Aucun commit/push, installation ou publication.
PA-00…13 et le RETEX restent ouverts.

## Contribution GLM et contrôles effectivement réalisés

Un appel SKYNET terminé `stop`, sans plafond client ni effort imposé :
11 736 jetons d’entrée, 12 666 de sortie, 24 402 total déclarés, 84,484 secondes.
Modèle API demandé/déclaré `glm-5.2`, sans attestation du backend. GLM a reçu les
producteurs complets, mais pas le texte intégral PA-01/03/09 : sa revue porte sur
le contrat fourni, sans verdict de couverture du programme ou d’indépendance.
Aucun défaut fonctionnel signalé sur ce code initial ; cet avis n’est pas un test.

Amélioration intégrée : mapping exhaustif des variantes de `State`, plutôt qu’un
bras `_`, pour forcer une revue à l’ajout futur d’un état. Documentation explicite
des refus possibles avant engagement. Tests précisés : sensibilité à la casse du
service, faux digeste d’observation pourtant valide en représentation, et clé de
transport différente compatible par construction. La phrase GLM « aucun alias de
transport ne peut passer » ne décrit pas ce contrat : `request_id` est volontairement
exclu et le test l’épingle. Champs privés de `CanonicalAction` ne signifient pas
contexte authentifié : seules les représentations sont garanties.

`rustfmt` stable a formaté et analysé syntaxiquement les deux nouveaux fichiers,
avec exit0, sans diagnostic. Commande et identité du formateur sont conservées.
Ce contrôle **n’est ni une compilation ni un test**, ne vérifie pas les types,
macros incluses ou effets. La syntaxe JSON de recette, les noms/compteurs des tests,
les liens de cette note, les empreintes et le diff sont contrôlés séparément.
Aucun processus d’effet, serveur ou test nécessitant compilation n’a été lancé.
Les originaux avant formatage et contexte exact de la revue restent conservés.
