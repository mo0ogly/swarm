# Task result — plan-68b50cc82e-T2

Le pilote Biba simulé et sa configuration Admin, API et CLI sont prêts à être
qualifiés. Les critères locaux passent après réparation opérateur de la copie
arrêtée ; cela ne constitue pas une acceptation ni une production autonome.

## Identité et attribution

Mission `w-68b50cc82ec3fe55624f9549`, tentative
`a-b4669e24bbfb91f7a9686499`, producteur `auto-c8053557575a81bb6164`.
Le processus du producteur reste `completed` avec sa remise partielle historique ;
les deux tentatives consommées sont conservées. La réparation externe est réalisée
par l'opérateur Codex à la demande explicite de l'utilisateur. Les anciens rapports,
manifestes et résultats restent dans la remise Git historique et les originaux
`/tmp/swarm-actions-confiance-launch/original-t2/`.

## Causes corrigées et résultat observable

La recette crée désormais son stockage par le CLI public `init`. `show.ledger`
renvoie `[]` lorsqu'il est vide, sans modifier le stockage. Les plafonds métier
arbitraires sont remplacés par les bornes réelles de représentation, exposées
par `policy_bounds`, appliquées par Go et affichées dans Admin. L'aperçu conserve
la configuration ; l'application, le rechargement et la réouverture conservent
la nouvelle valeur. Les unités, zéro et séparation des rôles sont documentés.

La recette navigateur distingue précisément la panne réseau volontaire des
anomalies : URL et code attendu, erreur visible, aucune autre erreur admise.
Elle attend la fin du chargement avant une navigation et la fin des animations
avant la mesure de contraste. Les couleurs utilisent les jetons Swarm existants.
Le catalogue complet de cette page est local et généré par l'outil versionné ;
le traducteur partagé est fourni intégralement à la revue. Les deux dispatchers
publics sont dans le diff. Aucun contexte de revue n'est tronqué.

## Vérification locale opérateur

| Critère | Commande / observation | Résultat |
| --- | --- | --- |
| 1 Identité, mandat, approbation et Biba | `python3 tools/actions-confiance/verify.py T2 c1 --timeout-seconds 25` ; demandes exactes et refus sans effet indu | PASS, 2 cas publics |
| 2 Idempotence et reprise | même commande `c2` ; réponse perdue, redémarrage, réconciliation, un seul effet | PASS, 2 cas publics |
| 3 Preuve et configuration | même commande `c3` ; aperçu sans mutation, persistance, refus périmé, export/ancre/clé séparés et altération rejetée | PASS, 2 cas publics |
| 4 Parcours opérateur | même commande `c4` ; navigateur réel, FR/EN, deux thèmes, mobile sans débordement, focus, Échap, état vide/succès/erreur | PASS, 1 cas, 13 groupes, 20 combinaisons largeur/langue/thème de vérifications |
| Régressions Go | `go test -race ./internal/engine -run '^TestActionsConfianceT2' -count=1 -v` | PASS, 15 tests et 15 sous-cas effectivement exécutés |
| Statique | `go vet ./internal/engine`, catalogue généré et `npm run test:i18n`, `git diff --check` | PASS |

Les reçus, empreintes, environnement et mesures sont dans
`docs/plan-68b50cc82e-T2.operator-verification.json` et dans l'aire de preuve
opérateur. Le contraste mesuré des boutons dépasse 4,5 dans les deux thèmes.
Les captures corrigées ont été inspectées séparément des assertions automatisées.
Le premier résultat d'erreur réseau, les annulations de navigation et les captures
initiales peu lisibles sont conservés ; ils ne sont pas présentés comme un succès.

## Replan après revue indépendante

L'avis `review-97ee81b76a1096d0e7b7525e` reste `unknown` et conservé. Ses inspections
ont signalé trois défauts reproduits : délai positif ignoré, vérification de
preuve HTTP absente et JSON supplémentaire admis. Ils sont corrigés avec tests
avant/après ; la frontière exacte du délai et le refus sans mutation sont vérifiés.
La suspicion de reprise sans redémarrage est examinée : chaque opération était
un processus CLI distinct. La recette en conserve maintenant les PID et fins de
processus observés. La modale anglaise vérifie aussi ses champs, pas seulement le titre.

