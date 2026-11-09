# Reprises par lots courts — plan directeur Swarm

8 novembre 2026 · [English](../en/REPRISES-PAR-LOTS.md) · [File centrale](FILE-APEX.md)

Décision de Fabrice : découper PAEA, Biba, Bell-LaPadula, identités et mémoire,
utiliser les chats comme lots du même méga-plan. Facturation en dernier, plus tard.
Découpage documentaire proposé à partir des sources lues ; pas nouveau plan moteur,
autorisation de développement, session, branche, appel ou test. Les IDs PA/BT/BLP/IA/MEM
restent canoniques : les Cxx ci-dessous sont des cartes de reprise, pas exigences nouvelles.

## Contrat de chaque carte

Un propriétaire, besoin précis, entrées et candidat, périmètre fichiers/ressources,
témoin valide, variante/refus/reprise, résultat observable, preuve et limites.
Une carte finit par une remise bornée : réalisé/vérifié/installé/committé/poussé
séparés, RETEX court et reste attribué. Blocage : cause + preuve + propriétaire +
précondition modifiée ; continuer uniquement travaux indépendants. Pas de durée,
budget, délai de test ni plafond de tentatives inventés. Revoir une carte si trop
de contrats : un consommateur ou une frontière par sous-carte, pas une limite de temps.

Les dossiers historiques établissent un état daté. La première action de reprise
rapproche les acquis du code courant, avant de décider qu’une correction manque.
Développement PAEA exclusivement GLM sous autorité existante, Codex supervise ;
ne pas profiter d’une carte Biba/identité pour contourner cette restriction.
Aucun lot nouveau moteur derrière le push main bloqué ; analyses indépendantes seules
selon la file nocturne. Cette organisation n'arrête ni ne duplique le propriétaire actif.

## PAEA — PA00–13, G0–G7

| Carte | IDs existants | Résultat borné | Preuve de fin | Porte / limite |
| --- | --- | --- | --- | --- |
| PAEA-C0 | PA-00/09, G0 | Actualiser sources/candidat, pointeur CURRENT-GLM et blocages distincts | Inventaire relu ; aucune identité/runtime inférée du seul PID. | Pas nouvelle campagne. |
| PAEA-C1 | Précondition GLM | Rétablissement amont utile au lot conservé | Précondition réellement changée au-delà de models200 ; demandes opérateur déjà pendantes non répétées. | Bloqué selon dernier suivi ; pas nouvel appel inchangé. |
| PAEA-C2 | PA-07/09, G0/G5 | Garde24 et proposition PreparationRefused conservée | Proposition GLM revue, garde personnelle et intégrité ; refus témoin/reprise ; ne pas modifier les preuves antérieures. | Développement GLM uniquement ; garde24 non reçue personnellement selon suivi lu. |
| PAEA-C3 | PA-07/09 | SignalRefused et causes cleanup : une issue par attribution | Témoin valide, faute ciblée, cause jusqu’au consommateur, effet connu/inconnu puis reprise. | Pas regrouper tous les chemins dans une tâche illimitée. |
| PAEA-C4 | PA-02/08, G1 | Parcours humain d’approbation sur projet exact | Session réelle habilitée et demande/accord/consommation observés ; fixture HTTP déjà prouvée ne vaut pas humain. | Demande de connexion pendante ; ne pas la répéter. |
| PAEA-C5 | PA-03, G1 | Accord exact et incompatibilités de rôles au règlement | Rapprocher P1 et dernier tour interrompu ; témoin/refus/reprise et effet unique sur un périmètre. | Même propriétaire ; pas deuxième développement Codex derrière restriction GLM. |
| PAEA-C6 | G2–G7 / PA-04–13 | Programmes aval : cartes des domaines ci-dessous puis E2/E3 | Un parcours et un candidat par carte ; G6 protocole peut se préparer sans campagne, ni contact tiers. | E3 et RETEX global restent ouverts ; pas tous les lots en une session. |

## Biba — BT01–BT05

