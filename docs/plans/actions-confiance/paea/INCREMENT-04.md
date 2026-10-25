# PAEA — incrément 04 : vérificateur de correspondance et correction des appels GLM

[English](../../../en/plans/actions-confiance/paea/INCREMENT-04.md) · [Plan](../PAEA-APEX.md)

## Résultat et périmètre

Suite autorisée « continue avec glm ». La consigne sans compilation demeure.
Plutôt que d’étendre seulement les essais du même binaire, Codex réalise un
consommateur utilisable des exports :
[verify_canonical.py](../../../../tools/paea/verify_canonical.py), avec
[contrat opératoire](../../../architecture/PAEA-CANONICAL-VERIFIER.md).
Le programme PA-00…13 reste ouvert ; cette première brique PA-01/PA-09 n’active
aucun projet ni effet. Aucun fichier Rust/Cargo ou service n’est modifié.

La [recette](evidence/20261025-glm-verifier-fadc9ff1/recipe-02/result.json) est
PASS et son intégrité VERIFIED : **neuf tests nommés**, 42 observations CLI
(14 exports Rust, 27 appels du vérificateur Python et une génération de vecteurs), 33 succès et neuf refus
attendus. Les doubles d’écriture et appels purs complètent ces observations,
sans être comptés comme invocations CLI. Le binaire existant garde son empreinte
`addcc5eb6b32dd7d2ed963a01a80062155ccd9613985fb518c079f5a461f794f`.
Ce candidat historique n’est pas le serveur installé.

| Critère | Preuve et limite |
|---|---|
| Correspondance des deux domaines | 14 sorties réelles du binaire comparées exactement aux vecteurs gelés avant vérification Python. |
| Substitution et ambiguïté | Digest, intention, texte canonique, autorité inventée, champs absents/inconnus, booléen-version, doublons, exposants, Unicode et second document refusés, avec témoins et reprises. |
| Identité des entrées | SHA256 des octets document/export réellement consommés ; aucun fichier d’entrée modifié dans les appels publics. |
| Contrat volontairement borné | Nouvelle clé de requête et ordre des labels équivalents ; couple forgé cohérent hors i64 accepté comme correspondance seulement. Cela interdit de confondre ce résultat avec une admission. |
| Erreurs IO et reprise | Fichier absent, répertoire, FIFO, JSON privé malformé et `/dev/full` réel ; écritures partielles/zéro progression/erreur testées avec doubles. |

Les [observations originales](evidence/20261025-glm-verifier-fadc9ff1/transcript-02.json)
conservent entrées, argv et sorties. Le lecteur refuse les fichiers non réguliers
sans bloquer sur une FIFO, écrit son rapport sans tampon Python et ne lit aucun
secret/politique/Root. Le schéma et l’oracle Python local sont des dépendances
communes déclarées, pas une frontière indépendante E3.

## Diagnostic et corrections vérifiées

L’[original avant correction](evidence/20261025-glm-verifier-fadc9ff1/verifier-before-io-fix.py)
et le [reproducteur](evidence/20261025-glm-verifier-fadc9ff1/io-reproduction.json)
conservent deux défauts de notre premier candidat : `fdopen` refusait le répertoire
avant la vérification du type, donnant un code IO générique ; la sortie pleine
produisait un diagnostic exit2 puis un reflush à l’arrêt Python changeait l’exit
réel en **120**, avec un second diagnostic hors JSON.

Correction générale : `fstat` avant transfert de propriété du descripteur et
fermeture de tous les chemins ; écriture complète non bufferisée sur stdout
standard, avec gestion de progression partielle et erreur. Les cas publics
réussissent maintenant selon les codes et la reprise attendus. Une sortie
partielle reste possible : exit2 ne prouve pas absence de publication.

GLM a proposé la vérification des écritures partielles et du consommateur stdout.
La proposition de détacher/rediriger globalement stdout pour un éventuel reflush
n’est pas intégrée : aucun contre-exemple démontré sur le chemin désormais sans
tampon. L’usage du descripteur stdout standard évite aussi de dépendre d’un wrapper
Python détaché. Le [registre de décisions](evidence/20261025-glm-verifier-fadc9ff1/glm-disposition.json)
conserve avis exploités, hypothèses et limites.

## Correction demandée par l’utilisateur : aucun plafond GLM arbitraire

Le principal avait imposé 4 096 jetons à une revue complète puis 2 048 à la reprise
ciblée. Ces valeurs n’avaient pas été demandées par l’utilisateur. Le premier appel
avait épuisé ses 4 096 jetons en raisonnement, sans réponse finale. L’utilisateur
corrige explicitement la méthode : GLM est on premise, ne pas imposer ce plafond.

Le lanceur personnel exigeait lui-même `--max-output-tokens`. La régression ajoutée
échoue avant correction : argparse exige cette option avant tout envoi HTTP.
Le [replan](evidence/20261025-glm-verifier-fadc9ff1/client-cap-replan.json) conserve
cause, propriétaire, ancien script et résultat observé. Le paramètre devient
facultatif : sans option, aucun `max_tokens` ou `max_completion_tokens` n’est envoyé.
Une valeur explicitement demandée doit rester positive. L’effort de raisonnement
n’est plus réduit par commodité. Aucun plafond ou proxy serveur n’est modifié.

