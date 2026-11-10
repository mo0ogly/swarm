# IA3c.2a — réponse authentifiée, source courante non raccordée

17 octobre 2026 — **PASS du protocole isolé ; enrôlement réel et admission ouverts.**

Le nouveau paquet `internal/enrollmentproof` authentifie une réponse sous clé
publique épinglée, liée au projet, à la demande complète et à un challenge neuf.
Il empêche sa consommation locale multiple, y compris par copies Go du handle.
Il ne décide d’aucun droit et n’est importé ni par le moteur ni par le CLI.

## Besoin et décision

Après le reçu durable IA3c.1, IA3b-I04 reste partiel : une référence privée
d’enrôlement ne constitue pas une source courante. Réutiliser les clés de preuve
Trust reste circulaire avec une isolation/configuration non établie. Le format
à racine épinglée distincte est retenu comme préparation commune aux deux
frontières possibles : compte système distinct local ou autorité séparée.
Le choix est demandé à l’utilisateur ; aucun déploiement n’est activé.

Le [contrat cible IA3c.2](../../identites-agents-enrolement-IA3c2.md) décrit
les cadres exacts, octets signés, garanties locales, prérequis et décisions.
Le module reçoit sa racine depuis le contrôle ; il n’adopte aucune clé de réponse
ou d’import. Signature Ed25519, SHA-256, nonce 256 bits et domaine/version fixes
sont des éléments du format cryptographique, pas des plafonds d’exécution.
Aucun TTL, timeout, budget, réglage Admin ou permission nouvelle.

## Ce qui est vérifié

| Contrôle | Résultat |
| --- | --- |
| Lot final normal | 35 noms PASS, 0 FAIL, 0 SKIP |
| Même lot final sous `-race` | 35 noms PASS, 0 FAIL, 0 SKIP |
| Watchdog global des tests | Désactivé explicitement (`-timeout=0`) |
| `go vet` et compilation du paquet | PASS |
| Entrées du paquet/module | 4 identiques dans les quatre contrôles finaux |
| Copies concurrentes du challenge | Exactement un succès pour 16 appels, autres refusés |
| Schémas FR/EN | 4 rendus SVG/PNG réussis ; 2 images inspectées |
| Sources précédentes d’IA3c.1 | 6 fichiers Go inchangés ; 117 contrôles antérieurs non rejoués |
| Import moteur/CLI du nouveau paquet | Aucun ; absence de raccord vérifiée |

Les nombres incluent parents et sous-cas, sans équivalence avec un nombre
d’attaques distinctes. Les clés et énoncés sont des fixtures locales. Pas de
fournisseur réel, compte humain externe, service isolé, revue indépendante,
mission native, déploiement ou acceptation globale.

Les cas couvrent contenu exact et copies possédées, clé épinglée, contexte
projet/demande/nonce, signature capturée avec empreinte réécrite, autre clé et
autre domaine, schéma strict, doublons/casse/champs inconnus/trailing/types,
base64url canonique, réponse malformée pourtant signée, source d’entropie en
échec, rejeu concurrent et copie de valeur. Une erreur ne consomme pas le
challenge ; une réponse valide reste possible ensuite.

## Deux contre-exemples du candidat, corrigés et conservés

1. **JSON null normalisé en chaîne vide.** Le décodeur initial faisait
   `Decode(&string)` ; null était accepté comme vide et conservait la signature
   d’un payload vide. [Échec original](null-before-run.json),
   [trace](null-before.jsonl). Correction : exiger un token de type chaîne.
   `TestResponseNullIsNotAString` passe dans les lots finaux. Limite originale :
   les empreintes de cette première tentative sont capturées après exécution,
   avant correction ; il n’existe pas de manifeste avant/après pour elle.
