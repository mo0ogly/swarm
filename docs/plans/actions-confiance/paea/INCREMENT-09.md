# PAEA — incrément 09 : reprise en boucle et vérification sans compilation

[English](../../../en/plans/actions-confiance/paea/INCREMENT-09.md) · [Plan](../PAEA-APEX.md)

## Décision et préconditions

Demande « next en boucle » ; Codex poursuit sans nouvelle relance, sans compilation
ni redémarrage. APEX/replan et verify-fix appliqués. Une autre session a construit
un candidat `v0.1.1-dev.75`. Sa copie figée propre est utilisée en CLI hors ligne,
jamais comme serveur. L’inventaire effectif observé correspond aux sources du
module `protection`, de la comparaison et du contrôle de reçu. Ni cette version
ni l’observation de son build ne constituent une installation par ce chantier.

Deux voies comparées : accumuler de nouvelles extensions non exécutées, ou
vérifier d’abord les incréments 05…08 sur ce candidat existant. La seconde donne
une preuve publique sans lever la consigne. Aucun appel fournisseur/agent.

## Replan — notre assertion d’erreur était inexacte

Le producteur CLI enveloppe son code dans `failure.code`, alors que deux
assertions Rust ajoutées en 08 lisaient `code` à la racine. Contre-exemple réel :
commande `check-receipt` sur deux chemins absents, exit2, stdout vide,
`failure.code=input-document-io`, aucun `code` racine. Original conservé dans
[failure-envelope.stderr](evidence/20261025-loop09-ys8pyhtn/failure-envelope.stderr).
Correction générale des deux assertions ; aucun échec de test Rust exécuté
n’est inventé. Leur exécution reste différée. La recette publique existante,
qui connaissait déjà cette enveloppe, est étendue plutôt que créer un outil
produit Python ou prétendre compiler les tests.

## Vérification préparée

Quatorze tests Python nommés : six cas antérieurs de canonicalisation/routage,
quatre de comparaison et quatre de reçu, via le binaire Rust figé. Rapports
complets, fichiers inchangés, refus/reprise, deux langues et domaines, UNKNOWN,
états terminaux et échecs d’écriture `/dev/full`. Chaque injection garde témoin
et reprise ; aucun Root réel, service, watchdog global ou retry automatique.
Le cas Rust de flush avec writer injecté reste non exécuté et distinct.

Sources, originaux, binaire, version observée, inventaire du build, trace de
reprise et résultat de recette :
[dossier de preuves](evidence/20261025-loop09-ys8pyhtn/).
L’état exécuté est conservé dans `recipe-01/result.json` après lancement ;
l’absence de ce fichier n’est pas un PASS. Le bilan final ci-dessous sera
actualisé après vérification de l’intégrité. Le protocole E3 et ses moyens
indépendants restent ouverts ; aucune approbation ou observation authentifiée.

## Résultat vérifié et deuxième passage de la boucle

La recette conservée a exécuté **14 tests réussis, 618 invocations**, dont 451
sorties0 et 167 refus attendus. Les 14 tests comprennent les six précédents et
huit nouveaux, pas 14 ajouts. Répartition : 248 invocations historiques de
canonicalisation/routage, 163 comparaisons, 207 contrôles de reçu. Trois refus
ENOSPC publics sont inclus. `recipe.py verify` confirme l’intégrité et le PASS
original. Le binaire n’a pas changé et aucun projet n’a été créé/modifié.
Ces résultats ne sont pas les tests Rust exécutés par Cargo et n’injectent pas
un échec du writer Rust au flush.

Le deuxième passage a vérifié la provenance : archive des **432 entrées effectives**
du build relue et chaque empreinte vérifiée ; huit entrées PAEA effectives
correspondent aux sources courantes. Le profil core normal n’a pas de feature
fixture et n’est pas un profil test. Notre premier contrôle supposait à tort
toutes les features de dépendance vides : Identité active `serde` via les manifests
enrôlement authority/config. Refus initial conservé, producteurs relus, puis
comparaison des features exactes (`protection=[]`, `identity=[serde]`). Aucun
changement de protection ou de binaire pour obtenir ce résultat. Voir
[provenance-verification.json](evidence/20261025-loop09-ys8pyhtn/provenance-verification.json).

Relecture des lots antérieurs : le vérificateur Python historique émet réellement
un `code` racine dans sa propre enveloppe ; ses assertions ne sont donc pas
changées par imitation du CLI. La correction concerne les deux assertions de 08.
Apprentissage : lire le sérialiseur consommé, au-delà du type d’erreur et de sa
méthode `code()`. Le contrat FR/EN et le helper de test transmettent maintenant
`failure.code` ; aucun ancien résultat n’est réécrit.

