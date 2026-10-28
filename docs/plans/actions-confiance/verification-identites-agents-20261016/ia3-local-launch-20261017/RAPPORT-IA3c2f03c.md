# IA3c.2f-03c — remise privée avant contenu et fin du processus

17 octobre 2026. Codex seul, checkout canonique, branche
`codex/clear-product-method-names`, HEAD initial
`f32b9f8fa6e99f75845f57872a7f8a56a0ef9dc6`.
**Incrément interne vérifié en fixtures ; aucun service activé.** Pas de création
de compte, appel fournisseur, orchestration native, mission modifiée, commit,
push ou déploiement. Auto-inspection du candidat, sans revue indépendante.

## Besoin, alternatives et résultat

03b vérifie le processus qui répond et lie sa réponse à la demande exacte, après
envoi. Il ne prouve pas à lui seul que le canal a été remis au service avant cet
envoi. I2f03b-05 et EP-07 conservent cette précondition. Alternatives comparées
dans le plan : chemin de socket à établir, nouvelle couche réseau, ou remise
privée avec preuve de démarrage séparée. La dernière est retenue pour la cible
locale : réutilisation du canal Linux et des codecs existants, pas de nouvelle
racine de confiance ou politique de connexion implicite.

Deux ajouts seulement dans le code :
`internal/engine/agent_identity_config_launch_linux.go` et son test. Les 41
sources/modules de référence sont inchangés. L’image courante est épinglée par
le helper existant ; une commande Go neuve avec environnement explicite consomme
deux socketpairs privées SOCK_SEQPACKET configurées avant exec. Le pidfd enfant
est retourné atomiquement au clone, pas rouvert depuis un PID numérique. Le pidfd
du parent est transmis indépendamment au helper avec les canaux et l’image.

Sur le canal de démarrage V1 : hello, ready et release canoniques, challenge frais
32 octets, empreinte des pins publics. Chaque réception compare l’objet processus
noyau et l’UID à la source détenue indépendamment. Les deux côtés ont leur propre
source de pins ; le helper ne les apprend pas depuis le paquet. Le parent n’envoie
aucune demande métier avant ready exacte. Le helper n’admet le canal métier
qu’après release exacte. Le format et le domaine SHA-256 sont précisés dans les
contrats FR/EN. Ready/release n’accordent aucune autorité métier.

La demande métier reste V2, soumise au contrôle d’autorité indépendant et au
coordinateur protégés existants ; le stockage reste V1. Aucun reset/migration
d’historique. Le lanceur exige réponse valide et sortie enfant zéro. Il ferme les
copies héritées et recueille Wait sur les issues lancées ; annulation par fermeture
des canaux possédés et signal au seul pidfd enfant. Un numéro de pidfd fermé est
invalidé dans Cmd. Aucun signal de groupe, shell, timer ou retry automatique.

Un refus attribué reste un refus. Une sortie non nulle après réponse de mutation
valide conserve l’erreur originale et l’état durable, retourne zéro résultat et
ErrIndeterminate. Le retry métier exact avec autorité renouvelée ne duplique pas
l’historique. Un échec avant envoi métier ne prétend pas qu’un effet a été tenté.

## Preuves et critères