`package.json`, le générateur, le catalogue complet propre à la page et son
chargement sont joints. La page ne charge pas le catalogue global, resté égal
au candidat précédent. Les rapports et contrôles de `task_contracts_reports_controls`
sont des preuves fournies par le moteur ; leur redemande comme artefact de contexte
répéterait le même contenu. Un contrôle dont la sortie est encore marquée tronquée
reste une preuve partielle. Le moteur corrigé transmet maintenant toute capture
jusqu'à 64 Kio, et le nouveau candidat doit obtenir ses propres reçus complets.
Ce relevé ne prescrit aucun verdict et ne remplace aucune pièce demandée.

## Limites et prochaine action

Les effets sont simulés, les identités et la clé sont locales. Les refus Biba sont
prouvés sur l'API médiée, sans prétendre empêcher l'opérateur du système de modifier
un fichier par un autre chemin. L'administration locale utilise l'accès à l'espace
et la session web ; les identités d'entreprise sont T3. L'ancre externe, la purge,
les connecteurs réels et la collecte eBPF appartiennent aux lots suivants.

Remettre l'arbre examiné par `planning revise-recovered-result`. Le moteur conserve
l'attribution opérateur, exécute ses contrôles sur le candidat immuable et requiert
la revue indépendante avant acceptation. Puis poursuivre T3–T8 jusqu'à la recette
et au RETEX, sans supprimer de dépendance ni déclarer ces extensions réalisées.


La seconde réponse `review-18d40d0f807c447c34a3ff50` est une erreur de protocole
conservée : deux defects étaient associés à une pièce marquée inspected.
Ses hypothèses ont été reproduites séparément. L'approbation refuse maintenant
une ressource explicite contradictoire et ne peut faire régresser un effet
committed, unknown ou reconciled. Les variantes gardent l'action et l'effet unique.
La réparation externe suivante confirme cette erreur explicitement sans valider
la réponse ni lui attribuer un refus métier indépendant. Le guide README inchangé,
qui ne produit ni ne valide le contrôle, n'est plus dupliqué dans les sources
supplémentaires ; le vérificateur opérateur complet et package.json restent joints.


La troisième revue `review-0004c2c85aa9458a3a30711c` reste `unknown` et conservée.
Le badge métier est maintenant traduit sur une véritable demande publique en
français puis en anglais ; la prochaine action contrôlée par le moteur est
traduite sans toucher aux valeurs saisies. Le montant indique ses unités mineures.
Un bénéficiaire long reproduisait un débordement mobile : le retour à la ligne
est corrigé et observé à 390 et 320 px. Les contrastes et l'erreur réseau volontaire
figurent dans le résultat structuré du contrôle. Les originaux complets des
dispatchers et des styles partagés sont maintenant joints, sans modifier ces
sources inchangées. La capacité de réception et les contrôles restent vérifiés
par le moteur ; aucun résultat indépendant n'est anticipé.


La quatrième revue `review-0172834249b5ae9e231edb5f` reste une erreur conservée,
sans verdict final : le modèle a mal recopié une empreinte lors de la sélection.
Les deux hypothèses produit ont été reproduites puis corrigées. Le rejeu de
configuration conserve sa réponse JSON originale après des changements ultérieurs
et un redémarrage ; un entier supérieur à 2^53 reste exact. Une réponse historique
absente est refusée sans mutation. L'authentification précède le rejeu CLI/HTTP ;
un en-tête absent, Basic, vide ou mal formé est refusé, un Bearer valide reste admis.
Les 13 tests et 12 sous-cas passent avec race. Les recettes publiques sont rejouées
car l'authentification est une frontière commune à toutes les opérations métier.
Cette correction opérateur ne valide ni la sélection rejetée ni ses hypothèses
comme avis final ; des contrôles et un avis frais restent requis.


