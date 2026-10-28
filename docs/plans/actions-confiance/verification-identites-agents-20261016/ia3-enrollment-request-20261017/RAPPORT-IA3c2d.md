# IA3c.2d — demande complète liée au challenge, raccord courant ouvert

17 octobre 2026 — **PASS du cadre local et de la projection privée moteur.**
Aucun service protégé, admission publique ou droit d’exécuter n’est qualifié.

`internal/enrollmentrequest` lie la publication proposée entière à son challenge :
humain, référence/dates de session, projet/plan, révision Trust, événement,
instantané complet, racines exactes, checkpoints et document d’enrôlement attendu.
Le résultat est une donnée possédée immuable. La projection privée moteur réutilise
ses validateurs ; le caller conserve le TokenDigest produit par Trust.

## Décision et périmètre

Le [plan](../../identites-agents-replan.md) conserve le cadrage avant code.
Hacher quelques IDs ne couvre pas le contenu effectif ; un cadre versionné avec
les octets complets est retenu. Pas de copie du schéma métier des grants dans
le transport : le moteur le valide, le cadre lie ses octets. Les contrats
[FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md),
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) et
[cible](../../identites-agents-enrolement-IA3c2.md) décrivent les seize champs,
le digest avec domaine/version, les appels et les garanties manquantes.

Codex seul, même checkout, aucun autre chantier édité. Nouveaux fichiers dans
`internal/enrollmentrequest/` et deux fichiers de projection/tests moteur ;
seule source existante modifiée : `agent_identity_contract.go`, ajout et
remplissage du champ privé sessionDigest. Ni endpoint, service réel, compte,
clé durable, fournisseur externe, mission native, commit ou déploiement.

| Critère | Résultat | Preuve et limite |
| --- | --- | --- |
| Cadre normal | 62 noms PASS, 0 FAIL/SKIP | [normal-02](normal-02-run.json), collecte effective du paquet |
| Même cadre sous race | Mêmes 62 noms, aucune course signalée | [race-02](race-02-run.json) |
| Analyse/compilation du paquet | PASS | [vet-02](vet-02-run.json), [build-02](build-02-run.json) |
| Sources du lot local | 13 entrées avant/après stables | Paquet et dépendances, dont tests existants hashés mais non rejoués |
| Projection et consommateurs privés | 28 noms PASS, 0 FAIL/SKIP | [engine-normal-01](engine-normal-01-run.json), OIDC local signé et autorité de fixture |
| Analyse moteur | PASS | [engine-vet-01](engine-vet-01-run.json) |
| Compilation application | PASS | [application-build-01](application-build-01-run.json), fixture temporaire retirée, non déployée |
| Graphe de sources moteur/application | 1 080 fichiers avant/après stables pour chaque contrôle | [inventaire](engine-normal-01-inputs.json), Go/tests et ressources ; inclut conservativement des tests de dépendances |
| Sources précédentes | 16 inchangées, 1 modifiée comme prévu | [préservation](preservation.json), [diff caller](caller.diff) |
| Documentation et schémas | FR/EN mis à jour, 4 rendus, 2 images inspectées | [rendus](diagrams.json), [inspection](visual-inspection.json), index final |

Les nombres incluent parents et sous-cas, sans équivalence avec autant d’attaques.
Tous les tests ont `-timeout=0` ; aucun watchdog global arbitraire. Les sources
ne sont pas éditées pendant les contrôles. La race porte sur le nouveau paquet,
notamment ses copies concurrentes ; le moteur ne change pas de synchronisation.
Aucun test global du dépôt, test navigateur, revue indépendante ou résultat de
la version servie n’est revendiqué. Les anciens résultats restent liés à leurs
sources historiques ; les tests non relancés ne sont pas comptés dans ces lots.

## Vérifications observables

Chaque dimension résolue est modifiée séparément : principal/issuer/sujet,
session/observation/expiration, projet, work, génération/digest du plan, Trust,
enrôlement/génération/tête/document, événement, révision/contenu du snapshot,
racines/ordre et checkpoint précédent. Chaque modification valide change le
digest. Le snapshot est présent en entier et son empreinte doit correspondre.
Ce contrôle de transport ne vérifie pas sa révision interne ou ses grants :
ces règles restent au producteur métier moteur. Des JSON de snapshot inconnus,
dupliqués ou avec records invalides sont refusés par la projection moteur.

`Prepared` possède des octets immuables : modifier les entrées, exports ou tableaux
retournés ne modifie pas la demande ; ses copies Go sont utilisables concurremment.
La reconstruction JSON du handle ne le peuple pas ; reconstruire via Decode
reste possible et produit une donnée, sans preuve d’identité. Les enveloppes
non canoniques, null/omissions, champs inconnus/dupliqués imbriqués, variantes de
casse, types incompatibles et racines invalides/dupliquées sont refusés.