| Contrat observable | Résultat | Preuve et portée |
| --- | --- | --- |
| Nouveau lanceur, première version | 39 noms PASS normal/race-01, aucun FAIL/SKIP | `normal-launch-01-run.json`, `race-launch-01-run.json`, sources archivées ; contrat nouveau, aucune phase rouge inventée |
| Candidat final moteur | 117 noms PASS normal/race-03, aucun FAIL/SKIP | 77 contrôles précédents + 40 nouveaux ; `*-engine-03-run.json`, pas 117 attaques distinctes |
| Preview/create/read/apply/read | Réponse validée, preview sans sauvegarde, revision finale 2 et deux fichiers durables présents | Enfants réels de l’image de test, même UID ; source humaine et autorité de fixture |
| Avant demande métier | Pins/nonce/purpose/UID serveur ou parent incorrects, fin précoce refusés sans état durable ni issue de mutation inventée | Test `RejectsBeforeBusiness` et sous-tests ; aucun corps métier émis par le wrapper avant ready |
| Processus distinct, même UID | Message noyau attribué à ce processus refusé contre le pidfd indépendant d’un enfant | `StartupRejectsOtherProcessSameUID` ; objet enfant encore détenu après sa sortie, aucune recherche numérique |
| Ready sans droit | Refus Unauthorized, enfant à zéro, aucun état sauvegardé | `AuthorityAndExit/deny` ; readiness ne remplace pas withAuthority |
| Échec après réponse | Zéro résultat, ExitError et ErrIndeterminate, état durable conservé | `AuthorityAndExit/exit-after-reply` ; retry exact revision 1, octets config/témoin identiques |
| Annulation | Avant Start : aucun enfant ; en startup/autorité : enfant recueilli, pas d’effet revendiqué | `Cancellation` utilise marqueurs observés et pipe OS, pas de sommeil ; après tentative métier l’issue demeure incertaine |
| Commande et ressources | Entrées locales invalides refusées, erreur exec conservée, attribut système conservé, copies parent fermées, originaux enfant 3–6 fermés ou remplacés par FDs CLOEXEC, socket adopté CLOEXEC, commande démarrée non réutilisable | Test `RejectsInvalidCommand`, assertions de cycle ; environnement ambiant sentinelle absent de l’enfant |
| Codec démarrage | Versions/champs inconnus, doublons/casse/null/trailing, challenge et digest invalides refusés | `StartupCodec` ; buffer dérivé du plus long paquet de format, aucune limite métier cachée |
| Configuration/témoin | 106 noms normal/race-01 de 03b réutilisés | `config-reuse.json` : les douze entrées propres sont identiques ; pas de nouvelle exécution config nécessaire |
| Statique et binaire | vet/build-03 PASS, 834 entrées stables, aucun overlay | Binaire `/tmp/swarm-identity-config-launch-03c-fixture-03`, jamais installé ; empreinte dans checks-final |
| Méthodes et transmission | Contrôleur commun réel, cinq tests/sous-tests | `workflow-03-run.json` et sorties ; source/contexts embarqués inchangés, pas de processus déjà démarré mis à jour |
| Documentation et schémas | FR/EN, installation, cible et note scientifique mis à jour ; deux rendus inspectés | [FR](diagram-fr.png), [EN](diagram-en.png), `checks-final.json` ; aucune validation Admin/UX revendiquée |

Les noms incluent sous-tests et helper inactif sans environnement de fixture.
Les scénarios parents vérifient les vrais résultats des enfants. Ne pas cumuler
normal/race ou versions 01/02 comme des cas indépendants. Les sorties JSON et
stderr originales sont conservées, avec dates, commandes, codes de sortie et
hashes avant/après pour chaque lot. Aucun source testé édité pendant compilation
ou exécution. Les 834 entrées exactes couvrent internal, cmd et modules ; les
archives de référence et du candidat permettent de reconstruire les ajouts.

Le lot 02 comptait déjà 117 noms PASS normal/race avec vet/build. Le lot 03 suit
une seule modification du test enfant : contrôle des originaux hérités 3–6 avant
autorité métier. Si Go a réutilisé un numéro fermé, son remplaçant doit être
CLOEXEC ; ne pas confondre numéro recyclé et fuite du descripteur original. Cette
nouvelle assertion justifie le rejeu ciblé et son manifeste final ; aucun code
produit n’a changé entre ces deux lots. Les résultats 02 restent conservés.

Les watchdogs globaux sont explicitement désactivés : `-timeout=0` pour Go et
`-test.timeout=0` pour les enfants. Aucune borne fonctionnelle ou politique
opérateur modifiée. Les anciennes deadlines de socket IA2 de cinq secondes sont
inchangées et n’ont pas expiré. Les marqueurs déclenchent l’annulation seulement
après l’entrée observée dans la phase concernée. Pas de timeout utilisé comme
preuve de blocage ou de récupération. La compilation est suivie comme telle,
distincte de l’exécution des scénarios.

## Documentation, périmètre et recette

Contrats FR/EN, INSTALL FR/EN, cible IA3c.2, plan canonique et note scientifique
actualisés au même incrément. Un répertoire `docs/diagrams/` est apparu hors du
périmètre possédé pendant le travail ; il est préservé et exclu des preuves 03c.
Les schémas propres à ce rapport résident ici. Aucun fichier mémoire, méthode,
ancien composant d’identité ou processus tiers modifié par ce chantier.

README.md/README.en.md et le suivi/preuves deepagents-integration ont aussi
évolué hors périmètre à la vérification finale. Ces travaux restent préservés,
sans inclusion dans les diff/archives 03c. Les entrées Go des contrôles restent
identiques. Une phrase FR est rectifiée pour attribuer explicitement la réception
du pidfd enfant au parent, conformément au code et au schéma.

Depuis le checkout canonique, choisir un label neuf pour conserver les originaux :