La citation invalide suivante est conservée sans transformation en verdict. Ses
hypothèses d'accessibilité ont été mesurées dans le navigateur : boutons à 38 px,
champs à 14 px, lien d'évitement absent. La page utilise maintenant le lien partagé,
transfère réellement le focus à main, garantit 44 px pour les contrôles et 16 px
pour la saisie. Les vingt combinaisons FR/EN, État/sombre et largeurs 1440, 1024,
760, 390, 320 passent sans débordement avec une valeur longue. La recette structurée
transmet les dimensions et les largeurs observées. Le contrôle delivery passe
avec son délai configuré de 15 secondes ; la définition opérateur reste inchangée.


La reprise finale du même avis fournit enfin un refus métier valide et conservé :
critères 1 et 2 validés, critères 3 et 4 à corriger. Les originaux sont désormais
conservés octet pour octet pour HTTP/CLI et canoniquement pour l'API Go interne.
Chaque admission, refus, approbation, engagement et réconciliation lie sa propre
entrée à la chaîne ; l'export version 2 les livre en base64. Altération, omission
et ajout non lié sont refusés. Les champs de justificatifs sensibles sont exclus
des commandes métier avant écriture. Les anciens brouillons incomplets restent
visibles et refusés, sans reconstruction. Les bornes de stockage existantes restent
applicables. Les 15 tests et 15 sous-cas passent avec race ; le contrôle public
validation exécute 30 cas Go nommés et les sept cas de recette.

La recette observe aussi le véritable état de chargement en retenant la première
réponse API avant de la libérer ; elle ne déduit plus ce contrôle du seul texte
statique. Le lien tactile et les champs sont mesurés dans les vingt combinaisons
FR/EN, thèmes et largeurs. Le nouveau composant et ses tests sont fournis entiers.
Ces corrections répondent au refus ; elles n'anticipent pas le nouvel avis.

## Qualification opérateur complémentaire

Les 19 tests Go principaux et 20 sous-cas passent avec race ; le parcours public
valide 39 cas nommés et sept cas de recette. Les 17 groupes navigateur couvrent
20 dispositions FR/EN, les deux thèmes et le montant 9007199254740993 sans arrondi.
C4 ajoute deux reçus npm réels en lecture seule. Les hypothèses de révocation,
borne finale et jetons d’exemple ont été reproduites puis corrigées ; la cible
associée à la case à cocher est observée par clic réel. Les anciens avis et les
échecs initiaux restent conservés ; ces contrôles locaux ne valent pas acceptation.

## Autorité et clé : correction opérateur complémentaire

Les hypothèses du dernier avis sont vérifiées séparément : le rejeu est lié à
l’autorité authentifiée, la clé de preuve est lue sans suivre de lien et la première
configuration ne peut adopter un fichier de clé sans attribution. Les anciens
rejeux sans autorité et une clé historique perdue restent des refus explicites,
sans fabrication de preuve. Le CLI observe le code partagé d’autorisation et le
refus d’une ancre symbolique sans mutation. La rotation d’un jeton pour une même
autorité reste compatible avec le rejeu exact.

Les 24 tests principaux et 34 sous-cas passent avec race ; la validation publique
vérifie 58 cas nommés et sept cas de recette. Les mesures navigateur complètes
sont transportées sans perte dans une table nommée, et les deux reçus npm partagent
un manifeste d’empreintes. Les 64 833 octets de validation restent sous la capture
de 64 Kio sans retrait de cas ni modification du vérificateur opérateur. Un arrêt
prématuré du conducteur a été réconcilié publiquement ; l’ancien avis, sa sélection
et ses dépenses restent distincts de la qualification de ce nouvel arbre.

## Originaux des contrats communs utilisés par le pilote

| Original candidat complet déclaré | Contrat utilisé dans le chemin T2 |
| --- | --- |
| `internal/engine/main.go` | `readInput` borne le CLI à 16 Mio ; `strict` refuse champs inconnus et documents supplémentaires ; `printJSON` restitue la réponse JSON. Ces définitions figurent dans le fichier complet, hors du petit diff de répartition `trust`. |
| `internal/engine/model.go` | `atomicWrite` écrit, synchronise et ferme le fichier temporaire puis le renomme ; les helpers de chemin et de nom restent dans cet original. |
| `internal/engine/store.go` | `openStoreWithMigration` impose un répertoire `.swarm` local, crée ses répertoires privés et ouvre le stockage requis par les parcours publics. |

