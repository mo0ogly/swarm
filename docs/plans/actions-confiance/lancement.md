# Lancement Actions de confiance

Le programme couvre 14 stories et R01–R20 dans huit lots dépendants : architecture ; pilote Biba ; identités ; données et secrets ; processus métier ; preuves durables, simulation, connecteurs et référentiels ; eBPF ; recette et RETEX.

## Reprise du produit à partir du code existant

Le code du pilote est repris dans le projet principal. La recette publique relancée sur ces sources le 15 octobre passe
ses sept scénarios, dont les opérations métier, les refus, la réconciliation et
l’interface FR/EN. Le [bilan fonctionnel](reprise-a-zero.md) est le point de départ
courant. Les extensions d’entreprise, données, processus, preuves avancées,
connecteurs, eBPF puis la recette globale et le RETEX restent à livrer.

L’ancien lancement est suspendu en révision468 pendant cette reprise, sans
supprimer sources, preuves ou consommations. Les relevés techniques suivants
restent historiques ; ils ne signifient pas que le code du pilote est absent.

## Relevé historique — révision 464

**Un lot accepté sur huit ; Swarm reste non qualifié pour diffusion.**
La correction des mandats périmés passe les huit contrôles T2 et sept T1 sur
`b7179c5756bd56de2c5ff17c7b7ea0aa5239ac01`. R21 trouve ensuite un défaut
distinct de reprise avant authentification. La pause459 conserve 90 appels et
l’avis fermé sans acceptation. Le contre-exemple est reproduit avant correction.

La reprise est désormais réservée au rejeu opérateur de la configuration initiale
exacte. Métier, aperçu et lecture refusent sans terminer la publication ; une
configuration différente échoue avant tout effet. Les 126 cas Go avec race
(33 principaux, 93 sous-cas) et sept recettes publiques passent. Le CLI observe
les fichiers intacts après refus puis la reprise des mêmes octets d’état et clé.
La nouvelle remise de l’arbre `08ede6843a08ffec564a4c009da7161478f60f0b`
enregistre le résultat `bdb4d220a97220fd47f51787d9bb0a5e5b856706` ;
ses contrôles natifs sont en cours et sa nouvelle revue reste à vérifier.

Le quota public 460 conserve 90 appels et autorise 94 pour quatre appels frais.
Les 32 originaux et rapport totalisent 398126 octets, d’où une limite 409600 sans
suppression de preuve. Fenêtre 400000 et autres politiques inchangées. Le même
conducteur unique PID 423881 et le binaire 560a1961 restent actifs et qualifiés.
Voir [le replan](verification-launch-20261014/native-preauth-replan.md).
Le programme continue après acceptation T2 vers T3–T8 et le RETEX.

## Relevé historique — révision 453

État observé au relevé453

**Swarm n’est pas qualifié pour diffusion. Un lot est accepté sur huit.**
T2 est en réparation et T3–T8 restent à réaliser. Le programme demeure ouvert
jusqu’à la recette complète et au RETEX.

Le résultat T2 antérieur est `186518d36c2346d4e9fd8f3e169c543153171e4a`,
arbre `80d881f9873679ee63d99ae4935705caae7c870a`, intégré sous
`2b0694d693872bf1c0e674060ebd42094e8e58f6`. Ses huit contrôles T2 et sept
contrôles T1 passent. La revue R20 a sélectionné 33 originaux puis réservé
l’appel de décision : ce parcours franchit le défaut de capacité de R19.
Il ne fournit pas de verdict final : son premier constat métier a déclenché une
pause et une correction, sans réutilisation de son avis.

Le contre-exemple R20 reproduit deux refus manquants : après retrait du demandeur
par configuration, son ancien mandat autorise encore approbation et premier
engagement. Le correctif lie nouvelle demande, approbation et premier engagement
à la révision de configuration courante. Une nouvelle configuration exige un
mandat renouvelé ; les effets déjà engagés restent réconciliables. L’Admin explique
cette conséquence avant application en FR/EN. Les 83 tests Go avec race
(32 principaux et 51 sous-cas), sept recettes publiques et 21 contrôles navigateur
passent localement, avec vingt dispositions et aucun diagnostic inattendu.

L’arbre correctif est `a5d59e04d055e7d7de27d96b0f6ad21d926d141a`.
Ses 32 originaux et rapport totalisent 392621 octets, sous la limite inchangée
393216. La remise publique enregistre le résultat
`b16b81dd05ea35f2bac1ea86824e40a79e9d4db3` ; ses contrôles frais sont
passent : huit contrôles T2 et sept T1 sur le candidat intégré
`b7179c5756bd56de2c5ff17c7b7ea0aa5239ac01`. La revue fraîche
`review-4c9934b4e3e64645f7c49c12` est active ; son premier appel est réservé
(89 consommés sur 92), aucun verdict final n’est encore disponible. Le refus sous pause puis le conflit de
révision sont conservés, sans mutation ni acceptation ; la reprise change cette
précondition. Les 88 appels et les deux tentatives restent conservés ; le plafond
92 appliqué publiquement en447 autorise quatre appels frais. La fenêtre demeure
400000 tokens, sous le maximum vérifié 872000 du client exécuté ; 40 activations
sur 60 sont consommées.

Le conducteur unique PID423881 est vérifié par son démarrage et sa commande dans
[son reçu](verification-launch-20261014/conductor-context-reuse-process.json).
Son binaire et `bin/swarm` sont identiques, SHA-256
`560a19612a0ff22346f000cf3860682f7fc76afa4f789aa93cc7a267ff5d7adf`,
lié aux sources par [le manifeste de qualification](verification-launch-20261014/context-reuse-source-proof.json).
L’ancien PID27445 est arrêté avec preuve ; aucun serveur de production n’a été
démarré par cette installation. Le réveil autorisé de ce chat reste actif et
actualisé ; il ne remplace ni la supervision active ni la preuve d’un résultat.

Les preuves originales, échecs et campagnes interrompues restent conservés.
Une campagne interrompue n’est pas PASS ; les tests locaux ne sont ni la revue
indépendante ni l’acceptation native. L’interdiction des réponses longues est
inscrite dans les consignes communes et reçue par les douze contextes de rôle :
réponse brève immédiate, étapes et sorties bornées, point utile au moins toutes
les 60 secondes. Une reconnexion affichée ne prouve pas sa cause.

Pour relire l’état public courant :

```sh
./bin/swarm --root "$PWD" --json work show w-68b50cc82ec3fe55624f9549
```

Les sections suivantes conservent les relevés historiques ; leurs anciens PID,
budgets, candidats et binaires ne décrivent pas cet état courant.

## Relevé historique — révision 430

## État observé — 15 octobre 2026, révision 430

**Swarm n’est pas qualifié pour diffusion. Un lot est accepté sur huit.**
La remise corrective publique a enregistré le candidat
`186518d36c2346d4e9fd8f3e169c543153171e4a`, arbre
`80d881f9873679ee63d99ae4935705caae7c870a`. Les contrôles frais sont en cours ;
aucune nouvelle revue ni acceptation n’est encore établie. Les 84 appels et les
deux tentatives restent conservés. R19 et ses constats ne sont pas réécrits.

Le binaire installé et celui qualifié sont identiques :
`3d35671fd4936ce2c671aee864212d55cbd02e3c02d98b0692c9455714492a2d`.
L’ancien conducteur PID 27445 est arrêté, identité et fin confirmées. La remise
publique en cours tient le parcours de contrôle ; aucun conducteur concurrent
n’a été démarré. Le nouveau réglage est sauvegardé par l’API publique de quotas
en révision 428, puis la planification reprise en 429 : fenêtre de revue 400000
tokens, sous le maximum observé 872000 du même client/modèle, 32 sources et
393216 octets. Plafonds de revue 89 et de planification 60 ; consommations 84 et
40 conservées. Ce changement de capacité et le candidat corrigé remplacent les
préconditions en défaut ; une hausse de budget seule n’aurait pas suffi.

Le même dossier natif avec ses 20 originaux et son schéma passe la mesure avec
109977 tokens de marge, sans retrait ni résumé. En local, 29 tests Go principaux
et 46 sous-cas du pilote passent avec race ; les sept recettes CLI/navigateur et
vingt dispositions passent. Les deux constats R19 sont reproduits avant correction :
mandat invalide approuvé et clé orpheline après échec d’écriture. La nouvelle
publication initiale est testée aux trois étapes d’interruption, après réouverture,
en concurrence et avec des originaux remplacés ou invalides.

Les contrôles du moteur couvrent la politique sauvegardée, la commande du client,
inspection/sélection/décision et réouverture, ainsi que la lecture historique d’un
avis fermé pour préparer un candidat différent. Le refus de relance inchangée
reste appliqué. L’Admin FR/EN, thèmes, mobile, aperçu sans mutation et compteurs
conservés passe sans appel fournisseur. Le format natif Go verbeux conserve les
75 cas et diagnostics en 20588 octets, au lieu des 80624 octets JSON qui dépassaient
la capture de 65536 octets. Les deux sorties sont conservées.

Ces preuves locales ne sont ni l’avis indépendant du nouveau candidat ni
l’acceptation globale. La campagne élargie interrompue conserve son échec et
n’est pas déclarée PASS ; la correction ciblée et le contrôle du manifeste
courant sont consignés séparément. L’interdiction des réponses longues reste
active : réponses brèves, étapes bornées et points utiles au moins toutes les
60 secondes ; une reconnexion affichée ne démontre pas sa cause.

Le conducteur antérieur a été arrêté après vérification de son PID, démarrage,
commande, empreinte et absence d’enfant. Le conducteur ciblé actuel est le PID
27445, identifié dans
[son reçu de processus](verification-launch-20261014/conductor-original-references-process.json).
Il utilise `bin/swarm-actions-confiance-original-references`, identique au binaire
installé `bin/swarm`, SHA-256
`163d097f6eb0991a1b95a211a4637cb149693c3921d60331c10e973357eb38a7`.
Aucun serveur de production n’a été démarré par cette installation.

La pause en révision 402 a conservé les 80 appels et les deux tentatives de T2.
Après qualification locale et aperçu public, le plafond minimal 85 est appliqué
en révision 403, les autres plafonds et consommations inchangés. La reprise
publique en révision 404 archive intégralement l’ancien avis R18, sans aucune
inspection réutilisée ; la révision 405 reprend la planification. La reprise prévoyait trois
inspections et deux appels finaux neufs ; quatre appels ont été consommés avant le
refus de capacité. Codex doit traiter ce refus, vérifier l’effet de la correction,
puis reprendre les autres lots et le RETEX.
Le réveil de ce chat toutes les cinq minutes est actif et mis à jour ; il ne
remplace pas cette supervision ni la preuve d’un résultat.

Le [manifeste final](verification-launch-20261014/citation-originals-source-proof.json)
lie 280 sources Go, 362 fichiers de tests, 251 ressources embarquées et quatre
entrées de compilation. La campagne élargie conserve 43 tests principaux passés
sur 44 et son échec initial. Après traitement de ce cas et du LF perdu à la lecture
Git, les [contrôles ciblés finaux](verification-launch-20261014/citation-raw-originals-final-race.out)
passent : six tests principaux, seize sous-cas, avec race. Les résultats précédents
sont réutilisés seulement pour leurs contrats et entrées inchangés. Vet et les
contextes des douze méthodes passent. Ces fixtures ne sont pas une revue native
indépendante ni l’acceptation du programme.