2. **Copie Go du challenge.** Le handle contenait son mutex et booléen ; une
   copie faite avant consommation permettait deux succès. [Échec stable](copy-before-01-run.json),
   [trace](copy-before-01.jsonl), quatre entrées inchangées. Correction générale :
   état partagé derrière pointeur privé, sans mutex copié. Le test concurrent
   utilise aussi des copies de handles. `TestChallengeValueCopyCannotDuplicateConsumption`
   et le refus de reconstruction JSON passent dans les lots finaux.

Les sources originales et étapes intermédiaires restent dans ce dossier.
Les lots 01 (31 noms) et 02 (34 noms) étaient verts ; la relecture a ajouté les
variantes pertinentes sans les réétiqueter finaux. Un premier patch de contexte
erroné a été refusé sans mutation ; lecture des lignes réelles, deux patches
exacts puis nouveaux contrôles 03. Aucun source modifié pendant compilation/tests.

## Une limite rendue explicite par un test

`TestAuthenticityDoesNotAttestCurrentEnrollment` vérifie qu’une réponse « active »
signée avant une révocation pourrait encore être authentique après celle-ci.
Ce test documente une **limite**, pas un PASS de fraîcheur ou de révocation.
Il n’exécute aucune révocation dans un service réel ; l’API ne consulte aucun
état courant. Aucune livraison ou application distante n’est observée.
Le service doit encore fournir une source courante et le consommateur sérialiser
son usage avec la révocation. Un simple TTL arbitraire ne résout pas cette course.
La lecture du payload reste opaque, sans schéma d’enrôlement ou de permissions.
Un payload vide authentique ne donne aucune autorité. La signature ne chiffre rien.

L’anti-rejeu est local au challenge en mémoire, pas une réservation durable,
un effet métier unique ou un droit d’exécution. Après redémarrage, nouveau
challenge ; aucune reconstruction d’autorité par JSON/import. Les champs privés
et clés de fixture ne constituent pas une frontière OS contre le même UID.

## Sources, recettes et documents

- [Sources finales et empreintes](source-final.json), archives avant correction et finales.
- [Normal final](normal-03-run.json), [race final](race-03-run.json),
  [vet](vet-03-run.json), [compilation](build-03-run.json).
- [Recette](run_checks.py) : identifiants distincts et refus d’écraser une tentative.
- [Isolation du consommateur](consumer-isolation.json) : aucun raccord moteur/CLI,
  sources IA3c.1 conservées ; aucun effet prétendu sur le binaire qui sert l’application.
- [Schéma FR](proof-fr.svg), [schéma EN](proof-en.svg), [commandes de rendu](diagrams.json).

Les sources, contrat FR/EN, contrat IA2, plan et notes de recherche sont actualisés
ensemble. Le nouveau paquet n’importe pas le paquet de ressources web/méthodes :
ces tests isolés ne nécessitent pas la vue figée du chantier mémoire/web.
Cela ne résout pas le couplage historique des tests moteur (IA3b-I07).
Le contrat embarqué et les consignes personnelles ne sont pas modifiés ici.

## Reprise et RETEX local

Le besoin d’enrôlement protégé reste ouvert, propriétaire Codex avec l’opérateur
pour principal initial et frontière de contrôle. Avant raccord : schéma métier
strict et demande moteur complète, source durable courante, administration
protégée, révocation/usage linéarisés, témoin plan/registre indépendant, confinement
et réception/décision/application/effet vérifiés. Le choix de déploiement est
encore attendu ; aucun compte système ni accès distant n’a été créé.
F01–F07, IDA, IA4/IA5, mémoire et RETEX global restent ouverts.

Apprentissages vérifiés : un décodage Go permissif peut transformer un type JSON
invalide en contenu signé vide ; une capacité à état unique ne doit pas dupliquer
son état par copie de valeur. Tests négatifs et relecture ont chacun découvert
un défaut ; aucun test vert ne prouve à lui seul toutes les propriétés d’un
protocole. L’origine authentifiée d’un énoncé reste distincte de la validité
courante d’une autorité. Le mécanisme signature/challenge est classique ; aucune
priorité scientifique ni supériorité comparative n’est établie.