Le challenge lie le digest du cadre entier et le projet ; un vérificateur d’un
autre projet ou un autre digest est refusé. Le service futur doit recevoir le
cadre entier et vérifier Bind avant sa décision. Bind compare uniquement projet
et digest : la cryptographie et l’authentification du pair sont des obligations
séparées. La finalité est uniquement publication de registre, pas création,
validation humaine d’une action ou autorisation générique de workflow.

Le consommateur construit ses attendus depuis le même cadre, applique les dates
explicites de session et vérifie le document d’autorité exact sous empreinte.
Un autre document authentique pour le même ID/génération, avec des permissions
différentes, est refusé. L’instant d’observation est inclus ; expiration exclue ;
instant nul ou antérieur à l’observation refusé. Ce test ne démontre pas une
session actuellement active ou une horloge fiable : il ne relit pas Trust.

Le résolveur conserve désormais le TokenDigest qu’il vient de recevoir du
contexte Trust vérifié. La projection ne reçoit aucun autre token. Le test moteur
suit OIDC code/PKCE sur fournisseur local signé, résolution Trust puis projection :
empreinte exacte de ce token, dates, checkpoint, scope, snapshot et racines sont
observés. L’autorité d’enrôlement et son témoin restent explicitement des fixtures,
pas une identité externe approuvée par un opérateur.

## Anomalie de candidat et récupération

Le [contrôle préalable](request-red-01-run.json) échoue à compiler, l’API n’existant
pas encore : zéro test exécuté, aucune vulnérabilité produit ainsi reproduite.
Les 62 noms normal/race du premier candidat passent, mais
[vet-01](vet-01-run.json) refuse quatre littéraux externes sans noms de champs.
La [sortie originale](vet-01.stderr) est conservée. Correction : nommer les champs
des fixtures ; aucune validation affaiblie. Le source avant correction est
reconstruit par inversion exacte des quatre remplacements et comparé à son
empreinte originale avant archivage, sans prétendre qu’il fut capturé avant le run.
Les contrôles 02 passent, avec leurs propres empreintes avant/après.

Autre amélioration de conception, détectée avant l’écriture de la projection :
recevoir séparément caller et token permettrait d’associer par erreur une autre
référence de session. Le nouveau champ est pris directement du résultat Trust ;
le producteur n’a pas de second argument token. Il s’agit d’un risque de composition
prévenu, pas d’une faille d’endpoint existant reproduite puis corrigée.

## Limites, apprentissages et suite

La projection est pure : ses callers doivent résoudre les frames courantes et
les tenir sous la sérialisation adaptée. Elle ne consulte ni Trust ni SQLite,
ne prouve ni adoption du plan, ni actualité du checkpoint/enrôlement, ne valide
pas à elle seule les nouveaux plafonds sur l’historique et ne crée aucun permis.
Un cadre canonique reconstruit depuis une déclaration d’agent reste non authentifié.
Une date/session sous empreinte, signature ou génération exacte ne fournit pas
une non-révocation à l’usage. Les publics F01–F07, IDA, descendants et mémoire
restent ouverts. Les limites d’IA3c.2a/b/c restent applicables.

| Découverte / origine | Valeur et périmètre | Propriétaire / état / critère |
| --- | --- | --- |
| I2d-01, EP-02 | Couvrir tout le candidat et les deux checkpoints, pas seulement des IDs | Codex ; réalisé/vérifié localement, pas de source courante prouvée |
| I2d-02, composition caller/token | Conserver la référence issue de la résolution effective | Codex ; champ privé et projection sans token séparé, vérifiés en OIDC local |
| I2d-03, contrôle du corps signé | Distinguer même ID/génération et même document exact | Codex ; comparaison du digest réalisée, substitution de droits refusée |
| I2d-04, vet des fixtures | Garder les constructions de contrats lisibles et compatibles | Codex ; quatre champs nommés corrigés, original et reprise conservés |
| I2d-05, candidat complet | Préserver immutabilité sans plafond de taille caché | Codex ; copies possédées vérifiées ; coût croissant constaté, mesures et transport réel à définir avant limites opérateur |

Prochaine dépendance : choisir/implémenter la frontière de contrôle, le témoin
indépendant durable et le point de linéarisation source/usage ; versionner le
cadre de lecture avant son premier client externe (I2c-06). EP-02 reste partiel
jusqu’au producteur courant et au raccord final. Le choix opérateur de frontière
et du principal initial reste sans réponse ; aucune activation n’en est déduite.
F01–F07, IA3 public, IA4/IA5 et RETEX global restent ouverts. Les mécanismes
classiques de signature et de liaison par digest ne prouvent aucune nouveauté
scientifique ou supériorité comparative.