La [preuve de remise native](verification-launch-20261014/citation-originals-native-dispatch-proof.json)
vérifie candidat, contexte, plan, consignes figées et version 2. Pour relire l’état :

```sh
./bin/swarm --root "$PWD" --json work show w-68b50cc82ec3fe55624f9549
```

Les sections suivantes conservent les relevés historiques et leurs limites ;
leurs anciens PID, budgets et binaires ne décrivent pas cet état courant.

## Historique du lancement

Mission : `w-68b50cc82ec3fe55624f9549`. Préparation : `prep-89213e04486b23342819ace0`, révision 32 après configuration et autorisation publique. Les identifiants des huit tâches ont été conservés. La conversion et la configuration ne démarrent aucun fournisseur ; `mission start`, puis le conducteur ciblé, ont activé cette mission.

Le premier lot est effectivement exécuté par Codex, niveau standard, dans sa copie Git gérée. Le conducteur `mission watch` reste actif pour cette seule mission. Le relevé de démarrage constate une organisation prête, une mission autorisée et active, un worker avec des commandes exécutées et zéro lot encore accepté. Ce relevé établit le lancement, sans établir la réalisation des fonctionnalités.

Les quatre autres travaux sont identiques avant/après. Les trois fichiers déjà modifiés au départ restent identiques et sont exclus du binaire testé et de l'instantané du programme. Le binaire dédié provient de HEAD avec les corrections propres à ce lancement ; le binaire installé antérieurement reste conservé.

## Relevé courant — remise des originaux et garde de ressource

Relevé du 15 octobre 2026, avant résultat de la nouvelle remise : **1 lot accepté
sur 8**. T2 est en réparation externe, pas accepté ; T3–T8 restent dépendants.
Les paragraphes de lancement précédents sont des relevés historiques. Le binaire
courant inclut les sources locales décrites dans son manifeste, y compris les
changements préexistants ; l’exclusion initiale de trois fichiers ne décrit plus
les compilations ultérieures.

Le moteur publie finalement l’avis R10 `changes_requested` après correction du
lecteur de résolutions. Ses refus métier sont reproduits puis corrigés : audit
attribué des refus authentifiés et invalidation des aperçus Admin en vol. La revue
R11 échoue ensuite sur une citation présente mais attribuée à une autre pièce.
Le diagnostic commun indique maintenant les coordonnées, l’empreinte, l’extrait
et les correspondances exactes dans les autres pièces ; il ne déplace aucun
constat. Les prompts historiques restent identiques. La garde vérifie aussi que
le message tient dans le contexte remis au superviseur.

Le soupçon de compilation 32 bits est vérifié : trois constantes de tests, dont
deux héritées, débordaient `int`. Les valeurs passent désormais par le décodage
JSON strict et la validation existante. Trois tests nommés sont effectivement
exécutés sous `GOARCH=386` ; cela ne prétend pas exécuter toute la suite 32 bits.

R12 (`review-fb30671b67eb4d02f1bdd487`, candidat `10e3ff0f3a453fe22309aafb0424f315b2abbe1c`)
produit trois inspections valides, puis une sélection `unknown`. Deux causes sont
séparées : un nouveau défaut métier et un dossier de preuves mal recentré.
L’engagement et la réconciliation retournaient un résultat terminal avant de
vérifier une ressource explicite contradictoire. Le contre-exemple échoue avant
correction ; la garde commune aux trois opérations est désormais placée avant
ce retour. Six variantes engagement/réconciliation et trois états vérifient le
refus sans nouvel effet, l’audit, la ressource correcte et l’identifiant hérité.
La recette publique vérifie également ces deux refus.

Les rapports courants réclamaient d’anciens fichiers `.out` non fournis ; leurs
chronologies multipliaient les demandes de preuve dans une sélection bornée.
Les anciens octets sont conservés dans cette aire et les anciens candidats Git.
Le rapport courant fournit les originaux actuels, dont la sortie race brute comme
pièce textuelle sélectionnable. Les 31 sources complètes occupent 346648 octets.
Le quota de nombre de pièces passe publiquement de 30 à 31 ; le plafond de
352 Kio, les autres politiques, les 50 appels et les deux tentatives sont conservés.
Aucune limite de transport du fournisseur n’est modifiée.

Qualification locale du nouvel arbre `6bba6908bd5a18f609eb187b2a53fc4d39c99bcf` :
24 tests principaux et 34 sous-cas Go avec race passent ; le vérificateur public
observe 58 tests Go nommés et sept cas de validation. La livraison inclut les
parcours navigateur, vingt combinaisons langue/thème/largeur et les courses
réseau des aperçus. Vet et `git diff --check` passent. Ces contrôles locaux ne sont
ni la revue indépendante ni l’acceptation.

Après pause publique et état terminal, seul le conducteur identifié sans enfant
est arrêté. La reprise publique révision 273 permet la correction du même résultat
avec `confirm_review_unknown_correction`, sans nouveau producteur ni remise à zéro.
Les contrôles natifs et la revue du nouvel arbre sont en attente à ce relevé.
Le manifeste `verification-launch-20261014/current-evidence-replan-proof.json`
lie les preuves originales, refus avant correction et empreintes.

## Correction du contrat de lancement

Une organisation fournie après matérialisation était ignorée. L'attachement exige maintenant un plan frais, le travail et les tâches canoniques non démarrés, aucun profil préexistant et des verrous cohérents. Un nouvel événement ne simule aucune création, révision ou autorisation déjà appliquée ; le même event_id conserve son reçu. Les historiques, politiques et décisions antérieurs ne sont pas adoptés implicitement.

La préparation conserve le verrou Git jusqu'au résultat SQL et annule ses artefacts nouvellement créés si la transaction échoue. Les manifestes existants sont vérifiés pour leurs chemins, sélection, empreinte et objets Git ; les représentations finale et préparée doivent s'accorder. Une preuve incomplète ou divergente est conservée et refusée. Une reprise exige sa réconciliation ou la restauration de la base attendue ; un rejeu inchangé ne corrige pas cet état.

Les défauts ont été reproduits avant correction. Les contrôles ciblés et avec race detector passent, ainsi que le parcours CLI sur une racine isolée, go vet, le vérificateur opérateur, le contrôle des méthodes et git diff --check. Six contextes indépendants de revue source ont conservé leurs verdicts successifs ; le dernier approuve ce candidat sans finding. Ces revues sans outils ne reproduisent pas elles-mêmes les tests et ne constituent pas les revues des livrables du programme.

## Organisation conservée

Les politiques persistent 56 contrôles, 32 critères, un maximum de 20 tâches, 40 activations du responsable et 40 appels de revue. Chaque lot garde deux tentatives et les limites effectives du fournisseur. Les consommations sont conservées séparément ; les coûts monétaires non disponibles restent inconnus. Les appels directs de revue source précédant le lancement restent distincts du registre de la mission.

Les contrôles des fonctionnalités attendent leurs recettes publiques et leurs tests nommés ; aucun succès n'est enregistré sur une recette absente. Le collecteur eBPF doit démontrer une collecte réelle sur le périmètre du projet. Une permission ou capacité noyau manquante reste un blocage visible, et les autres lots indépendants peuvent continuer. Les effets destructifs restent simulés, sans paiement réel ni déploiement externe.

Les preuves originales de cette préparation sont conservées dans ce dossier et dans l'aire de travail `/tmp/swarm-actions-confiance-launch`. `launch-acceptance.json` est un relevé daté ; l'état courant se lit avec le binaire dédié et les opérations publiques `mission status`, `agent list` et `planning show`.

Contrôles Go observés : 17 tests principaux et 58 sous-cas dans le relevé ciblé. Conducteur observé : PID 532011. Source du binaire : `4e58f7cbf8200a58f69f1f4d03d657c3313fb642`, modifiée, empreinte `d175d62d85ab8dfdaff57818ec9ee06b429d85c5271eaa81563499e9d603fa37`.

## Première transition observée

Le lot T1 est accepté après ses contrôles et la revue indépendante `review-7191bdca102c7e18b7cfbb05` (état passed). Le responsable a consommé une activation et le vérificateur un appel, sur 40 chacun. Le conducteur a ensuite lancé T2, le pilote Biba, dans sa propre copie. Le relevé constate donc 1 lot accepté sur 8 et 1 lot en réalisation, sans échec de planification. Les deux erreurs d’outils rencontrées par T1 ont été suivies de corrections et de vérifications ; elles restent dans son historique. Cette acceptation concerne l’architecture, pas la réalisation des garanties prévues pour les lots suivants.

## Récidive et nouvelle frontière en échec

Les quatre incidents initialement relevés sur T2 sont une observation intermédiaire, pas le total final. La collision `containsString` est détectée par la compilation ; les tests Go nommés ont passé après correction. L'inventaire des fichiers et l'utilisation du répertoire courant vérifié ont corrigé les chemins observés. Ces corrections locales ne démontrent pas une prévention générale : Swarm fixe le répertoire du processus fournisseur mais ne contrôle pas chaque paramètre `workdir` de ses outils internes.

La recette navigateur a ensuite révélé un refus d'ouverture du serveur local dans le bac à sable du fournisseur : `listen tcp 127.0.0.1:0: socket: operation not permitted`. Les sorties publiques 200071, 200075 et 200079 conservent ce refus après correction de la récupération de la recette. Trois échecs finaux équivalents ont provoqué l'interruption à 78 outils ; le relevé public compte 17 erreurs au total. Une socket loopback éphémère fonctionne dans l'environnement opérateur, sans établir un succès du navigateur dans celui du producteur.

La seconde tentative reçoit une copie propre et aucun résultat Git immuable de la première. Les sources non remises restent conservées dans l'ancienne copie, sans être importées ; les activités historiques transmises ne contiennent pas les sorties originales. Cette limite est explicite dans le contrat de reprise, mais la décision attendait une conservation exploitable des travaux. Le superviseur racine a reçu le diagnostic par l'opération publique `planning resume`, événement `recurrence-1ac34a7218c643b3bba3db9bff29c1cc`, révision 28. Le dossier `verification-launch-20261014/t2-recurrence.json` conserve les observations et originaux attribués.

Le responsable racine arbitre la reprise et le propriétaire opérateur traite une éventuelle permission manquante. Aucune augmentation de droits ou de tentatives, aucun affaiblissement de contrôle, aucune acceptation n'est établi par ce signalement. La reprise du critère navigateur exige un environnement autorisé à ouvrir le serveur local, des sources conservées et un nouveau contrôle du candidat exact. Une consigne ou un diagnostic reçu ne remplace pas son effet vérifié.


## Replan du moteur pour chaque exécution

Les corrections ponctuelles n'empêchaient pas la récidive. Le diagnostic a
reproduit les frontières fautives : cause finale perdue dans une longue trace,
erreurs ordinaires non transmises pendant la production, reprise fondée sur une
simple déclaration, capacités supplémentaires ignorées, sources conservées
seulement lors d'une nouvelle tentative et confusion entre vérification locale
et qualification du candidat.

Deux routes ont été comparées : élargir les permissions du producteur pour chaque
recette, ou remettre ses sources avec leurs limites honnêtes à des contrôles
moteur obligatoires dans un environnement qualifié. La seconde est retenue :
`ready` version 2, sans PASS local inventé, sans permission supplémentaire ni
suppression du contrôle ou de la revue. Les tentatives version 1 restent figées.
Les identifiants T1–T8 et leurs dépendances sont conservés.

