# IA2.3a — canal d'instance Go et raccord au vérificateur

17 octobre 2026. Codex seul, `replan` et APEX, auto-revue locale `code-reviewer`.
Checkout direct, branche `codex/clear-product-method-names`, HEAD
`4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, changements préexistants conservés.
**Composant interne vérifié, aucun lanceur ni serveur raccordé.**

## Besoin, choix et périmètre

Remplacer l'instance simulée du noyau IA2.2 par une attribution observée sur un
canal réel. Deux options : résoudre le PID des credentials par `/proc`, ou utiliser
le pidfd associé par le noyau au message et ouvrir ses namespaces par ioctl.
La première garde une course entre numéro et processus ; la seconde est retenue.
Cette course numérique est identifiée par inspection, pas exploitée dans une
preuve de réutilisation forcée du PID. L'émetteur disparu est effectivement testé.

Quatre nouveaux fichiers Go : `agent_identity_channel_linux.go`,
`agent_identity_runtime_linux.go` et leurs tests. Le noyau IA2.2, les lanceurs,
`Store`, Trust, le stockage et les sources mémoire préexistants ne sont pas
modifiés par cette tranche. Comparaison avec [l'état initial](baseline.json).
Les contrats [FR](../../../../architecture/AGENT-IDENTITY-CONTRACT.md) et
[EN](../../../../en/AGENT-IDENTITY-CONTRACT.md) décrivent le mécanisme présent.

## Mécanisme observé

Le contrôle de fixture lance deux programmes Python sous Bubblewrap avec porte
bloquée. Il reçoit PID hôte et références de namespaces du bootstrap, appelle
`pin`, vérifie les objets attendus et conserve leurs descripteurs avant de libérer
les programmes. Le namespace PID hôte, les références incompatibles et les
réutilisations de liaison/instance/namespace PID sont refusés. Aucune inscription
depuis le premier message. La table de liaisons reste dans le processus Go.

Le récepteur Unix `SOCK_SEQPACKET` exige `SO_PASSCRED` et `SO_PASSPIDFD`.
Chaque paquet apporte des credentials et un `SCM_PIDFD` ; les objets PID/montage
du processus sont obtenus par ioctl sur ce descripteur. Le numéro de PID et le
pair de connexion ne sont pas des sources de rattachement. La fermeture du
descripteur est protégée pendant les opérations système. Les métadonnées
incomplètes, troncatures et droits de fichier injectés sont refusés et fermés.
Tampon et deadline appartiennent au consommateur ; aucune nouvelle politique
opérateur cachée ou configuration Admin n'est introduite.

Réception hors Trust → Trust partagé → registre runtime → résolution des
namespaces/liaison/UID/révision → horloge fraîche → IA2.2. Révocation et fermeture
du registre sont sérialisées avec l'évaluation. L'évaluation ne réserve, ne
consomme et n'exécute aucun effet métier ; aucun verrou SQLite n'est ajouté.

[Schéma FR rendu](channel-fr.svg), [EN](channel-en.svg),
[commandes de rendu](diagrams.json). Les deux aperçus PNG ont été inspectés.

## Résultats et preuves

| Contrôle | Résultat / portée | Originaux |
| --- | --- | --- |
| Canal et runtime nouveaux | **19 noms PASS / 19**, cinq tests racine et quatorze sous-tests | [JSONL final](tests-final.jsonl) |
| Noyau IA2.2 et contrôles existants | **114 noms PASS / 114** | Même exécution, sélection ciblée conservée |
| Total ciblé | **133 noms PASS / 133**, 26 tests racine, aucun FAIL ou SKIP | [Commande/dates](tests-final-run.json), [résumé](tests-final-summary.json) |
| Concurrence du nouveau composant | **19 noms PASS / 19 sous `-race`**, aucun signalement de course | [Commande](race-final-run.json), [JSONL](race-final.jsonl), [résumé](race-final-summary.json) |
| Statique et compilation | `go vet ./internal/engine` et `go build ./cmd/swarm`, codes zéro | [Sorties](static-build-final.json), [binaire isolé non déployé](binary-final.json) |
| Entrées du candidat | **1 046 fichiers**, aucune modification pendant les vérifications | [Inventaire réel Go et ressources embarquées](inputs-final.json), `source-final/*.go.txt` |

Les 19 contrôles sous `-race` répètent les nouveaux cas : ne pas les ajouter aux
133 comme des scénarios distincts. Les 114 anciens contrôles et les 13 essais du
prototype Python précédent ne sont pas une campagne comparative scientifique.
L'[environnement](environment.json) est observé après les premiers essais, pas
présenté comme une capture préalable. Plateforme exécutée : Linux amd64,
noyau `6.17.0-41-generic`, Go 1.24.3, Bubblewrap et fournisseur scripté local.

Cas observés : A/B légitimes ; sous-processus d'outil attribué à A ; paquet d'un
outil déjà terminé refusé ; références de bootstrap incompatibles et réinscription
refusées ; vraie connexion B exportée puis utilisée par l'hôte (refus), par A
avec l'octroi B (refus) et par A avec l'octroi A (accord attribué à A). Le pair de
connexion est explicitement confirmé B. B fonctionne encore après ces transferts.
Table vide ou projet différent refusés ; révocation concurrente de A suivie de
refus, réactivation par réinscription refusée ; changement Trust et fermeture
distingués par leur motif ; namespace hôte refusé. Les deux programmes terminent
normalement, sans nettoyage forcé sur les exécutions réussies.

## Échecs, diagnostic et correction

| ID | Observation et cause | Action et état |
| --- | --- | --- |
| IA2-G01 | La première fixture casse après export du socket : Go active `O_NONBLOCK` sur la description partagée, mais le lecteur Python copié suppose un socket bloquant. Les sous-cas suivants échouent à la suite du même programme mort. | [Premier échec](runtime-initial.jsonl), [commande/empreintes](runtime-initial-run.json), sources `source-initial/`. Gestion du socket par les timeouts Python, compatible avec le mode non bloquant partagé ; aucun assouplissement du contrôle d'identité. [Passage après correction](runtime-after.jsonl), puis candidat final ; B encore fonctionnelle vérifiée. |
| IA2-K03 | L'horloge était échantillonnée sous Trust, avant l'attente du registre. Un octroi expire pendant cette attente mais la décision reste acceptée avec le temps ancien. | [Échec discriminant avant correction](clock-before.jsonl), [commande/empreintes](clock-before-run.json), sources `source-before-clock-fix/`. Échantillonnage après les attentes et les lectures de namespaces ; contrôle positif avec horloge réelle et refus après expiration, puis concurrence vérifiée. |
| IA2-G02 | La recherche des constantes avait omis les fichiers Go générés par architecture. Encodage ioctl supposé amd64, alors que certains Linux utilisent des bits `_IO` supplémentaires. | `unix.SO_PASSPIDFD` réutilisé ; bits d'encodage repris de `unix.NS_GET_NSTYPE` pour `_IO(0xff,3/5)`. Correction par inspection du contrat UAPI ; aucune exécution sur autre architecture revendiquée. |

La [première auto-revue](review-local-initial.md) demande ces corrections et
renforce les preuves du pair B et du motif de fermeture. Le checkpoint passant
avant l'ajout des assertions d'erreur sur l'inventaire des FDs reste dans
`tests-before-inventory-guard*`. Une erreur de lecture de cet inventaire n'est
plus assimilée à un résultat vide prouvant l'absence de fuite. Les preuves
initiales restent intactes ; les sources archivées sont en `.go.txt`.

L'instrumentation `-race` a nécessité sa compilation distincte ; le diagnostic
de processus a confirmé le passage compilation → test, sans relance inchangée.
Les lectures web de `fs/pidfs.c` ont échoué ; aucune conclusion n'est tirée de
ce contenu absent. Sources primaires consultées : [socket UAPI Linux v6.17](https://raw.githubusercontent.com/torvalds/linux/v6.17/include/uapi/asm-generic/socket.h),
[pidfd UAPI](https://raw.githubusercontent.com/torvalds/linux/v6.17/include/uapi/linux/pidfd.h)
et [credentials de processus/groupe de threads](https://raw.githubusercontent.com/torvalds/linux/v6.17/include/net/scm.h).
La preuve de fonctionnement des primitives vient des exécutions locales.

## Limites, apprentissage et suite

L'attribution de canal est réelle dans ces cages. **Octrois, instantané et cadre
d'autorité restent des fixtures.** Ce récepteur et son pont interne ne sont ni
un service d'admission publié, ni le producteur protégé durable. Le bootstrap
est conduit par le test ; les lanceurs réels ne l'utilisent pas. Pas de
consommation atomique, arrêt/réconciliation, reprise durable, approbation humaine,
fournisseur distant ou mesure de coût. Pas d'attestation de chaque thread ou
d'image binaire, ni garantie contre l'administrateur ou le compte contrôleur
hôte non confiné. Namespace imbriqué non enregistré : refus, pas dérogation.

Apprentissage réutilisable : échantillonner la fraîcheur après la dernière
attente de coordination ; distinguer identité du message et identité du pair ;
conserver le modèle d'appartenance et les objets épinglés ; traiter explicitement
les erreurs des sondes. Cette transmission documentaire ne prouve pas une
réparation autonome du moteur ou la réception de tous les futurs contextes.

Propriétaire Codex. Suite IA2.3b : source durable d'autorité et protocole
d'engagement Trust/SQLite, ancre courante indépendante de l'entrée, bootstrap,
ordre des verrous, réserve/départ/révocation/panne ; puis IA3 raccord de toutes
les activations et décision humaine exacte. Aucun raccourci par bearer partagé,
premier message ou rôle `admin`. IA2 entier, F01–F07, IA4/IA5 et le RETEX restent
ouverts ; aucun scénario IDA global n'est requalifié PASS.

Condition IA2.3b supplémentaire : le producteur possède un instantané immuable
ou coordonne sa lecture avec son écrivain. Le verrou runtime ne protège que les
liaisons, pas des maps empruntées au producteur. Propriétaire Codex ; réception
de cette contrainte à vérifier lors de la revue du protocole d'engagement.

Vérification finale et empreintes courantes : [index](checks-final.json).