| Carte | IDs existants | Résultat borné | Preuve de fin | Porte / limite |
| --- | --- | --- | --- | --- |
| BIBA-C0 | BT01 | Inventaire et témoins historiques vs Rust courant | Réutiliser private-root-before ; identifier profil et chemins réellement couverts. | Ne pas refaire BT01 sans différence démontrée. |
| BIBA-C1 | BT02 | Un lanceur à la fois : agent, planning, reviewer, préparation | Témoin valide puis refus sans repli non confiné et reprise ; périmètre du helper nommé. | Quatre cartes filles si quatre écarts ; pas quatrième succès déduit du premier. |
| BIBA-C2 | BT03 | Un chemin secondaire/descendant réellement identifié | Aucun droit I3 récupéré depuis I0 ; état ressource et issue d’échec conservés. | Inventaire avant correction ; droits Linux inchangés. |
| BIBA-C3 | BT04 | Une promotion mémoire/import/effet à la fois | Auteur/labels et octets liés, ancien résultat/révocation/rejeu refusés avec témoin. | Dépend de BT02/03 ; coordination mémoire et Git. |
| BIBA-C4 | BT05 | Livraison et RETEX de la frontière qualifiée | Profil réel, Admin/API/CLI, FR/EN, reçus et revue applicable ; critères globaux ouverts. | Dette CLI/engine signalée par RETEX class2 à rapprocher, pas tenue pour encore présente sans reproduction. |

## Bell-LaPadula — BLP-01–07

| Carte | IDs existants | Résultat borné | Preuve de fin | Porte / limite |
| --- | --- | --- | --- | --- |
| BLP-C0 | BLP-01/07 | Inventaire des frontières et preuves actuelles | Correspondance frontière → producteur → consommateur → preuve/limite ; date et candidat. | La matrice source contient des mentions datées 9 novembre : dates déclarées, pas exécutions futures confirmées par cet inventaire. |
| BLP-C1 | BLP-03 | Une opération de configuration/migration restante | Preview sans effet, application révisionnée, refus concurrent, reprise ; ne pas refaire les parcours déclarés livrés. | Moniteur BLP-02 déjà vérifié historiquement : préserver. |
| BLP-C2 | BLP-04 | Un producteur/provenance ou usage mémoire nommé | Attribution établie au producteur puis contenu/labels jusqu’au consommateur ; pas champ client pour preuve. | Coordination identité et mémoire. |
| BLP-C3 | BLP-05 | Une frontière fichier/réseau/fournisseur/descendant | Témoin sain puis octets sensibles fictifs interdits non reçus, refus sans repli et reprise. | Quarantaine existante distincte d’un confinement universel. |
| BLP-C4 | BLP-06 | Une déclassification/copie à destinataire déterminé | Identité humaine, octets/destination exacts, décision indépendante applicable et nouvelle copie attribuée. | Pas de permission nouvelle par plan. |
| BLP-C5 | BLP-07 | Agrégat, revue et RETEX du même candidat | Réutiliser recettes éligibles, conserver exclusions et rouge ; acceptation indépendante distincte. | Pas clôture globale par agrégat partiel. |

## Identités — IA1–IA5

| Carte | IDs existants | Résultat borné | Preuve de fin | Porte / limite |
| --- | --- | --- | --- | --- |
| ID-C0 | IA1/IA2 | Inventaire des acquis et remise du tour interrompu | Sources/preuves/UUID et dernier résultat GLM conservés ; pas deuxième appel ou audit complet refait. | Historique session interrompu, résultat final non établi ici. |
| ID-C1 | IA2 | Un canal/contrat d’instance dans sa portée | Principal/instance/projet/tentative liés ; usurpation/rejeu/révocation refusés, reprise autorisée. | Ne pas refaire noyau privé vérifié. |
| ID-C2 | IA3 | Autorité humaine protégée avant usage | Demande légitime et décision exacte ; contrôle protégé, aucun accès permissif si prérequis absent. | Livrer la sortie IA3 nécessaire sans attendre toute IA5. |
| ID-C3 | IA3 + MEM-P2/P4 | Remise vérifiable au consommateur mémoire | Paquet d’autorité reçu et appliqué au bon contenu/destinataire ; refus/reprise. | Mémoire possède consommateur ; identité possède autorité. |
| ID-C4 | IA4 | Interface d’un droit de lecture/résumé/transmission | Même validation API/CLI/Admin, FR/EN et refus visible, preuve de réception. | Après sortie IA3 et usages M20 requis. |
| ID-C5 | IA5 | Qualification et RETEX du périmètre | Matrice IDA-01–18 : preuve ou ouvert explicite, persistance/concurrence et candidat précis. | Pas acceptation indépendante inventée. |