| Contrat corrigé | Preuve observée | Limite |
| --- | --- | --- |
| Diagnostic | Classification avant réduction ; début/fin et première/dernière trace conservés | Événements structurés visibles du fournisseur, valeurs sensibles masquées |
| Remise au responsable | `tool_failure` et `infrastructure` attribués au périmètre, sans doublon à compteur inchangé | Décision du superviseur requise dans ses droits et budgets |
| Reprise d'environnement | Déclaration seule refusée ; précontrôle fournisseur vérifié et capacités déclarées exigés | La sonde et les capacités requises doivent être correctement configurées |
| Sources interrompues | Snapshot attribué à la fin confirmée, remis au responsable même sans essai disponible | Checkpoint sans livraison ni acceptation ; refus de conservation visible |
| Livraison prête | `not_tested` conservé, contrôles réellement exécutés avant revue, refus sans acceptation | Fixtures déterministes distinctes d'un verdict IA réel |

Les 62 tests principaux et 67 sous-cas ciblés passent avec le détecteur de courses,
ainsi que `go vet`, la compilation, les méthodes et `git diff --check`.
Les contre-exemples avant correction, les premiers échecs de vérification et les
résultats finaux restent dans `/tmp/swarm-actions-confiance-launch` sous
`execution-replan-*.out` et `checkpoint-settlement-*.out`. Le contrôle des méthodes
a détecté un dépassement du contexte de 32 000 octets après ajout de consignes
redondantes ; la répétition a été retirée, sans troncature ni augmentation de limite.

Le conducteur de cette mission utilise maintenant le binaire construit depuis les
sources `bin/swarm-actions-confiance-execution`, PID 1231269, SHA-256
`c80f7edd7ed4bd4698fd97d80d08845ad9b600e538f3b07a3aebb80de1dc6a43`.
Son exécutable, ses arguments, son horodatage de processus et sa nouvelle activité
publique sont vérifiés dans `conductor-execution-process.json` et
`execution-replan-runtime-status.json`. Le binaire précédent est conservé ; les
trois fichiers initialement modifiés restent identiques et exclus par overlay.
Il ne s'agit ni d'un commit, ni d'une installation du serveur web principal.

### Retour au pilote T2

La seconde production s'est terminée avec une livraison partielle, sans revue ;
ses deux tentatives restent consommées. La planification est retenue pendant une
réparation opérateur autorisée de la copie arrêtée. Les défauts supplémentaires
ont été reproduits : fixture sans `init`, grand livre vide `null`, plafonds métier
arbitraires, recette confondant erreur réseau attendue et anomalie, et jetons CSS
absents masquant la lisibilité du thème sombre. Les recettes ont aussi été
renforcées contre les requêtes annulées par une navigation trop précoce et contre
les mesures prises pendant une transition de thème.

Les catalogues complets de plus de 440 Ko déclarés pour la revue empêchaient sa
réception. Une extraction par responsabilité fournit maintenant le catalogue
complet propre à la page, son générateur et le traducteur partagé ; les catalogues
globaux restent ceux du candidat accepté. Les dispatchers figurent dans le diff,
les implémentations métier complètes sont jointes. Les limites ne sont pas levées.

Les résultats locaux ne remplacent pas la réparation publique
`revise-recovered-result`, ses contrôles et sa revue indépendante. À ce relevé,
T1 reste le seul lot accepté sur huit ; T2 attend cette qualification, T3–T7
restent dépendants et T8 conserve la recette finale et le RETEX.


### Revue réelle et seconde correction

La réparation publique a reçu ses contrôles, puis l'avis indépendant
`review-97ee81b76a1096d0e7b7525e`. L'avis final reste `unknown`, avec inspections
et sélection originales conservées ; ses quatre appels ont porté le compteur de
revue à 5 sur 40. Trois constats locaux ont été reproduits puis corrigés : délai
de réconciliation positif sans effet, `verify` absent en HTTP, et deuxième objet
JSON admis avec mutation. La suspicion de redémarrage a été discriminée : les
appels utilisaient déjà des processus CLI séparés. La recette conserve maintenant
leurs PID et leur fin effective. Les champs de la modale anglaise sont également
vérifiés, au-delà du seul titre.

Une cause générale de manque de preuve a ensuite été reproduite : un contrôle
réussi de 10 477 octets ne livrait que 8 191 octets au vérificateur. La projection
intermédiaire de 8 Kio est retirée ; l'autorisation `review_output` reste requise,
la capture technique existante de 64 Kio et les limites du contexte restent
opposables. Deux nouveaux cas vérifient la fin de sortie, son empreinte et la
réception réelle dans le contexte du vérificateur simulé. Les sorties dépassant
la capture restent explicitement partielles. Les originaux historiques absents
ne sont pas réinventés.

Les deux fichiers de production déjà modifiés au départ, `planning.go` et
`recovered_result.go`, contenaient une reprise corrective explicite après
sélection finale `unknown`. Leur contenu antérieur est conservé et maintenant
réutilisé après inspection et vérification. Son cas positif échouait parce que
la fixture omettait le manifeste obligatoire ; cette précondition a été ajoutée
au test, sans affaiblir le contrôle produit. La version originale du test et les
premiers échecs restent conservés. Les six variantes couvrent succès, confirmation
absente, arbre inchangé, révision périmée, revue active et contexte altéré.

Les résultats ciblés incluent les 62 tests/67 sous-cas précédents, les cas passant
du relevé `execution-replan-full-source-race.out`, puis la reprise des cas en
échec après correction de leur fixture. Le dernier lot de 8 tests et 9 sous-cas
passe avec `-race`, y compris sortie complète, refus d'une capture réellement
tronquée, opt-in, persistance et reprise du checkpoint sans réexécution. Le
premier relevé global en échec reste un échec historique, pas un PASS reconstruit.

Le binaire de développement `bin/swarm` a été reconstruit par l'installateur
natif du dépôt, sans démarrer de serveur, après conservation de son prédécesseur.
La nouvelle qualification T2 utilise `bin/swarm-actions-confiance-final`, sans
l'overlay initial : les deux modifications de production préexistantes vérifiées
font désormais partie du binaire. Le conducteur reste momentanément arrêté pour
éviter une décision concurrente pendant cette qualification. La relance doit
reprendre le même travail et conserver toutes ses consommations.


### Capacité réellement utilisable et originaux déjà fournis

La troisième sélection finale a demandé un original de contexte de 107 689 octets
alors que seuls 62 255 étaient disponibles. Le même contenu complet figurait
pourtant déjà dans `task_contracts_reports_controls`. Le protocole final version 4
annonce cette identité exacte, vérifiée par octets et empreinte, dans
`already_supplied_originals` ; elle ne remplace aucun autre original. Les journaux
versions 1 à 3 et leurs messages sont reproduits sans modification. Les 26 tests
principaux et 43 sous-cas de ce périmètre passent avec `-race`.

Une deuxième cause a été reproduite : le constructeur conservait le transport
historique dès que ses paquets tenaient, sans examiner la capacité réelle du
client pourtant reconnue. Le client configuré observé est `codex-cli 0.159.2` ;
son catalogue embarqué déclare `gpt-5.6-sol`, fenêtre 272 000 et pourcentage effectif
95. Le moteur utilise maintenant cette observation et le comptage exact avant
les limites historiques. Il conserve réserves, identité du client et appels
restants ; aucun modèle ni quota de compte n'est changé. Les 18 tests principaux
et 11 sous-cas liés passent avec `-race`.

Les originaux complets supplémentaires demandés — parseur CLI, gestionnaire
HTTP/session et jetons de thèmes — sont joints sans réécrire leur contenu. La
capacité de sources de cette mission a été prévisualisée puis portée publiquement
de 128 à 224 Kio, dans l'autorisation utilisateur d'adapter les droits et budgets
du projet quand ils empêchent sa progression. Le relevé de quotas prouve 10 appels
de revue consommés avant et après, plafond toujours 40, six activations et six
décisions conservées. Les tentatives de production ne changent pas. Cette capacité
est persistée dans Admin ; elle ne lève ni la validation des originaux ni les
limites réellement observées du modèle.

La source du badge d'action et les messages de prochaine action sont localisés
sur une demande réelle dans le navigateur. Le montant affiche explicitement ses
unités mineures. Les mesures de contraste et l'erreur réseau volontaire figurent
maintenant dans la sortie structurée du contrôle, avec les états réellement lus,
et non seulement dans le nom d'un groupe de vérifications.


### Replan du schéma de sélection et de l'idempotence

La quatrième revue `review-0172834249b5ae9e231edb5f` a effectivement reçu la
capacité observée et les originaux supplémentaires. Les sept contrôles T2
passent ; la sortie de validation de 18 805 octets est reçue entière. L'avis
reste `error`, sans verdict final : le sélecteur a recopié 37 références et
altéré six caractères de l'empreinte d'une pièce. L'original est intact ; le
parseur a correctement refusé la réponse. Le coût reste 13 appels sur 40.

Deux solutions sont examinées : réparer une empreinte après réception, ce qui
transformerait une réponse invalide en preuve, ou borner la génération aux
références exactes. La seconde est retenue. Le protocole 5 fournit un schéma
dont chaque choix lie index et empreinte, avec les identités attendues. Le
contrôle indépendant du parseur reste obligatoire. Le contre-exemple local
échoue avec le schéma libre avant la correction ; les versions 1 à 4 restent
rejouables sans modifier leur journal.

Les hypothèses produit de l'inspection ont été reproduites séparément :
rejeu de configuration renvoyant une révision ultérieure, en-tête sans schéma
Bearer accepté et rejeu admis avant authentification. La réponse originale de
configuration est maintenant conservée sous ses octets JSON pour ne perdre
ni l'état d'origine ni la précision des grands entiers. Une ancienne réponse
absente reste un refus explicite, sans substitution ni mutation. L'authentification
précède le rejeu et l'API vérifie le schéma Bearer. Une erreur initiale du test
comparait l'ordre des champs après décodage générique ; elle a conduit à retenir
les octets originaux plutôt qu'à accepter un réencodage potentiellement arrondi.
Les sorties avant correction et les réponses indépendantes restent conservées.


### Réception et décision réelles, puis défaut lié de validation

Le conducteur reconstruit a été relancé avec identité, PID, début de processus et
empreinte vérifiés. Le superviseur racine a reçu l'erreur de citation de la pièce
18, a décidé `review-retry`, et le moteur a appliqué sa décision avec diagnostic
attribué au même avis et au même candidat. Les nouvelles inspections sont valides
et la sélection version 5 est `ready`. Les tentatives restent deux ; les appels
consommés sont conservés. Ce parcours prouve cette reprise de protocole, sans
prétendre corriger les défauts métier du candidat ni son acceptation globale.

La décision finale a ensuite révélé un autre défaut. L'hypothèse initiale d'une
nouvelle citation fausse est invalidée : l'extrait refusé de 524 caractères est
présent exactement dans `plan-criterion-2.review_output`. Le constructeur
d'originaux ne fournissait au validateur que le tableau JSON des contrôles,
avec ses guillemets et sa nouvelle ligne échappés. Deux contre-exemples isolés
reproduisent le refus d'une preuve pourtant reçue, pour l'avis et pour une
résolution de question. Le contrat est corrigé au même constructeur partagé :
les sorties déjà fournies sont aussi des originaux littéraux séparés. Les
variantes conservent les refus de données altérées, sorties absentes et collage
entre commandes. Aucune réponse indépendante originale n'est réparée en place.
Le conducteur est retenu pendant cette correction ; pause publique révision 104,
18 appels sur 40, huit activations racine conservées.