```sh
python3 docs/plans/actions-confiance/verification-identites-agents-20261016/ia3-local-launch-20261017/run_checks.py recheck-01 engine
python3 tools/agent-workflows/check.py
git diff --check
```

Le script refuse d’écraser un lot existant, archive les deux sources de lancement,
vérifie la stabilité des entrées et impose des tests réellement collectés et
terminés. Il lance uniquement le sélecteur moteur concerné, puis vet/build ;
aucune suite globale, compte, mission ou fournisseur. Les cas utilisent des
répertoires temporaires isolés. Le manifeste config doit être revalidé avant de
réutiliser les résultats 03b. Une mutation d’entrée impose une nouvelle preuve.

## Apprentissage et reprise

La remise avant contenu, l’attribution du retour, l’autorité métier et la fin
du processus sont quatre critères distincts. Une preuve de l’un ne doit pas
clore les autres. Ce découpage et ses gardes sont transmis par les contrats,
le plan de reprise et les sources de test ; les contextes des processus déjà
démarrés restent à reconstruire lors de leur prochain lancement autorisé.

I2f03c-01/02/03 sont réalisés et vérifiés au périmètre de fixture : remise privée,
incertitude après sortie non nulle, fermeture/invalidation des copies de FDs.
I2f03c-04 reste une limite documentée : Wait collecte aussi les goroutines Go des
flux standard. Un writer arbitraire de l’appelant peut se bloquer même si l’enfant
est terminé. Le raccord public devra définir propriétaires et collecte des flux,
avec preuve observable ; ne pas remplacer cela par un délai de commodité.
Propriétaire : Codex pour le raccord public, opérateur pour ses flux/processus.
Deux solutions générales restent possibles : imposer des fichiers OS explicites
aux flux du raccord public, ou posséder un collecteur de copie annulable dont
fermeture et collecte sont vérifiées. Préférer les fichiers OS pour ce futur
helper à usage unique ; réserver le collecteur à un besoin de capture établi.
La décision est conservée, pas encore appliquée au raccord public inexistant.

La première sortie du contrôleur des méthodes confirme ses cinq noms, mais le
lanceur de preuve n’a pas conservé son code de sortie dans un record structuré.
Les originaux `workflow-check.stdout/.stderr` sont préservés. La reprise 02
utilise le collecteur commun qui conserve code, dates et entrées : correction de
notre méthode de preuve, sans attribuer une panne au produit ou répéter un test
par habitude. Aucun FAIL comportemental 03c n’a été observé ou caché.

Le destinataire est établi avant contenu **dans ce wrapper privé** ; cela ne
prouve ni confidentialité, protection ptrace/root, bootstrap non compromis,
compte séparé, humain authentifié, délégation, fournisseur réel, admission ou
révocation dans tous les nœuds d’un workflow. EP-07 progresse en fixtures et reste
partiel. F01–F07, IDA, EP-01/04/05/07, IA3–IA5 et RETEX global restent ouverts.
Les fixtures n’accordent aucune autorité nouvelle et ne valent pas acceptation.

Condition de raccord réel : l’opérateur désigne l’humain initial, la source qui
prouve son issuer/subject et approuve la racine/les pins, puis autorise le compte
et service séparés. Codex pourra raccorder cette source au helper, configurer
les chemins protégés et vérifier le confinement et les consommateurs publics
Go/API/CLI/Admin. Ne jamais déduire cet humain du compte actuel, d’un cookie
cockpit ou du premier appelant. Aucun service n’est activé pour contourner cette
condition. Le choix local dev/pilote et l’option distante future sont conservés.

Le helper à usage unique n’est pas un service permanent : un pidfd ne donne ni
changement de compte ni droit de signaler un UID différent. Comparer bootstrap
privilégié protégé et service/broker authentifié avant l’intégration publique,
selon la source d’autorité désignée ; le cockpit entier ne doit pas devenir
privilégié. Critère Codex/opérateur : lancement, supervision et collecte du compte
séparé avec permissions vérifiées dans le parcours retenu. Cette intégration
reste ouverte avec EP-07 ; les enfants du même compte ne la démontrent pas.

Le patch documentaire de cette précision a été refusé à cause d’un contexte de
ligne erroné ; les hashes prouvent qu’aucune modification partielle n’a eu lieu.
Contexte relu, patch corrigé puis preuves documentaires reconstruites ;
`doc-patch-incident.json` conserve l’erreur originale et sa résolution. Aucun
source Go n’a changé, les tests ne sont donc pas relancés pour cette correction.