Les descriptions de deux schémas mélangeaient un contrat stable et un statut
temporel « non compilé ». Seules ces descriptions ont été corrigées ; leurs
propriétés/contraintes restent identiques, et ces fichiers ne sont pas des entrées
du build observé. La recette originale reste liée à ses anciens octets ; on réutilise
ses observations sur le même binaire/tests/code, avec revue séparée des métadonnées,
sans prétendre l’avoir rejouée. Aucun nouveau comportement produit livré, donc
pas de numéro/version ou note de livraison changé dans cet incrément tests/docs.

Une limite supplémentaire est vérifiée par deux appels conservés : deux reçus
ayant des digests d’observation différents peuvent produire le même rapport de
correspondance. Ce rapport ne revendique aucune authenticité. `PAEA-OBS-01`,
owner PA-09/Codex : lier un futur rapport autonome aux octets exacts de reçu/document,
ou imposer le dossier de preuves externe. La mitigation appliquée ici est le
transcript complet avec entrées, binaire et sorties. L’évolution de sortie est
différée au prochain contrat de compatibilité et build autorisés ; elle ne crée
aucun droit. Voir
[observation originale](evidence/20261025-loop09-ys8pyhtn/standalone-report-binding-observation.json).

## Réexamen des 17 invariants et reprise utile

**0/17 invariant démontré de bout en bout** ; six ont ici des observations
structurelles locales, onze n’ont pas de preuve de leur protection réelle.
Les campagnes distribuées T01…T22 ne sont pas remplacées par les 14 tests.

| Invariant | Preuve locale de cette boucle | Propriétaire et condition de reprise |
|---|---|---|
| I01 Non-autorité du modèle | PARTIEL : déclarations d’autorité rejetées, sortie non autorisante. | PA-02/06 : véritable modèle → contrôle protégé → usage refusé/admis. |
| I02 Identité/délégation | NOT TESTED : identifiants de fixture sans identité authentifiée. | PA-02/06, sortie IA3 protégée et délégation courante. |
| I03 Provenance/flux | NOT TESTED : aucune lecture/révocation/compaction protégée. | PA-05/06, raccord Mémoire avec versions et usages réels. |
| I04 Accord exact | PARTIEL : digests/contenus et liaisons de données contrôlés. | PA-02/03/08, approbateur courant et contenu consommé au service. |
| I05 Dernier usage | NOT TESTED : aucun engagement de service. | PA-02/03, autorité ordonnée au point d’engagement. |
| I06 Médiation | NOT TESTED : outil hors ligne sans confinement. | PA-04/05 et préconditions AC-02 ; inventorier et exercer les contournements. |
| I07 Secrets | NOT TESTED : aucun secret critique ou compte séparé. | PA-02/05, contrôle/clés hors domaine agent. |
| I08 Unicité métier | PARTIEL : identité stable sous clés de transport et contenus alternatifs. | PA-03, réservation durable et effet non dupliqué. |
| I09 UNKNOWN/reprise | PARTIEL : cycle syntaxique et refus des rejeux/relabellisations. | PA-03/09, réponse perdue puis observation durable et réconciliation. |
| I10 Supervision humaine | NOT TESTED : aucun approbateur/essai humain. | PA-08/12, parcours d’accord et protocole humain autorisé. |
| I11 Preuve indépendante | PARTIEL : binaire/originaux/empreintes conservés, sans observateur indépendant. | PA-09/12, ancres externes et évaluation indépendante. |
| I12 Défaillance sûre | PARTIEL : refus d’entrées/routage/sortie, aucune mutation de fixture. | PA-03/04/05, indisponibilité d’un prérequis pendant un vrai effet. |
| I13 Egress/confidentialité | NOT TESTED : aucune sortie réseau ou destination contrôlée. | PA-05/06, oracle réseau et politique effective. |
| I14 A2A | NOT TESTED : aucun pair/mandat/audience authentifié. | PA-06, remise et usage de droits vérifiés. |
| I15 Outillage | NOT TESTED : provenance de build ≠ admission et invalidation d’outil. | PA-07, substitution réelle sous contrôle protégé. |
| I16 Temps/ordre | NOT TESTED : aucune horloge protégée/epoch/bail. | PA-02/03/07, ordre d’engagement et reprise testés. |
| I17 Révocation au service | NOT TESTED : erratum conservé, aucun jeton consommé. | PA-02/03/12, révocation effective puis première consommation refusée. |

Suite concrète : Codex pourra lancer les targets Rust corrigés et Rustdoc après
levée de la consigne. L’intégration PA-02/Identité doit fournir identité et autorité
protégées avant les effets PA-03 ; AC-02 garde ses préconditions et n’est pas
démarré implicitement. Le commanditaire fournit les tiers E3. Aucun service
local redémarré/installé ici ; PA-00…13 et RETEX final restent ouverts.

La revue liée aux octets et les contrôles finaux sont dans
[source-review.json](evidence/20261025-loop09-ys8pyhtn/source-review.json).