Le diagnostic récursif explique aussi l'attente de la huitième activation :
les échecs de réponse finale revenaient en erreur générique, sans le code de
protocole utilisé par l'inspection. Le contexte propriétaire indiquait donc
`review_retry_unavailable: true`. Quatre contre-exemples échouent avant correction :
JSON final, référence de sélection, citation de critère et citation de résolution.
Ils conservent maintenant un code structuré jusqu'au routage. Les défauts
d'intégrité des originaux et les vrais avis négatifs restent distincts ; aucun
ancien reçu n'est reclassé. Cette correction porte sur toutes les réponses de
ces contrats, sans exception liée au programme ou au texte de l'erreur.


Les contrôles ciblés de la correction littérale passent : 11 tests principaux
et 35 sous-cas avec race. Le lot final de typage/routage passe également :
11 tests et 49 sous-cas, dont les quatre réponses invalides, l'intégrité conservée
et les avis métier non reclassés. Les sorties originales restent séparées.
Statique, compilation, installation native sans serveur et contrôle des méthodes
passent. Le nouveau conducteur utilise le binaire `swarm-actions-confiance-review-recovery`
lié au manifeste `review-recovery-source-proof.json`. Le même candidat a été
remis en reprise finale par les opérations publiques ; aucun ancien JSON de
réponse n'est promu en avis. Les correctifs d'accessibilité de la copie opérateur
restent à qualifier ensuite sur leur propre arbre.


### Correction du refus métier T2

Le nouvel avis réel `review-fdadd3c686a8d0d0f77c40fa`, devenu
`changes_requested`, valide identité/Biba et reprise idempotente. Le refus sur
les originaux et la cible tactile est conservé. La copie opérateur retient
maintenant les octets exacts des commandes HTTP/CLI et le JSON canonique des
appels Go sans transport. Les entrées sont liées par SHA-256 à chaque décision
et exportées sans perte en base64 dans le contrat de preuve version 2. Les
variantes vérifient redémarrage, altération, omission, ajout non lié, exclusion
des justificatifs et refus sans mutation sur lacune historique. Les originaux
anciens absents ne sont pas fabriqués.

Les quinze tests principaux et quinze sous-cas passent avec race. La validation
publique confirme 30 cas Go nommés et sept cas de recette. Le chargement est
maintenant observé avec la vraie réponse API retenue ; vingt combinaisons de
langue, thème et largeur vérifient la géométrie. Le nouveau composant et ses
régressions portent les 23 sources au-delà des 224 Kio précédents. Après
prévisualisation, seule cette capacité a été portée à 240 Kio, révision 115 :
19 appels de revue sur 40 et neuf activations/décisions restent consommés, les
deux productions restent conservées. Les nouveaux contrôles et l'avis doivent
encore qualifier le nouvel arbre ; aucun succès local n'est une acceptation.


### Contrats complémentaires vérifiés avant nouvelle remise

La sixième revue est restée `error` (`review_fragment_protocol`), 23 appels
consommés. Le nouveau code structuré est effectivement présent dans ce reçu ;
les hypothèses de produit restent séparées du verdict final absent. Trois ont
été reproduites : empreinte du mandat inchangée après révocation, écriture de
8 Mio + 1 octet admise puis illisible, et jetons d'exemple publics acceptés.
La cible de la case à cocher est en revanche son libellé associé : mesures
71,59 px en FR et 46,59 px en EN, avec clic réel et restitution de l'état,
à 390 et 320 px. Cette hypothèse est réfutée par observation, pas changée en bug.

Un contre-exemple lié au contrat de rejeu a également été conservé :
9007199254740993 devenait 9007199254740992 pour une réponse métier mémorisée.
La restitution des réponses est maintenant commune à la configuration et aux
commandes métier, sans décodage générique en flottants. Le stockage vérifie sa
taille finale, saut de ligne compris ; le mandat créé et révoqué utilise la
même empreinte de contenu couvrant version et date de révocation. La modale
reprend les identités effectives avec des jetons vides, exige leur saisie et
conserve ses contrôles de persistance. Les préremplissages publics ne sont plus
des justificatifs valides. Les premiers échecs de chaque contrat restent dans
l'aire de vérification, y compris le garde de modification opérateur qui avait
attendu 21 appels de preuve alors que la source réelle en comporte 23. Cette
erreur de méthode n'est ni un incident natif ni une régression produit.

### Replan de la conservation des sources et des contrôles visibles

Le diagnostic de la sixième réponse distingue les citations erronées des
originaux retirés par le moteur. Les résolutions visant les fichiers CLI et HTTP
citaient leurs lignes physiques, mais `compactManagedReviewContext` avait retiré
ces fichiers déclarés parce que leurs octets étaient reconstructibles depuis
le diff. Les préfixes du diff empêchaient leur citation littérale. La correction
conserve tous les originaux déclarés et ne réduit que le contexte inchangé du
diff ; les contrôles de capacité restent applicables au contenu réellement reçu.
L’alternative consistant à normaliser les citations ou modifier l’ancienne réponse
est écartée : elle changerait la preuve. Les anciens contextes restent inchangés.
Le contre-exemple échoue avant correction puis accepte la source exacte, tout
en refusant les variantes altérée et aplatie.

Une autre réserve concernait les traductions : leurs empreintes ne prouvaient
pas l’exécution de npm sur le candidat. Le générateur du pilote dispose d’un mode
`--check` en lecture seule ; sa régression refuse un artefact périmé et prouve
qu’il n’a pas été réécrit. C4 joint les deux reçus npm complets avec empreintes
des entrées. Une erreur de diagnostic opérateur avait envoyé un Buffer à la
place du chemin dans le premier reçu ; elle est corrigée avant qualification,
avec conservation de cette sortie initiale. Les mesures géométriques n’attendent
plus les transitions de couleur ; les mesures de contraste les attendent encore.

Le navigateur a également reproduit l’arrondi 9007199254740993 → 9007199254740992.
Une projection décimale dérivée, distincte du montant entier autoritaire, préserve
son affichage. Les 19 tests principaux et 20 sous-cas passent avec race ; la
validation publique observe 39 cas Go nommés, sept cas de recette, vingt
combinaisons de disposition et les reçus npm. Le contrôle de livraison passe
avec son délai configuré de 15 secondes. La copie opérateur et son arbre sont
préservés ; ils nécessitent encore les contrôles et la revue natifs frais.

Les 24 pièces complètes représentent 257 196 octets après documentation et reçus.
La capacité totale a été prévisualisée puis appliquée publiquement à 256 Kio,
révision 130. Seul ce champ change ; les 23 appels de revue, neuf activations et
neuf décisions restent consommés. Aucun avis négatif ni essai n’est effacé.

Les contrôles du moteur complet passent : 26 tests et 54 sous-cas de revue avec
race, plus trois tests d’attribution/capacité des lots ; statique, compilation,
installation native et vérification des méthodes passent. Le conducteur actuel
utilisable est `bin/swarm-actions-confiance-complete-originals`, SHA-256
`ef9a32750ae8d3e5e75fd89b24289f93cbe7ce032b60c5cafbb42462a26b00e6` ;
la preuve de qualification lie ses 270 sources de production et ses résultats.
Le binaire par défaut est installé par `install.sh`, sans serveur implicite.

La septième réparation publique est attribuée à l’opérateur sur l’arbre
`b1876ee08b003fff5721c26e8361d48031b4ffaf`. Les sept contrôles natifs T2 passent
sur le candidat `c6c13be3f3eb51065d6783a1b66675c2ca1d4738`. La nouvelle revue
`review-481aead2af7d5e3314889c6e` reçoit exactement les 24 pièces T2 et leurs
257 196 octets, ainsi que les sources déjà acceptées de T1. L’égalité des
empreintes est vérifiée pour chaque original. Les 51 415 octets de validation
et les 12 284 octets du contrôle C4 sont fournis entiers, reçus npm compris.
L’observation initiale avait sélectionné `tasks[0]` (T1) : la seconde sélection
est liée à l’identité T2, avec conservation des deux relevés. Le verdict de cette
revue reste distinct de ces contrôles et de leur réception.

### Reprise propriétaire observée et arrêt réconcilié

La septième inspection a reçu toutes les sources mais a déclaré deux défauts sur
une pièce marquée `inspected`. Le refus de protocole est conservé. L’activation
racine 10 a reçu l’incident et décidé `review-retry` ; le moteur a transmis cette
directive et repris la même revue. L’inspection suivante est valide, la sélection
est `ready`. Ce parcours prouve réception → décision → application → effet,
sans nouvelle production ni modification d’une réponse indépendante originale.

Les deux hypothèses de sécurité sont reproduites dans des fixtures : un autre
sujet authentifié pouvait rejouer une réponse et une clé symbolique était suivie.
La copie opérateur lie maintenant le rejeu au sujet, aux rôles et à l’intégrité,
avec refus sans mutation après changement de droits ou absence d’attribution
historique. La rotation d’un justificatif pour la même autorité reste admise.
La clé est créée exclusivement pour la configuration initiale ; lecture régulière,
permissions privées, absence de suivi de lien et format de 32 octets sont vérifiés.
Une clé historique perdue n’est pas régénérée. Une variante a également reproduit
l’adoption d’une clé régulière ou d’un lien physique préparé avant configuration :
cette adoption sans attribution est maintenant refusée.

Le parcours CLI a détecté un défaut lié de la correction : le nouveau code
`unauthorized` n’appartenait pas au contrat partagé du CLI/API. La frontière utilise
maintenant `authorization_required`, vérifié dans le test et la recette publique ;
le refus ne devient pas une erreur générique. Les 24 tests principaux et 34 sous-cas
passent avec race. La validation publique nomme 58 cas et sept cas de recette.
Les mesures de disposition sont transportées sans perte dans une table avec ses
colonnes et dictionnaires, et les entrées npm partagent un manifeste d’empreintes.
L’égalité de reconstruction est vérifiée. Les 64 833 octets de validation tiennent
sous la capture technique déclarée de 64 Kio, sans retirer de cas ni changer le
vérificateur opérateur.

L’arrêt du conducteur après pause a révélé une autre frontière : le processus
pouvait disparaître avant le classement durable de sa dernière réservation de
revue. Une première correction a échoué sur l’identifiant d’événement trop long et
sur la transmission du nouveau code d’interruption ; leurs échecs sont conservés.
L’identifiant est maintenant une empreinte bornée. Après acquisition du verrou
d’attribution exclusif, la réconciliation écrit un nouveau journal, classe la
réservation sans résultat comme `interrupted` et remet `review_fragment_interrupted`
au propriétaire `reviewer_runtime`. Aucune décision de reprise n’est implicite.

Les 14 tests et 22 sous-cas de ce périmètre passent avec race, dont inspection,
final, conservation des originaux, transmission, récupération explicite et refus
sans nouvelle dépense. Un ancien test allouait 20 octets à une variation de
contexte de 28 octets, due au motif complet de clôture ; la marge de fixture est
corrigée, en conservant le résultat de 8 Ko hors du lot et le refus de sa reprise.
Aucun plafond de production n’est modifié pour ce test.

