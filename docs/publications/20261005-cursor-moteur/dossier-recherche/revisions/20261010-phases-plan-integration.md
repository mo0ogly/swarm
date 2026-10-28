# Acceptation gérée incompatible avec les phases du plan — 10 octobre 2026

Cette observation appartient à la mission Cursor `w-ca937edf4e0abd85f8d3d67b`,
hors campagne contrôlée. Elle prolonge le [défaut de transmission des preuves](20261010-correction-globale-preuves.md).
Le résultat T1 avait passé une revue indépendante fraîche, mais T2 ne démarrait
toujours pas. À la révision publique 39, T1 était `accepted` et la mission
affichait zéro résultat validé sur sept. Le responsable avait reconnu la
dépendance périmée ; aucune décision du modèle ne pouvait réparer la gate.

## Cause établie et attribution

Le plan persisté distinguait `plan-entry: entry`, `plan-validation: validation`,
`plan-delivery: delivery` et quatre `plan-criterion-N: validation`. Le moteur
d’intégration écrivait pourtant `gate: delivery` pour chaque contrôle. L’évaluation
mécanique était verte, mais `requiredPlanChecks`, appelée par `validGate` et
`acceptedFresh`, rejetait cette preuve. La tâche dépendante restait protégée.

La faute se situe dans la génération du moteur, pas dans le résultat du worker
ni dans la décision sémantique du vérificateur. Une revue favorable ne répare pas
une preuve structurée contraire au plan. Les fixtures gérées usuelles ne
possédaient pas de `PlanChecks`, ce qui explique l’absence de cette régression
dans leurs scénarios, sans garantir l’absence d’autres défauts.

## Choix de réparation et contrôles

Deux voies étaient possibles : modifier directement la gate conservée, ou retirer
la validité courante et requalifier publiquement le résultat. La seconde est
retenue pour conserver les preuves historiques et produire des contrôles et un
avis liés à une nouvelle clé de preuve. Aucun état `accepted` n’est forcé.

La correction commune reprend les phases persistées et vérifie les contrôles du
plan avant publication. `planning requalify` est étendue au cas d’une gate
incompatible avec le plan malgré un avis `passed`, avec confirmation et identité
exactes de cet avis. Les protections et plafonds existants restent en place.
L’[architecture et le diagramme](../../../../architecture/MANAGED-PLAN-GATES.md)
décrivent cette frontière et la reprise.

Le test rouge `TestManagedImportedPlanPhasesProduceFreshAcceptance` reproduit
`accepted` sans fraîcheur. `TestRequalifyExplicitManagedPlanPhaseMismatch`
reproduit le refus de la reprise du cas affecté. Après correction, ces deux
scénarios passent, ainsi que les régressions historiques et de publication.

| Vérification | Résultat | Portée |
| --- | --- | --- |
| Régression ciblée et requalification historique | PASS, 15,865 s | Racines temporaires, revue déterministe |
| Régressions gérées, sources, contextes et reprise | PASS, 40,393 s | Paquet moteur, sélection pertinente |
| Détecteur de courses sur phases et requalification | PASS, 15,765 s | Fixtures temporaires |
| `go vet ./...` | PASS | Analyse statique |
| Contrat commun des 12 méthodes | PASS | Configuration des workflows |
| Diagramme Mermaid | PASS | Analyse et rendu SVG, aucune erreur JavaScript |

Ces vérifications ne constituent pas une mesure de qualité de la revue réelle
ni une démonstration de conformité globale au contrat JAN/FEB. La suite complète
et l’observation de reprise réelle doivent conserver leur résultat propre.

## Installation et intervention explicite

Le binaire installé est construit depuis `1169df70bec73d50fd23f640245b8ffd1317b9de`
avec modifications, le 10 octobre à `18:33:41Z`, SHA256
`ecd2256d04cf71c6c23a50ef88db6c5f26604fdd52e94c4d89dadf6953f86bf0`.
Le diagnostic public confirme l’identité du CLI et du serveur actif. Le serveur
a été redémarré après vérification de l’absence d’agent, de revue et de
réservation de responsable actifs.

La demande publique `plan-phase-requalification-2ec9419434be4ef0b16332ddc116881a`
retire la validité T1 à la révision 40, conserve ses deux tentatives et archive
l’avis `review-bc4a136c4713357aed3c56b9`. Le conducteur reprend ensuite le
résultat `20c2961aecf9bd9332a01e3abdd52c38a1244bec` sans nouvel exécutant.
La revue `review-536346277798553f58126b80` démarre à `18:35:25.870508221Z`,
révision 41 ; le compteur passe de 4 à 5 sur le plafond inchangé de 30 appels.
Il s’agit d’une intervention explicite, suivie d’un règlement par le moteur.

La [trace de reprise](20261010-phases-plan-observation.json) confirme ensuite T1
acceptée avec phases correctes, reçus historiques inchangés et T2 effectivement
en cours avec APEX, après reprise publique de sa préparation conservée. La
[suite Go a fait l’objet d’un audit distinct](20261010-audit-suite-go.md) : le
délai de cinq minutes concernait le paquet entier, pas la durée du dernier test.
Cet ajout de constat est postérieur au patch de reprise figé transmis à T5 ;
il ne change pas son empreinte ni les événements déjà enregistrés.