Ces deux derniers originaux étaient réellement absents de la sélection déclarée.
Le fichier complet main.go était déjà dans l’inventaire mais son diff seul avait
été sélectionné. Les déclarations par critère rendent désormais ces dépendances
explicites ; les citations du rapport ne remplacent pas leurs originaux lus au
même commit par le moteur. Le code métier et les contrôles restent identiques à
la qualification locale précédente : 24 tests et 34 sous-cas, 58 cas Go publics.
La nouvelle remise exige sa propre qualification native et un avis frais.

Le lot T1 décrit l’architecture et les frontières existantes/cibles. Les helpers
de règlement d’agents appartiennent à cette base antérieure ; le nouveau chemin
`trust` retourne depuis le répartiteur dédié et utilise `withTrustLock`. L’examen
des interactions et des régressions sur tous les critères reste obligatoire ;
aucun résultat T1 antérieur ne suffit à accepter ce candidat T2.

## Correction du refus indépendant courant : audit et aperçu

L’avis natif `review-4ce246e4f3db9ce3d191d8fe` est conservé : critères 1/2
passent, 3/4 échouent. Le défaut du lecteur de reçu a été corrigé dans le moteur
contrôleur, séparément du candidat, puis un nouvel avis final natif a confirmé
ces deux défauts. La présente réparation ne modifie aucun avis antérieur.

Les erreurs de validation métier authentifiées sont maintenant auditées par la
transaction commune avec leur original et leur autorité. L’erreur garde son code,
le rejeu après redémarrage conserve le même refus sans ajout d’effet ou de preuve,
et une autre autorité ne peut pas le rejouer. Quatre variantes renforcent la garde
existante de conservation des originaux : ressource, date de début, expiration,
mandat déjà présent. Le producteur commun `commandFailure` est fourni entier dans
`internal/engine/cockpit_service.go`, sans recopier son contrat d’erreur.

Le navigateur attribue chaque aperçu à une génération du formulaire. Saisie,
fermeture et nouvelle demande empêchent une ancienne réponse de devenir applicable
ou de remplacer un aperçu plus récent. Le bouton bloque les doubles soumissions
de la même génération ; les erreurs anciennes ne reviennent pas dans une nouvelle
modale. Les trois variantes sont exercées sur des requêtes HTTP réellement retenues.

Qualification de cette réparation : 24 tests principaux et 34 sous-cas Go avec
race, sept cas de validation publique et 58 cas Go nommés ; sortie intégrale de
64 943 octets, sans troncature. La remise passe son délai déclaré de 15 secondes.
Le navigateur vérifie 18 groupes, vingt dispositions FR/EN/deux thèmes, aucune
erreur inattendue et l’erreur réseau volontaire attribuée. Le CLI public observe
le compteur de preuves 0→1, les codes 2/2/4 (refus/rejeu/autre autorité), les octets
originaux exacts et l’absence de mutation métier. Les contrôles moteur frais et
la nouvelle revue indépendante du nouvel arbre restent requis.

## Garde de portabilité ciblée

Le soupçon 32 bits de l’inspection suivante était mal attribué au fichier CLI ;
la citation réelle se trouve dans le test opérateur. La compilation `GOARCH=386`
a néanmoins reproduit trois débordements de constantes, dont deux dans des tests
préexistants du délai de revue et des requêtes web. Les fixtures utilisent désormais
des valeurs JSON de type int64 : sur 32 bits le décodeur refuse un entier qui ne
tient pas, sur 64 bits la validation refuse la durée hors borne. Aucun plafond
métier n’est ajouté ni modifié. Les trois tests ciblés sont réellement exécutés
en 32 bits, et la garde Biba modifiée passe aussi avec race en 64 bits. La preuve
originale complète et les empreintes des trois fichiers sont dans
`docs/plan-68b50cc82e-T2.portability.json`. Ce contrôle ne prétend pas exécuter
toute la suite du moteur sur toutes les architectures.