Le redémarrage natif confirme la même correction sur la réservation finale
`fragment-final-f2ab79f789fb417c5f7f71d8` : `reserved` devient `interrupted`,
la sélection et les inspections restent identiques, l’avis devient `error` typé.
Les 28 appels de revue sont conservés avant/après. Le relevé distingue l’activation
racine 11 sans décision supplémentaire du budget de revue. Le conducteur utilisé
est `bin/swarm-actions-confiance-orphan-recovery`, empreinte
`6a2e3a168de4a09a736b04caaef2c0dea7703caf4a119d58c4c9d5d13d9d9aea`.
Après pause publique, absence de descendants et état propriétaire vérifiés, ce
conducteur est arrêté ; aucune exécution cachée n’est annoncée.

### Nouvelle qualification et contre-exemple de citation

L’arbre corrigé `e206c4fbb7d9d42c1e7e49e452039701e71d40ee` est remis par
l’opération publique ; les 24 pièces complètes représentent 274 291 octets.
Après prévisualisation, seule leur capacité passe à 272 Kio, sans remboursement
ni changement des autres plafonds. Les 28 appels de revue restent consommés.
Les sept contrôles natifs passent sur `aee4912e14c203077e9906f56644d5f29708ad0e`,
avec 64 848 octets de validation transmis intégralement. La comparaison des
24 sources reçues avec la copie figée ne trouve aucune différence.

La revue `review-2d04785fe3141f8224439317` refuse d’abord une citation absente.
L’original invalide est examiné : quatre hypothèses supposent `height>44`, alors
que la pièce 17 contient exactement `height>=44`. Deux citations de JSON ont
également omis les préfixes `+` de leur diff. Aucun défaut produit n’est établi par
ces quatre hypothèses ; le refus de format n’est pas promu en verdict métier.
Le contre-exemple exact et les résultats natifs sont remis au responsable par
`planning resume`, sans modification du candidat.

L’activation racine 12 a reçu ce diagnostic et décidé une reprise attribuée du
même avis. Le moteur applique la décision et transmet l’opérateur réel avec les
règles de citation. Tentative, réservations et consommations restent conservées.
Le verdict de cette reprise reste à distinguer de sa mise en route : la décision
et la réception sont prouvées, l’acceptation ne l’est pas encore à ce relevé.

### Base vérifiée, originaux communs et récidive du contrat de constat

La huitième revue finit `changes_requested`, avec des critères `unknown`.
Le diagnostic initial « base absente » était trop large : le contexte natif
contenait déjà la base version 0. La portée de cette base n’était pas explicitée
dans les inspections partielles. Le contrat version 1 la rend explicite sans
reprendre un ancien verdict comme acceptation du nouveau candidat. Un autre
contre-exemple, isolé et distinct, vérifie aussi la présence de cette base quand
le dossier complet tient dans le transport. Les prompts historiques sont gardés.

Deux originaux communs réellement absents, `model.go` et `store.go`, sont ajoutés
au manifeste de T2 ; `main.go`, déjà reçu entier, est rattaché explicitement aux
critères. Les 26 sources complètes représentent 320 138 octets. Le plafond passe
à 320 Kio par aperçu/application publics dans l’autorisation utilisateur, sans
remboursement. La qualification ciblée du moteur compte 21 tests principaux et
22 sous-cas avec race ; le contrôle des 12 contextes de méthodes, vet, compilation
et installation sans démarrage passent. Un précédent contrôle élargi avait
échoué sur une ancienne assertion de motif : cet échec reste conservé. Le cas
corrigé vérifie `authorization_required`, autorisation absente/révoquée et erreur
de lecture explicite ; il ne crée aucun droit.

La neuvième revue `review-a533e395b07d47d75811ebad`, candidat
`671336f40f5e33279a61898131abab2809990c77`, reçoit cette base et les originaux ;
les sept contrôles natifs passent. Elle s’interrompt au second paquet : deux
défauts visent la pièce 14 dont le constat porte `v=inspected`. L’original et son
empreinte sont conservés sous `nested-association-original`, dans la même zone
de vérification. Aucun des deux soupçons ne constitue un défaut produit validé.
Le programme reste à 1 tâche acceptée sur 8, avec 36 appels de revue consommés.

Replan : renforcer encore les consignes aurait laissé la même contradiction
représentable, déjà revenue malgré un diagnostic reçu. Le choix retenu est un
contrat de constat version 2, où chaque défaut appartient structurellement à
son constat `fail`. Les anciens contrats restent lisibles ; aucune réponse
indépendante n’est réparée ou requalifiée. Le précontrôle compte le nouveau
schéma entier ; une reprise complète explicite doit préserver appels et histoire.
La première qualification détecte une assertion de fixture devenue ancienne :
`unknown` porte désormais une question précise et atteint la décision finale,
dont l’absence de résolution reste refusée. Le nombre d’appels attendu est
corrigé, sans réduire un contrôle de production. Le résultat natif de cette
nouvelle méthode doit encore être observé séparément des fixtures.

La qualification race élargie est interrompue par l’opérateur après capture de
pile : elle n’est pas un PASS. La pile montre du calcul JSON sous instrumentation,
pas une preuve de blocage du moteur ; les derniers sous-cas progressaient encore.
La vérification est séparée en couverture fonctionnelle des dépendances et race
ciblé sur les nouvelles frontières, sans modifier de délai de production.
Une fixture de journal final produisait encore un tableau historique tout en
annonçant le nouveau contrat. Elle fixe désormais explicitement son contrat
historique ; les producteurs de fixtures courantes émettent des constats indexés.

Le premier aperçu opérateur utilisait à tort `task` ; le parseur strict refuse
ce champ sans mutation. Après lecture de `PlanningRequest`, la requête `task_id`
atteint le vrai précontrôle : trois inspections et deux appels finaux sont requis,
contre quatre disponibles. Les deux réponses restent conservées. La recette FR/EN
documente la clé publique exacte. L’augmentation préparée du budget de ce seul
projet conserve les 36 appels et ne change aucun quota fournisseur.

Les 109 tests principaux de dépendances ont été exécutés : 107 passent et deux
producteurs de fixtures historiques échouent sous le nouveau contrat. Les deux
corrections de fixture sont ensuite couvertes par une qualification ciblée avec
race : 15 tests principaux et 25 sous-cas passent, dont reprise de méthode,
conservation des avis, budget, preuves altérées et réception des constats indexés
par les adaptateurs de fournisseur. Les sorties FAIL et l’exécution interrompue
restent distinctes de ce PASS. Un premier sélecteur race trop strict ne collectait
qu’un test ; le sélecteur corrigé et les noms réellement exécutés sont conservés.

Le binaire issu du parcours de compilation canonique est
`bin/swarm-actions-confiance-nested`, SHA-256
`304f72d41dbe1cd785b7920ad19b34503595578da6e9c884d67faffb1e2e1508`.
Les 282 fichiers Go de production et les tests sont liés par empreintes dans
`nested-qualified-source-proof.json`. Installation native sans démarrage, vet,
contrôle des 12 contextes de méthodes et `git diff --check` passent. Le budget
de revue du projet est appliqué publiquement à 64, révision 194 : les 36 appels
restent consommés et toutes les autres limites sont identiques. L’aperçu devient
`method_restart_ready`, trois inspections neuves, deux appels finaux, zéro avis
réutilisé. La réception de ce diagnostic par l’activation racine 16 est observée ;
sa décision et le résultat natif restent à vérifier à ce relevé.

### Réception native du contrat2 et résolution du reçu

L’activation racine 16 décide `review-retry` en révision197. Le moteur crée
`review-4ce246e4f3db9ce3d191d8fe` sur le candidat inchangé, avec trois paquets
version 2 et zéro inspection réutilisée. Le journal précédent est intact et les
deux tentatives sont conservées. Les trois inspections aboutissent respectivement
à `changes_requested`, `unknown`, `changes_requested`, avec des réponses valides
sans tableau global de défauts. La sélection finale est `ready`.

Le premier avis final original passe les critères1/2 de T2 et refuse3/4, mais
le moteur ne peut encore le publier : une résolution est rejetée. Les activations
racine17/18 reprennent deux fois sans isoler la bonne citation. Le message générique
« résolution sans preuve originale visible » et le dernier constat partiel UI
ont entraîné un diagnostic erroné d’absence de `web/trust-actions.js`. Les appels
passent de41 à43. Les nouveaux départs sont suspendus publiquement, révision224,
puis le conducteur identifié est arrêté après absence de descendants et état
durable `error`. Aucun appel n’est remboursé.

La lecture pure des originaux trouve le vrai contre-exemple : paquet2, pièce12,
besoin1, citation du champ `candidate_commit` du reçu moteur. Le reçu est bien
transmis ; le parseur des résolutions ne le considérait pas comme original de
contexte. La fixture avant correction échoue sur ce cas et sur l’absence des
coordonnées dans le diagnostic. La correction admet le JSON effectivement visible
du reçu uniquement pour la résolution, jamais comme preuve d’implémentation. Les
refus incluent désormais les trois coordonnées, l’empreinte complète et un extrait
borné de la citation. Les gardes refusent un reçu absent, une identité altérée,
une opinion de producteur ou l’usage du reçu pour justifier l’implémentation.

La relecture du même avis original, sans appel fournisseur ni mutation d’une base
de mission, devient `changes_requested` avec deux tâches. Sous race, rendu2,86s et
parse0,18s : l’hypothèse de blocage de calcul n’est pas confirmée. La lecture de
l’état public était trop espacée et avait masqué des reprises déjà effectuées ;
il faut suivre le parcours public pendant le diagnostic, pas seulement le premier
état running. La preuve distingue lecture locale corrigée et future publication
native d’un nouvel avis indépendant.

Deux constats fonctionnels sont reproduits indépendamment sur le binaire T2
retenu : un mandat invalide avec identité Admin valide laisse `evidence_count` à0 ;
un aperçu retenu pendant la modification d’un champ devient applicable au retour
réseau. Le premier montage de fixture n’avait qu’une identité, refusée par le
contrat d’au moins trois identités ; il est corrigé avec les quatre rôles du
producteur de recette. Les refus et les deux preuves originales restent conservés.
Les corrections métier ne sont pas encore annoncées comme réalisées à ce relevé.

### Remise corrigée reçue et nouveau refus métier

La remise de l’arbre `6bba6908bd5a18f609eb187b2a53fc4d39c99bcf` crée le candidat
`9fecfc8acb9cddefd30b629f4bda8ff49900cb60`. Les sept contrôles natifs passent ;
la sortie de validation complète contient les 58 cas Go nommés, sans troncature.
R13 (`review-4b43f0d3a78a3c830eac85ce`) termine ses trois inspections, sélectionne
quinze originaux et rend un vrai verdict `changes_requested` en révision 287.
Les appels passent de 50 à 55 ; les deux tentatives sont conservées. Ce parcours
établit la réception et l’effet de la remise recentrée, sans erreur de format ou
blocage de capacité sur cette exécution. Il ne garantit pas tout futur avis.

Les défauts métier sont reproduits par le CLI sur une racine isolée : approbation
sans empreinte complète admise ; seconde demande différente avec la même clé
créée avant le conflit d’engagement ; champs non fiables `actor`/`integrity`
influençant le rejeu. Trois gardes Go échouent avant correction. L’implémentation
en cours exige l’empreinte, détecte le conflit dès la demande et normalise ces
champs sans réécrire les événements historiques. La qualification fraîche et la
nouvelle revue restent requises. Un premier test de rejeu supposait à tort un
`TrustAction` : le producteur retourne volontairement les octets `json.RawMessage`.
L’assertion compare désormais les JSON exacts ; la panique originale est conservée.