## Mémoire — MEM-P1–P5 / M20

| Carte | IDs existants | Résultat borné | Preuve de fin | Porte / limite |
| --- | --- | --- | --- | --- |
| MEM-C0 | M20, M29–M33, MEM-P1 | Raccrocher les incréments et audits dispersés | Index INCREMENT ↔ RESULTATS ↔ ID métier ; suites AUDIT-REPRISE visibles. | Voir INDEX-REPRISE ; numéro de fichier différent du numéro Mxx. |
| MEM-C1 | MEM-P1 | Une famille de lectures/replis/enveloppes encore inconnue | Inventaire Rust actuel ; témoin valide avant erreur, diagnostic et absence de succès vide puis reprise. | Anciens M33 Go vérifiés ne qualifient pas Rust ; pas reprise globale des 19 appels. |
| MEM-C2 | MEM-P2 / M20 | Propriété et auteur : stockage/migration bornés | Attribution authentifiée, anciennes données inconnues gardées inconnues, migration réversible. | Sortie protégée IA3 requise pour attribution réelle. |
| MEM-C3 | MEM-P3 / M20.2–3 | Un usage lecture, résumé ou transmission | Lier source exacte/demandeur/destinataire ; droits distincts et refus testable. | Réutiliser M24/M25 au lieu de les réimplémenter. |
| MEM-C4 | MEM-P4 / M20.4 | Transmission, retrait et reprise | Nouveau contexte refuse ancienne source révoquée ; contenu reçu auparavant préservé ; réception attribuée. | Alimente IA4, pas attente de toute MEM-P5. |
| MEM-C5 | MEM-P5 | Qualification et RETEX du périmètre | M20-U01–U10 avec preuve/limite, CLI/API/UI, revue applicable, traces exactes. | Ne pas appeler M20 terminé parce que M21/M33 passent. |

## Ordre et dépendances

Remise de l'actif → inventaire courant → carte d'autorité/accord → consommateur
des usages → composition → qualification/revue → RETEX du périmètre.
IA3 → MEM-P2/P3/P4 → IA4 ; ne pas créer un cycle IA5 entière ↔ MEM-P5 entière.
BT02/03 conditionnent BT04 ; BLP-03 → BLP-04 → BLP-05/06 selon plan canonique.
Préparation de preuve/protocole possible séparément des droits d'exécution.
La mémoire et l'identité sont coordonnées, pas fusionnées ; Biba et BLP ont
des propriétés et des oracles différents, pas une certification réciproque.

## Sources lues et portée

PAEA-APEX.md 808–835 et 1044–1069, GAP-CLOSURE-PLAN-20261103.md 1–100,
dernier suivi PAEA 8 novembre 22:25 UTC ; SECURITY-PROOFS-APEX.md 42–59 ;
BIBA-TRANSVERSE-APEX.md 15–44 ; BLP-APEX.md 93–99 ;
identites-agents-replan.md 177–181 ; deepagents-integration/PLAN.md 35–58 ;
INCREMENT-MEMOIRE-16 et 29, RESULTATS-MEMOIRE-29, AUDIT-PROVENANCE-M20 :
passages initiaux et reprises finales lus. Code et tests non exécutés ici.
États des sessions lus via outils officiels, pas titres seuls.

## Facturation — FIN, PLUS TARD

Projet distinct à forte étendue dans le suivi (950/1700, campagne principale non
commencée selon dernier message). Durée/coût restants non mesurés. Suivi automatique
mis PAUSED après report explicite ; aucune reprise ni attribution nocturne.
Préserver échec SQLite, candidat figé, sources/preuves et tentatives. Reprise dédiée
explicitement demandée seulement ; aucune clôture ni remise à zéro.
Le règlement fictif ciblé de PAEA n'est pas cette campagne de réplication complète.

## Vérification de cet incrément

Documentation/organisation seulement ; aucun code/fonction/politique exécutée ou
issue d'erreur produit modifiée. Pas build/test/install/version à annoncer.
Liens des nouveaux documents et titres vérifiés ; ni conformité runtime ni
garantie future d'absence de doublon. Cloud différé. Retraite hors projet Swarm.