Les **neuf tests du lanceur** passent sur un serveur HTTP local de fixture :
[sorties originales](evidence/20261025-glm-verifier-fadc9ff1/client-cap-tests.stderr),
[commande et résultat](evidence/20261025-glm-verifier-fadc9ff1/client-cap-tests.json).
Ils vérifient aussi refus sans envoi, reprise, secret expurgé, redirection refusée,
originaux préservés et absence de retry. Ces neuf tests ne sont pas des appels SKYNET.
La skill personnelle `skynet-agents` est actualisée avec la méthode `skill-creator`.
Les fichiers personnels et leurs originaux sont inventoriés dans la revue finale.
Cette transmission persistante ne modifie pas les processus déjà démarrés.

La requête de revue finale conservée ne contient effectivement aucun plafond
client ni `reasoning_effort`. Le serveur conserve ses limites réelles : absence
de plafond client ne signifie ni sortie infinie ni droit d’appels sans limite.
Les appels historiques demeurent inchangés. Les verdicts et usages exacts sont
conservés dans les dossiers `glm`, `glm-focused` et `glm-final`.

## Revue finale sans plafond et corrections associées

La [requête réelle](evidence/20261025-glm-verifier-fadc9ff1/glm-final/request.json)
omet bien les deux champs de plafond et l’effort de raisonnement. Le
[résultat](evidence/20261025-glm-verifier-fadc9ff1/glm-final/result.json) est complet :
5 824 jetons d’entrée, 33 773 de sortie (31 009 raisonnement, 2 764 texte),
39 597 au total, fin stop après 216,927 secondes. Cela démontre concrètement que
le plafond client 4 096 aurait été insuffisant pour cette réponse. Aucun coût
monétaire n’est déduit. Sur trois appels, usage total déclaré : 49 798 jetons.
Le modèle demandé/déclaré reste glm-5.2 ; la version réelle du backend n’est pas
attestée. La latence observée ne prouve pas un blocage ; le parcours HTTP actuel
n’expose pas de progression intermédiaire. Aucun watchdog n’est ajouté.

Deux observations de GLM sont reproduites avant correction : un schéma booléen
`true` acceptait un contexte vide rempli par les defaults de fixture, et un schéma
affaibli pouvait provoquer un KeyError non maîtrisé. Le profil du schéma est
maintenant vérifié, les erreurs de décodage du schéma sont distinguées de celles
du document, et les champs de contexte sont exigés par le consommateur. Les
rapports comprennent aussi l’identifiant et le SHA256 du schéma consommé ; cela
ne l’authentifie pas. Références externes et bases imbriquées sont refusées pour
préserver le fonctionnement hors réseau, même avec un candidat de schéma changé.

Le générateur autonome écrivait un cache bytecode du publieur, contrairement à sa
docstring. Le cas est reproduit dans une fixture minimale ; le flag anti-cache
est ajouté au générateur et sa sortie est rapprochée des 14 vecteurs gelés, sans
modifier les fixtures. Les préconditions POSIX/jsonschema>=4 et les limites de
ressources sont documentées. Les [contre-exemples](evidence/20261025-glm-verifier-fadc9ff1/schema-reproduction.json)
et le [cache observé](evidence/20261025-glm-verifier-fadc9ff1/cache-reproduction.json)
restent conservés, comme la première recette PASS à sept tests. La recette finale
ne réécrit pas cet historique. Aucune nouvelle revue GLM n’est lancée pour répéter
la même question après ces corrections ; le principal assume intégration et tests.

## Revue et suite

La revue liée aux octets est conservée dans `evidence/20261025-glm-verifier-fadc9ff1/source-review.json`.
Politiques : aucun nouveau choix opérateur Swarm ; schéma, représentation JSON,
fd stdout standard et compteurs de corpus sont techniques ou fixtures. Aucun
watchdog ni retry automatique. L’allocation GLM personnelle est facultative et
absente par défaut conformément à la correction utilisateur.

Fonctions : lecteur strict, unicité des noms, Unicode, lecteur détenu, writer,
comparaison, entrée CLI, helpers/tests et lanceur personnel ont leurs contrats
relus. Les erreurs remontent jusqu’au code retour et au consommateur ; les
entrées privées ne sont pas imprimées ; une erreur stderr secondaire conserve
exit2. La documentation FR/EN, recette, plan et file sont actualisés. Aucune note
de version runtime : outil dépôt et méthode personnelle, sans construction ni
installation du service. Les entrées déclarées de la recette sont stables ; cela
ne reconstitue pas un graphe de compilation ni une qualification globale.

PA-00/01 restent partiels et PA-09 seulement amorcé. Autorité courante, mandats,
approbations authentifiées, services d’effet, réseau et confinement restent à
raccorder aux producteurs IA3/AC-02 et à un candidat Rust qualifiable après levée
explicite de la consigne sans compilation. Codex porte l’intégration, les
propriétaires Identité/AC-02 leurs interfaces ; l’autre session est préservée.
Le commanditaire doit encore désigner évaluateur indépendant et équipe tierce.
PA-11…13 et RETEX global restent ouverts. Aucun avis interne ne remplace E3.