### Deux deltas et deux destinataires du contexte

R13 refuse aussi de réévaluer T1 : le champ `diff` vidé dans le message final est
interprété comme un delta incrémental vide, alors que les paquets contiennent
217450 caractères de changements depuis la base acceptée. `changes_since_observations`
est absent car aucune inspection ancienne n’est réutilisée ; ce champ concerne
un autre delta. L’original R13 reste intact.

Deux options sont comparées : retransmettre systématiquement tout le diff, avec
risque de dépasser la capacité du client, ou expliciter le contrat de transport
et son inventaire exhaustif tout en gardant la sélection des sections entières.
La seconde corrige l’ambiguïté sans couper de preuve ni forcer de verdict. Le
contrat de base 2 distingue diff canonique, champ vidé et observations réutilisées ;
les versions 0/1 conservent leurs octets. Le décideur garde `unknown` pour une section
nécessaire réellement absente. Les tests interdisent de citer le diff caché comme
si le décideur l’avait reçu.

La première correction atteignait le décideur, mais pas le sélecteur de pièces :
le nouveau test échoue sur ce second destinataire. Les 127 secondes de contrôles
conservent cet échec distinct des autres réussites. Le transport de sélection est
ensuite corrigé ; sa qualification reste à conclure à ce relevé.

Les trois corrections métier passent les 24 tests principaux et 34 sous-cas avec
race, la validation publique de sept cas et 58 noms Go, la livraison navigateur,
vet et trois tests ciblés réellement exécutés en 32 bits. Le parcours CLI isolé
observe une seule action, le rejeu identique malgré les champs ignorés, le refus
sans empreinte et l’accord avec l’empreinte exacte. L’arbre externe prêt est
`93f2fdbb0e4d702ec0d9dce548dc0ed94ec65918` ; il exige encore la qualification
moteur fraîche et un nouvel avis indépendant. Les 31 originaux complets font
358456 octets, dans le quota inchangé de 352 Kio.

### Qualification du contrat de base 2 et nouvelle remise

Le contrôle complet des 28 tests principaux avait quatre échecs, pas seulement
celui du sélecteur : trois parcours de chaîne refusaient encore la version 2 dans
le lecteur d’intégrité principal. La cause est un consommateur oublié, pas une
permission réellement révoquée. Le constructeur, le lecteur de paquets, le rendu
et la vérification des originaux partagent maintenant la même liste de versions
acceptées. Le garde-fou d’attribution, de fraîcheur et d’intégrité reste actif.

La qualification finale avec race exécute huit tests principaux et sept sous-cas
incrémentaux, tous réussis : portée des deux messages, inventaire complet, absence
de preuve cachée, nouvelle décision, chaîne relue après réouverture et refus des
bases altérées. Deux tests de sélection d’originaux et deux tests de compatibilité
avec cinq sous-cas historiques passent séparément. Ces résultats réutilisés sont
liés aux entrées inchangées ; le sélecteur intermédiaire trop strict, qui n’avait
collecté que deux tests principaux, n’est pas annoncé comme une qualification
des scénarios qu’il n’avait pas exécutés. Les premiers échecs restent conservés.

Vet, contrôle des douze méthodes, compilation et installation native sans
démarrage, ainsi que le contrôle de diff passent. Le binaire dédié
`bin/swarm-actions-confiance-diffscope` a pour SHA-256
`143e1c4e39ce20aadefb37f3485751e6626e102085b86bec0b9b1ca2c1ccc8b2`.
Le manifeste `baseline-transport-source-proof.json` lie 282 sources Go de
production, 353 fichiers de tests et 255 entrées embarquées/de compilation.
Ce n’est ni une revue indépendante du moteur ni un déploiement du pilote.

La correction externe du même résultat est remise publiquement en révision 288,
arbre `93f2fdbb0e4d702ec0d9dce548dc0ed94ec65918`. Le programme conserve
55 appels consommés et ses deux tentatives. Les contrôles natifs puis l’avis
indépendant de ce nouveau candidat restent à observer à ce relevé.

### R14 : gardes réellement testées et lecteur de réponses structurées

Le nouveau candidat `f715c17eaacde8cfda0d1e26ecd7278cff929631` est reçu par
R14 (`review-ccf9e3a44811222ac65d2748`), avec les 31 sources exactes, ses sept
contrôles réussis et trois inspections neuves au contrat de base2. Les inspections
révèlent deux causes distinctes : des tests négatifs d’approbation désormais
masqués par l’empreinte obligatoire, et un lecteur d’état qui vérifie un inode
par Lstat puis rouvre le chemin pour lire.

Le premier défaut est reproduit en retirant la garde de ressource par overlay
Go isolé : l’ancien test reste vert. Après correction des champs valides et du
motif attendu, ce même retrait fait échouer le test. Les trois overlays retirant
respectivement ressource, état terminal et montant provoquent maintenant trois
échecs comportementaux nommés, sans erreur de compilation. Ils restent hors du
candidat livré. Le lecteur d’état est reproduit avec un ouvreur contrôlé : un
remplacement après validation lit la révision2 d’un autre fichier valide. Après
correction, ouverture unique O_NOFOLLOW/O_NONBLOCK, fstat et lecture bornée sur
le même descripteur refusent cette substitution ; le remplacement du chemin après
ouverture conserve l’inode original. Répertoire, FIFO, lien et taille excessive
sont également refusés. Les 24 tests principaux et 34 sous-cas passent avec race.

R14 termine ensuite en `error`, révision301, 60 appels consommés. Ce n’est pas
une preuve de JSON mal généré par le modèle : la capture conservée se termine par
`[Réponse tronquée à 16000 octets]`. Le lecteur commun providerReply appliquait
la projection humaine à la réponse structurée avant la borne contractuelle de
64Kio. Cette coupe introduit le saut de ligne non échappé et détruit la fin du
JSON. Le fichier conservé est exactement le texte déjà coupé par le moteur ; la
fin du fournisseur n’a pas été conservée et n’est pas reconstruite. Le journal
classe l’appel interrompu ; le fichier brut de diagnostic reste séparé du verdict.

La partie review encore décodable contient des opinions positives sur T1 et
les critères1/2/4 de T2, avec refus du critère3. Ce préfixe est uniquement un
diagnostic : les résolutions manquantes empêchent tout verdict valide et toute
acceptation. La correction du transport doit préserver l’original structuré,
maintenir la borne explicite de64Kio et laisser la projection humaine historique
inchangée. Les doubles Codex/Claude de 15999 à65537 octets échouent avant cette
correction ; aucun appel payant supplémentaire n’est lancé pour ce diagnostic.

### Qualification de la réponse originale et remise R15

Le parcours réel de sous-processus simulant les adaptateurs Codex et Claude
conserve désormais les JSON de 15999 à65536 octets exactement ;65537 est refusé
avec sa taille et le texte complet capturé. La projection humaine historique et
l’exclusion du raisonnement restent vérifiées. Onze tests principaux et dix-sept
sous-cas passent avec race, ainsi que deux tests principaux et cinq sous-cas de
livraison/contextes. La leçon des refus masqués est incluse dans le prompt de
livraison construit pour les nouveaux workers ; son application par un nouveau
worker natif reste à observer. Aucun contrôle ou critère n’est affaibli.

Vet, les douze contextes de méthodes, le contrôle de diff et l’installation native
sans démarrage passent. Le binaire `bin/swarm-actions-confiance-structured` a pour
SHA-256 `6af965eacece7fb60290a34d568239da3b1064cca46abc68f1b3f5c2c3c0243e`.
Son manifeste lie 282 sources Go, 354 fichiers de tests et les entrées embarquées.
Les preuves du pilote restent séparées :24 tests principaux/34 sous-cas avec race,
58 noms Go de validation, trois tests32bits et trois mutations détectées. Le
remplacement contrôlé de fichier est une fixture déterministe, pas une attaque
privilégiée exécutée sur une mission.

Le manque est mesuré avant reprise :60 appels consommés et4 restants, contre
3 inspections plus2 étapes finales ;31 originaux font364903 octets, au-dessus des
360448 autorisés. L’opération publique révision302 applique80 appels et360Kio,
selon l’autorisation existante, sans effacer les60 appels ni les2 tentatives.
Les autres limites et celles du fournisseur restent identiques. Cette hausse
ne répare pas la réponse coupée : la cause est corrigée séparément dans le lecteur.

La correction externe de l’arbre `5a0820cb475291be2fa87c2fb9191395d6effa4b`
est remise avec `confirm_review_error_repair`. R15
(`review-f5b1b04170c262cb710b41e6`) ouvre trois inspections neuves, sans réutilisation
d’avis ; le relevé révision305 compte61 appels. Le contrôle et le verdict de ce
candidat restent à conclure à ce point.

### R15 : attribution de la cible et portée du refus

R15 reçoit le candidat `1ccf61a749cb7143455ce0287daa8d600abd67ae`, ses31 sources
et les sept contrôles réussis. La validation contient58 cas Go nommés sans
troncature. L’inspection2 fournit un vrai JSON complet de2319 octets, mais sa
citation se retrouve trois fois dans l’original. Le parseur conserve son refus
`ambiguous quote` et n’invente aucun numéro de ligne. L’erreur typée reste
`review_fragment_protocol`, révision310,62 appels conservés. Il ne s’agit pas
de la coupe16000 corrigée au tour précédent.

Le constat est reproduit indépendamment : un `approve` visant un action_id absent
ajoute une preuve dont ObjectID est vide. Le résolveur commun de cible sert
désormais à la recherche et à l’audit dans approve/commit/reconcile, y compris
les refus préalables de rôle du service d’effet. Huit variantes vérifient cible
explicite, alias, autorité, original, rejeu sans écriture et grand livre inchangé.
Le CLI public vérifie également les trois ObjectID exportés pour ces opérations.
Le contre-exemple Go FAIL puis les24 tests principaux/34 sous-cas race PASS,
la validation de58 noms et les trois cas32bits sont conservés.

L’autre constat partiel vise un contrat documentaire trop large sur les codes
d’erreur. R03 exige un refus sans mutation ; il n’impose pas un statut HTTP aux
décisions de politique. La documentation décrit désormais le comportement réel :
state=denied, motif, prochaine action et business_mutated=false sont une décision
explicite en HTTP200/CLI0 ; les erreurs de validation de création de mandat,
conflits d’idempotence et authentification conservent leurs codes distincts.
Aucun refus ni test de sécurité n’est converti en autorisation ou affaibli.

Les JSON de qualification courante sont compactés sans perte de valeurs ni
modification de leurs sorties originales. Les31 pièces complètes font368240
octets, dans le plafond inchangé de368640. Les contextes et avis des candidats
précédents restent immuables. La prochaine remise exige de nouveaux contrôles
et une revue indépendante ; elle ne présume aucun verdict.

### R16 : preuve vide et critères absents des inspections

R16 rend une décision JSON complète : T1 passe, T2 C3 échoue car un export vide
et une ancre de séquence zéro pouvaient être déclarés vérifiés, même avec une
clé de longueur invalide. Le contre-exemple échoue avant correction ; la preuve
exige désormais des événements, une ancre positive et une clé de 32 octets. Les
24 tests principaux et 34 sous-cas passent avec race sur cette correction locale.
Cela ne constitue pas une nouvelle acceptation native.

Les questions finales révèlent aussi une frontière de transmission : les critères
et la base acceptée n’arrivent qu’au paquet contenant les métadonnées. Les autres
inspections réclament des réaudits sans rattachement au contrat courant, jusqu’à
la décision finale elle-même comme précondition circulaire. Deux options sont
comparées : recopier toutes les métadonnées et tous les fichiers historiques
augmente sans borne le contexte ; transmettre à chaque paquet un périmètre
compact dérivé du contexte canonique permet un examen borné et vérifiable.
Codex retient la seconde, avec un contrat versionné pour préserver les anciens
messages. Critères : chaque inspection reçoit tâches, critères et delta nommé ;
un périmètre altéré est refusé avant appel ; la décision finale tranche la
pertinence des demandes avec preuve du contrat, sans supprimer les questions,
sans accepter un défaut ni masquer une preuve nécessaire. Les contrôles natifs
de race et de portabilité restent une lacune distincte à traiter par la politique
publique, sans transformer les déclarations locales en reçus du moteur.

Le premier précontrôle de qualification ne lance aucun test : propager
GOFLAGS=-race à build.sh échoue car ce build force CGO_ENABLED=0. L’original
est conservé dans t2-native-race-preflight.out. Le wrapper est corrigé à la
frontière pertinente : recette publique avec son build existant, puis tests Go
avec -race et CGO_ENABLED=1. Aucun code produit, contrôle ou compte n’est modifié
pour rendre ce refus vert. La portabilité exécute bien ses trois cas nommés.

Le parcours public est reproduit en fixture : modification des contrôles après
un refus → avis archivé → correction externe refusée faute d’avis courant.
`policy-recovery-before.out` conserve le FAIL. La reprise permet désormais de
désigner uniquement le dernier avis archivé attribué à la réparation courante,
avec contrat et base identiques, originaux intacts et nouveau contrôle complet.
Ce choix conserve l’invalidation des preuves anciennes ; restaurer artificiellement
l’ancien avis comme courant ou le réutiliser pour accepter est écarté. La preuve
d’application native et la nouvelle revue restent à obtenir.

Les précontrôles courants exécutent58 noms Go avec race et les sept cas publics,
dont CLI de preuve vide refusée et vingt dispositions navigateur. La portabilité
exécute trois tests386 nommés. Les sorties complètes font65078 octets pour la
qualification race, sous la capture64Kio ; elles restent locales jusqu’aux reçus
frais. Les nouvelles régressions moteur passent :17 tests principaux/28 sous-cas,
puis neuf principaux/dix sous-cas pour les dépendances de reprise et les deux
parcours de capacité. Le test supplémentaire de reçu archivé altéré passe avec
race. Vet, les douze contextes de méthodes et le contrôle de diff passent.

La politique publique est reçue en révision325 et la correction externe en326,
avec l’avis R16 explicitement conservé comme précédent. Le binaire lié au
manifeste exécute ces opérations. La décision finale et son effet sur T2 ne sont
pas encore établis ; la réception seule ne clôt pas ce lot.

### Réception native du périmètre et des nouveaux contrôles — R17

Le candidat `3d66a1fdd7d6a16f848f1f3f79890c6fcb0fcd43` passe les huit contrôlesT2
et les sept contrôlesT1 sur le même reçu. La sortie race complète fait65071
octets et contient58 cas Go nommés ; le contrôle386 fait3362 octets et exécute
les trois cas requis. Le contrôleC3 constate le refus de la preuve vide.
Le contexte original porte inspection_scope_version=1 et une base version2 ;
ses39 sources comprennent les30 piècesT2 et celles deT1. Chacun des trois
paquets contient les deux tâches et leurs huit critères, même sans les métadonnées
complètes dans son propre paquet. Le premier appel est réservé avec son empreinte
de paquet en révision328 ; cela établit le message préparé pour cet appel, pas sa réception par le
fournisseur ni l’avis à venir.

Les [reçus et paquets originaux](verification-launch-20261014/scope-native-controls-proof.json)
conservent les dates, sorties et attributions. La réception, l’exécution des
contrôles et le démarrage de R17 sont établis. Sa décision et l’effet sur le
statutT2 restent à vérifier ; le compteur est68/80, sans remise à zéro.

En révision330, le premier avis partiel est reçu et validé : son candidat, son
contexte et son empreinte de paquet correspondent aux originaux préparés.
L’original et ses limites sont conservés dans
[le relevé de réception](verification-launch-20261014/scope-native-inspection-0-received.json).
Ses demandes portent notamment sur les contrôles et sources des autres paquets ;
elles restent des questions pour la décision finale. Cette réception ne démontre
ni la résolution de ces questions ni l’acceptation. Les huit sorties de contrôle
sont vérifiées octet pour octet contre leurs tailles et SHA-256, avec58 noms
Go et trois noms386 effectivement exécutés :
[assertions sur les reçus natifs](verification-launch-20261014/scope-native-exact-control-assertions.json).

### R17 : longueur Unicode de la demande de preuve

R17 s’arrête en révision333 sur « demande de preuve invalide »,69 appels
conservés. L’original du paquet1 est un JSON complet : une demande compte240
caractères Unicode, conformément au schéma transmis, mais243 octetsUTF-8. Le
parseur mesurait des octets. Ce défaut du contrat moteur ne prouve aucun défaut
du pilote et n’autorise aucune validation manuelle. L’original n’est ni tronqué
ni réécrit. Le plan est révisé : garder le comportement des anciens paquets,
versionner les nouvelles inspections avec comptage Unicode conforme au schéma,
conserver la limite totale de transport et fournir un diagnostic avec les deux
mesures. Une nouvelle revue complète sera nécessaire ; aucune réponse ancienne
ne deviendra un verdict par réparation. L’omission du diff dans la pièce de
métadonnées sera également explicitée dans les nouvelles consignes partielles.

Le contre-exemple du parseur échoue avant correction : deux tests principaux,
dont les cas de240 caractères accentués/horsBMP et le minimumUnicode. Le nouveau
contrat d’inspection1 distingue caractères et octets, conserve le contrat0 des
anciens paquets et refuse une version inconnue avant appel. La première campagne
ciblée passe avec race, y compris la nouvelle revue complète après changement de
méthode, le refus des anciens originaux altérés et les deux modes de capacité.
Le contrôle supplémentaire UTF-8 et le comptage sur les sources finales restent
en vérification avant compilation et reprise publique. Les69 appels consommés
et le refus R17 restent intacts.

### Reprise publique sous le contrat Unicode — R18

Les seize tests principaux/trente-huit sous-cas du contrat d’inspection et de la
reprise de méthode passent avec race ; les trois tests principaux/sept sous-cas
finaux couvrent en plus le UTF-8 invalide, les caractères horsBMP et les deux
parcours de capacité, sans référence à un scope absent. Vet, les douze contextes
de méthodes, les liens des cinq pages modifiées et le contrôle de diff passent.
Le binaire402fdbee… est compilé/installé sans démarrage de serveur, sous son
manifeste ; les anciens binaires et preuves restent conservés.

`planning retry-review` reçoit la correction en révision334, archive R17 et
conserve69 appels ainsi que le candidat et ses contrôles. Le conducteur ciblé
reprend le résultat existant ; R18 démarre en336 avec70 appels et trois inspections
entières nouvelles, chacune inspection_contract_version=1 et scope.version=1.
Ce démarrage prouve l’application de la décision de reprise et la préparation
des messages ; leur réception, le verdict final et son effet restent à suivre.

La première inspectionR18 est reçue en révision338 sous inspection1 : identités
et empreinte correspondent, son état partiel estunknown, jamais accepté.
Le [relevé de réception](verification-launch-20261014/unicode-native-inspection-0-received.json)
lie cet avis au nouveau contrat. Le deuxième appel est réservé ;71 appels sont
consommés. L’identité, le démarrage et le SHA-256 du conducteur3407194 sont
revérifiés. La mission poursuit son exécution ; cette documentation ne clôt niT2
ni le programme initial.

### Supervision Codex persistante et citations finales — révision 370

L’utilisateur demande explicitement de ne plus devoir réveiller Codex. Les
réponses précédentes confondaient continuité du conducteur et supervision
effective : le processus vivant a continué, mais quatre décisions finales ont
échoué sur des citations modifiées. Le réveil du chat est désormais créé et
confirmé dans l’application, toutes les cinq minutes, sous
`superviser-swarm-actions-de-confiance`. Il doit reprendre le diagnostic et agir,
pas seulement signaler un état. La première création a été refusée faute de
destination de chat ; la création corrigée cible bien cette conversation.

Le relevé constate 77 appels conservés sur 80, T1 seul accepté. Les contrôles du
pilote restent verts ; les décisions finales modifient notamment des échappements
JSON ou l’indentation d’un extrait de patch. Le parseur refuse à raison ces
citations. Répéter une consigne générale n’a pas corrigé cette frontière.
La planification est temporairement mise en pause par opération publique en
révision 370 ; le conducteur appartenant à ce travail est arrêté après contrôle
de son PID, démarrage, binaire et absence d’enfant. Les sources, avis, appels et
réservations historiques restent intacts. Ce n’est pas un abandon du programme.

Options : élargir l’admission aux citations normalisées masquerait les écarts ;
répéter une instruction générale laisserait le même défaut de transmission.
Codex retient un diagnostic de citation attribué aux originaux exacts, transmis
avec la décision de reprise avant le prochain appel, et des conventions explicites
pour octets, JSON, espaces, tabulations et marqueurs de patch. Le refus littéral
reste inchangé ; l’agent peut corriger sa citation ou garder unknown. Critères :
reproduction de l’absence de ce diagnostic, attribution et bornes, persistance
au redémarrage, réception par le vrai parcours de sous-processus et effet observé,
sans acceptation implicite ni nouvelle tentative de production. Reprendre le
programme après vérification de ce correctif général.

### Diagnostic de citation qualifié, puis refus de capacité avant reprise

Les neuf tests principaux et six sous-cas ciblés passent avec race sur les
sources du binaire `b83ee971…`. Ils couvrent notamment la réception réelle par
un sous-processus de fixture, après réouverture, pour les décisions opérateur et
superviseur, les originaux altérés et les conventions avant premier appel.
Vet et les douze contextes partagés passent. Ces résultats déterministes
n’établissent ni le verdict natif ni l’acceptation du pilote.

La reprise publique révision 371 quitte la pause. Le précontrôle de
`retry-review` refuse ensuite « consignes et paquet d’inspection dépassent la
limite ; aucun envoi tronqué », avant mutation et dépense supplémentaire. Les
trois inspections et la sélection finale sont pourtant déjà durables : le
précontrôle ajoute à tort la nouvelle consigne à tous les anciens paquets.
La planification est remise en pause publique en révision 372, toujours à
77 appels. Raccourcir au hasard la consigne laisserait ce défaut général ;
augmenter le transport ne serait pas justifié. La correction retenue vérifie
l’intégrité de l’historique puis mesure la prochaine sélection ou décision sur
ses originaux exacts. Les appels non encore inspectés gardent leur précontrôle
complet. Un contre-exemple isolé reproduit le refus avant correction ; la
qualification et la nouvelle reprise restent à vérifier.

### Reprise avec message restant mesuré et diagnostic unique — révision 378

Le contre-exemple de capacité passe après correction, avec les variantes de
sélection interrompue, décision prête, refus d’une décision réellement trop grande
et original historique altéré. Le nouveau précontrôle refuse ensuite à raison
le message final réel avant réservation : l’ancien message avait 1154 tokens
disponibles, le diagnostic initial sérialisé deux fois produit un déficit de 3311
tokens. Une mesure locale lit les originaux immuables et la projection reçue,
sans ouvrir de base ni appeler un fournisseur. Réduire seulement le texte de la
consigne ne suffit pas ; supprimer seulement la deuxième sérialisation laisse
encore 1059 tokens manquants.

Le diagnostic détaillait quatre résolutions alors que le parseur s’arrête au
premier écart. La correction garde les quatre originaux possibles et les fenêtres
exactes de 256 caractères, cible ce premier refus et sérialise son diagnostic
une seule fois. Toutes les questions finales et la réponse source intégrale
restent conservées ; l’agent doit examiner chaque réserve. Le message mesuré avec
la consigne courte conserve 681 tokens, sous les réserves de transport inchangées.
Les contre-exemples de duplication et d’accumulation échouent avant correction.

Dix tests principaux et six sous-cas passent avec race sur les sources finales,
dont les parcours opérateur/superviseur après réouverture, l’intégrité des
originaux, les échappements JSON, l’indentation, les capacités réelles et les
messages historiques. Vet, douze contextes de méthodes et le contrôle de diff
passent. Le texte de la reprise publique sous-estime par erreur ce décompte à
neuf tests ; l’original reste conservé et la sortie établit dix. Ces fixtures
n’établissent ni une revue indépendante du moteur ni l’acceptation du pilote.

La reprise est enregistrée publiquement en révision 376, toujours 77 appels ; le
diagnostic attribué est présent dans sa directive. Le conducteur ciblé démarre
sur le binaire 7b16c2c1… après vérification des sources compilées. L’appel final
`fragment-final-58af3c552bf17c850db2fc75` est réservé en révision 378, avec son
empreinte de prompt et la même directive. Le compteur passe à 78 sans remise à
zéro. Cela établit la préparation de l’appel ; sa réponse, le verdict et l’effet
sur T2 restent à vérifier. Le suivi Codex toutes les cinq minutes est mis à jour
avec ce conducteur et ce point de reprise. Le programme reste à 1 lot accepté
sur 8, avec poursuite attendue jusqu’aux extensions et au RETEX.

### Priorité fiabilité et arrêt des relances — révision 402

L’utilisateur refuse le délai et constate à juste titre qu’un conducteur vivant
et des reprises automatiques ne constituent pas la supervision demandée. Codex
reconnaît son défaut de pilotage : la correction précédente a amélioré des
messages et conservé les refus, sans rendre fiable la copie littérale de code et
de JSON par le fournisseur. Les appels 78, 79 et 80 restent des erreurs. Le dernier
appel déjà réservé est laissé aller à son terme ; ensuite la pause publique
révision 402 arrête les nouvelles relances sans effacer les consommations.

L’analyse en lecture seule de l’appel 79 identifie une citation non originale sur
29 résolutions : un LF final ajouté à une sortie i18n capturée dans un JSON.
L’appel 80 échoue sur une autre citation de patch, préfixée par un signe plus
sans l’indentation originale. Aucun de ces avis n’est corrigé ou réaccepté.
La copie libre de chaînes complexes est la frontière à remplacer. Les conventions
seules, même transmises, ne garantissent pas cet échange.

Options examinées : répéter des consignes ou normaliser les citations continuerait
la boucle ou affaiblirait la preuve. La correction retenue doit transmettre des
références structurées vers des plages originales attribuées. Le moteur doit
matérialiser les octets exacts depuis les sources déjà visibles, puis appliquer
les contrôles existants de preuve, périmètre, candidat et décision indépendante.
Ce comportement est une cible, pas une fonctionnalité déjà mise en service.

Plan borné, propriétaire Codex seul :

1. Construire et vérifier un catalogue déterministe, sans perte des originaux,
   avec références vers des extraits contigus, empreinte de catalogue et droits
   de preuve conservés. Rejouer les formes exactes des anomalies observées,
   avec Unicode, tabs, patchs, JSON imbriqué, indices invalides et fraîcheur.
2. Versionner le schéma, le rendu et le décodage des nouvelles réponses. Les
   avis historiques gardent leurs octets et leur contrat ; un nouvel identifiant
   ne remplace jamais une preuve ni une justification indépendante.
3. Vérifier les capacités sur les messages réellement remis et la reprise
   publique après réouverture, sans nouvelle dépense. Garder les originaux entiers,
   l’obligation de résoudre chaque réserve et le refus des preuves de métadonnées
   pour une conformité métier. Ne pas élargir silencieusement les droits.
4. Après qualification, reprendre une vérification native contrôlée, superviser
   réception, décision, application et effet. Une insuffisance chiffrée de budget
   pour cette correction qualifiée relève de l’autorisation utilisateur existante,
   sans remise à zéro ; elle ne justifie pas de relancer le protocole actuel.
5. Revenir ensuite aux huit lots et au RETEX. Une revue ou un test réussi ne suffit
   pas à qualifier la diffusion de l’ensemble.

### Interdiction explicite des attentes silencieuses et des annonces prématurées

Le 15 octobre, l’utilisateur interdit les heures de blocage sans prise en charge,
les attentes silencieuses et les annonces répétées « corrigé » sans résultat
vérifié. La règle impose un point utile au moins toutes les 60 secondes pendant
le travail interactif actif, le diagnostic de toute lenteur anormale et l’absence
de relance identique fondée seulement sur une reformulation ou une hausse de
budget. Elle est inscrite dans les consignes personnelles, `AGENTS.md` et le
contrat commun ; les pages de supervision FR/EN exposent son application.

Le premier contrôle échoue : les contextes planner/subplanner dépassent la
borne technique existante. La mesure donne 32 043 et 32 046 octets. Les doublons
du contrat sont condensés, sans supprimer cette obligation, tronquer un contexte
ou relever sa borne. Le contrôle des douze méthodes et des contextes assemblés
passe ensuite. Ce résultat prouve l’assemblage local des consignes, pas leur
installation dans le conducteur conservé ni la réparation du moteur.

Les tests locaux du nouveau contrat vérifient la matérialisation des octets des
trois formes observées (tabs de patch, JSON et patch multiligne), les références
périmées, les revues simples/par lots et la distinction entre la taille du JSON
reçu et celle des preuves matérialisées. La version de citation 2 reste en cours
de qualification : elle n’est pas encore choisie par la construction des nouveaux
contextes ni installée. Aucune nouvelle revue native n’est lancée ; la pause
publique reste en révision 402 avec 80 appels consommés et un lot accepté sur huit.

### Qualification du contrat de références — reprise du 15 octobre

Codex remplace le contrat de copie libre des nouveaux avis par
`citation_contract_version: 2`, pour les revues simples, par lots et fragmentées.
Les références sont proposées par le schéma ; le moteur reconstruit les octets
originaux et conserve les contrôles de périmètre, de candidat, de critères et de
réserves. Le catalogue partage sans perte les textes communs aux JSON, rapports
et sorties de contrôles. La première projection du dossier natif dépassait la
capacité de 71 713 tokens ; après partage JSON réversible, les 23 pièces choisies
restent entières et la projection garde 21 793 tokens sous les réserves existantes.
Cette mesure lit les preuves historiques : elle ne crée aucun nouvel avis.

Les nouvelles revues fragmentées figent leur méthode et leur préfixe attribués.
Un avis favorable demeure vérifiable après une évolution des consignes. Un refus
sous le protocole 2 ne peut plus être relancé par une reformulation, un délai,
un budget augmenté ou un changement des seules consignes communes. L’évolution
effective du contrat exige une revue complète, sans réattribuer les anciens avis.
Les essais isolés couvrent réception par sous-processus, réouverture, décision
opérateur/superviseur, refus sans dépense, application et acceptation normale du
même candidat après passage de l’ancien contrat au nouveau. Il s’agit de fixtures,
jamais d’une revue indépendante native du moteur ou du pilote.

Deux frontières liées sont contrôlées explicitement : le schéma d’un lot exige
uniquement ses tâches affectées tout en conservant le contexte global ; les erreurs
de protocole des revues simples et par lots sont classifiées et soumises au même
contrôle de précondition que les fragments. Les erreurs initiales des tests et des
mesures sont conservées. Un ancien cas de taille utilisait un texte répétitif
désormais compactable ; la qualification doit distinguer son transport intégral
d’un texte réellement non transportable avant de conclure.

Le binaire de diagnostic non installé `bin/swarm-actions-confiance-catalog-preflight`
a exécuté l’aperçu public sur la révision 402. Celui-ci demande trois inspections
fraîches et deux appels finaux, aucun avis réutilisable, plafond minimal 85 au lieu
de 80 appels consommés. L’aperçu ne réserve aucun appel et ne change pas le travail.
Les deux premières requêtes de lecture de Codex utilisaient des noms de champs
incorrects ; les rejets originaux sont conservés. La requête corrigée utilise
les clés réelles `schema_version` et `task_id` après lecture du DTO et du CLI.
Ce défaut de notre propre méthode ne constitue ni une panne du fournisseur ni
une autorisation de mutation.

La campagne élargie cible 44 tests principaux des contrats communs, avec détection
des courses. Le contrôle statique et l’assemblage des douze méthodes passent ;
aucune conclusion globale n’est tirée avant le résultat complet et le traitement
du cas de taille. Le conducteur antérieur reste en pause, 80 appels sont conservés,
T1 seul est accepté sur huit lots. Le binaire définitif et la reprise native restent
à lier aux sources finales après qualification.

### Frontière Git : octets de bord conservés

Le test de conservation intégrale a ensuite révélé un défaut réel antérieur au
catalogue : `managedGit` appliquait `TrimSpace` aux contenus. Le rapport répétitif
comptait 330 000 octets dans le candidat mais 329 999 dans la revue : son LF final
était retiré. La preuve initiale et son diagnostic sont conservés ; le catalogue
ne pouvait pas restaurer un octet déjà perdu par son producteur.

Le lecteur partagé distingue maintenant métadonnées Git et contenu exact.
Les contextes de citation 2 lisent stdout brut pour les rapports et les diff,
y compris leurs variantes compactes, incrémentales et de réutilisation. La
validation Git de l’avis suit la version enregistrée du contexte. Les avis 0/1
conservent leurs anciennes chaînes et empreintes ; aucun avis historique n’est
réécrit. Stderr ne devient pas une preuve de contenu et la cause d’un échec Git
reste disponible dans le lecteur exact. Le test de rapport intégral et les
acceptations simples/par lots ainsi que la transition publique de version passent
avec race sur cette correction ; les variantes de bord sont vérifiées séparément.

La campagne élargie précédente a exécuté 44 tests principaux : 43 réussis et un
cas de taille fondé sur un texte désormais compactable. Ce résultat échoué reste
conservé, sans le présenter comme PASS. Le cas est remplacé par un texte non
répétitif réellement hors transport ; un test positif distinct exige les octets
intégraux du rapport répétitif dans le contexte reçu. Les deux comportements sont
distincts de la qualification native, toujours non réalisée à ce point.
