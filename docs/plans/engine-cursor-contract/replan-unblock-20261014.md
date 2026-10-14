# Replan — débloquer la mission conservée, 14 octobre 2026

Demande utilisateur : traiter le blocage visible. Source propre c8f1069, serveur
identique. Mission conservée r839 en pause, T2/T3 bloquées, T4–T8 en attente.
Les développements directs et le RETEX livré ne remplacent pas ses acceptations.
Les originaux publics sont dans le cache opérateur existant, préfixe
`replan-native-unblock-20261014`.

## Diagnostic et décision

1. `managedScopeFiles=100` refuse 112 fichiers T2 et 272 fichiers T3. Cette
   politique non réglable contredit le contrat Admin. Rendre la limite configurable
   est préféré à réduire arbitrairement les fichiers : aucun original omis.
   Une décomposition de revue reste possible, décidée par le responsable.
2. La lecture publique T2 qualifie à tort son manifeste valide de 28 sources
   d’invalide : un seuil fixe de 24 sources est mêlé au schéma. Ses 175190 octets
   dépassent aussi le plafond fixe128 Kio. Corriger les politiques ne supprime
   pas son ancien refus sémantique.
   T3 fournit un inventaire attribué, mais pas une reprise de revue interrompue.
3. Les tentatives sont consommées et restent telles quelles. Pas de reset,
   d'acceptation manuelle sans revue ni de relance inchangée. Réemployer les
   résultats existants avec réparation attribuée, nouveaux contrôles et revue,
   plutôt que redévelopper ce qui existe déjà dans la livraison directe.

## Séquence et critères

| Étape | Propriétaire | Effet vérifiable |
| --- | --- | --- |
| Politique de fichiers | Codex direct | Même limite effective persistée via quotas CLI/API et modale Admin ; 0 sans plafond, défaut ancien100 visible ; aucune seconde limite dans un lot ou un patch |
| Régression | Codex direct | Refus avant appel au défaut100, aperçu sans mutation, seuil supérieur100 et0, négatif/périmé refusés, réouverture identique ; consommateurs scope/recovery/coordination cohérents |
| Livraison | Codex et revue indépendante déjà autorisée | Documentation FR/EN, navigateur FR/EN/thèmes, commit/push et version servie ; missions/historique conservés |
| Reprise des résultats | Opérateur puis propriétaire racine | Diagnostic du manifeste, correction générale/attribuée, décision publique reçue, contrôles/revue sur candidat exact, effet observé ; aucun Claude pour développer le moteur |
| Clôture | Superviseur | Fin uniquement après preuves fraîches des exigences ; distinguer levée d'un verrou et mission acceptée |

Les budgets/droits peuvent être augmentés dans l'autorisation existante après
shortfall constaté, avec valeurs persistées et consommation inchangée. Le modèle
configuré et les limites fournisseur restent opposables. L'ancien refus et le
candidat original ne sont jamais modifiés pour fabriquer une réussite.

## Vérification et replan de la correction

La revue indépendante a détecté trois consommateurs résiduels : plafonds de lots
24/96/128, refus individuel non reprenable, revalidation des contextes historiques
sous le réglage actuel. Un rouge reproduit la rétro-invalidation (politique1
contre28 pièces figées). Correction : les nouveaux contextes appliquent la
politique persistée ; les transports/historiques vérifient attribution et intégrité,
sans réappliquer une politique ultérieure. Les refus individuels sont typés.
Le diagnostic connexe utilise aussi les réglages de sources.

Une erreur de recette comparait maps vides et nil : JSON persisté identique,
aucune mutation moteur. La recette compare maintenant deux lectures persistées.
Un processus de tests a été interrompu143 : preuve conservée, jamais PASS ;
la reprise ciblée du cas interrompu passe. Aucun nouveau lancement de suite entière.

## Livraison et premier effet natif

47583ac est poussé et installé par le parcours natif supporté. Les quatre missions,
leurs états de départ, compteurs, dépenses et candidats sont conservés ; santé ready,
version injectée47583ac non modifiée. Recettes ciblées : packet10 tests principaux,
reprise publique1 avec2 variantes, batches figés5 avec11 variantes, manifeste1 ;
paquets PASS, race sur comportement. Vet et workflows passent. Admin FR/EN ×2 thèmes,
390px, aperçu non mutateur, sauvegarde/réouverture et0 : PASS, aucun appel modèle.
Revue source indépendante favorable après correction de trois P2 ; ceci ne vaut
pas verdict natif de mission.

Réglages publics :300 fichiers,196608 octets par source,524288 octets par contexte.
Un premier seuil de32 sources restait insuffisant pour T2 : son manifeste valide
28 se complète de la livraison, union35 sources /214328 octets /maximum49079.
La mesure sur le candidat88e76a est conservée ; seuil40 persisté, consumption
inchangée. Les aperçus scope T2/T3 ne requièrent plus de réduction ; capacité T2
4 paquets +2 finaux et T3 7 paquets +2 finaux sous18 appels disponibles. Demande
publique de reprise T2 reçue à r842, original et avis précédents conservés.
Mode manuel pour les résultats conservés, aucun nouveau worker ni planificateur.
La réception, l’exécution et le verdict natif restent des faits distincts à suivre.

RETEX de méthode : lire le schéma producteur avant un POST ; la query GET utilise
`task`, la requête PlanningRequest utilise `task_id`. Un POST malformé a été refusé
avant mutation, puis corrigé selon le schéma. Calculer la déclaration complète,
pas seulement le manifeste. Vérifier le dernier consommateur et le contexte figé,
pas seulement le collecteur ou les consignes.

## Replan du conducteur

La reprise T2 reçue r842 ne produisait aucun effet. Cause reproduite : un producteur
completed déjà relayé sort de settleAgentTask ; reconcileKnownMissionResult ne
reprenait que les résultats récupérés d’agents interrompus. La nouvelle intention
publique restait integrating sans contrôle ni appel. Correction générale : traiter
integrating pour la tentative courante completed, ou interrompue avec résultat
récupéré attribué. Gardes d’intégration habituelles et relais original conservés.
Rouge `completed-retry-red.jsonl` : statut blocked et0appel malgré décision reçue.
Vérifier normalité, rejeu, budget, pause et remise inchangée avant livraison.

## Autorisation locale du règlement manuel

L’annonce initiale sur le mode manuel était trop large. Observation native en
lecture seule : intention integrating mais événement `settle-error` : mission
continue non autorisée. La garde d’intégration exigeait le mode autonome, qui
relancerait ici la planification Claude. Aucun appel de revue dépensé.

Solution bornée : un consentement local, jamais exporté avec Work, émis dans la
transaction d’une reprise publique et lié au résultat, base, contrat, tentative
et permission locale de mission. Le mode manuel reste tel quel ; aucune nouvelle
production. Cette permission est contrôlée à l’entrée et dans la publication.
Une revue a détecté l’utilisation du Work pré-revue à cette dernière frontière :
rouge `planning-pause-red.jsonl`, puis capture de la planification actuelle de la
transaction et contrôle de pause des callbacks ordinaires et par lots.
Les versions7ff3a92 et précédentes restent historiques ; la nouvelle autorisation
et son effet natif restent à observer après qualification et installation.

## Effet manuel observé puis protocole invalide

079e9fd installé, reprise publique reçue r845 en mode manuel. Aucun nouveau worker ni activation : la revue native T2 consomme trois appels, compteur85/100 à r854. Le journal conserve les paquets0 et1 unknown, puis2 interrupted : `defect without unique failed artifact`. Les trois allégations sont associées à inspected, aucun fail ; la réponse originale ne constitue pas un verdict. Candidate88e76a inchangé, basee32388f inchangée et quatre tentatives T2 conservées. Original, paquet, diagnostic indépendant et compteurs sont dans la zone de vérification existante.

Diagnostic indépendant : trois P2 présents également dans le canonique079e9fd. (1) Le helper de qualification D03 ne confine pas les parents symboliques et sa forme plate lit sans confinement ; ce constat concerne le helper, aucune fuite serveur démontrée. (2) Le judge public direct exempte des entrées qu’il ne valide ni acquitte. (3) Le test pause partage un quota déjà épuisé et n’identifie pas le refus. Deux défauts comportementaux sont reproduits en rouge ; un test isolé établit le refus de pause puis la réussite de la même reprise avec quota disponible.

Choix : corriger ces contrats généraux et le diagnostic de reprise, plutôt que convertir le JSON en verdict ou rejouer un candidat inchangé. Le judge direct refuse Inputs ; decide→judge conserve validation et acquittement atomique. Les lectures D03 utilisent un handle enraciné pour pièces et sources. La revue de cette correction détecte le risque FIFO : Open en lecture peut bloquer avant Stat ; reproduire puis préserver le rejet non bloquant des fichiers spéciaux. Le feedback historique reste distinct du contrat citation1 autorisant plusieurs défauts distincts.

Critères encore ouverts : qualification ciblée finale, livraison supportée, résultat T2 corrigé attribué et contrôlé, nouvelle revue native indépendante, puis récupération T3 et besoin initial/RETEX. Une correction canonique ne modifie pas le résultat historique ni son acceptation.

## Résultat corrigé attribué, seconde anomalie du protocole

d853a3c poussé et installé propre ; quatre missions et trois préparations conservées. Delta repris dans la copie existante T2 :14 tests principaux et12 sous-cas PASS avec race16,469s ; ancien test sans événement explicite FAIL de précondition conservé. Docs historiques corrigées : cette copie acquitte par decide puis juge directement, tandis que le canonique propose aussi decide→judge atomique. Aucun faux parcours déclaré. Union40 sources /375462octets mesurée ; résultat5f7c6e0, arbre5557271d reçus par revise-recovered-result, même agent et tentative. Revue native du candidat34f03f démarre15:16UTC puis échoue15:19UTC, trois appels supplémentaires, compteur88/100 àr865 : citation de la pièce25 quote absent. Aucun verdict validé, basee32388f inchangée.

La réponse invalide contient deux nouvelles allégations. Diagnostic indépendant : doublons des entrées plates indexés par chemin brut, confirmés ; admission d’un JudgeState incohérent, confirmée. MaxCycles0 est légitime dans le canonique, Judge nil est compatible legacy : ces deux aspects de l’allégation sont écartés. Deux reproductions rouges ciblées puis correction : doublons par clean ; validation commune du juge à l’admission/import, sans plafond caché.

Défaut lié de reprise reproduit : le décodeur refuse une citation absente avant fragmentRetryFeedback, qui renvoyait vide. Correction : vérifier séparément les identités de l’original immuable et du paquet, transmettre un diagnostic borné, jamais un verdict réparé. Test rouge/vert inclut autre candidat et réponse altérée. Les analyses ne justifient pas une répétition identique ni l’effacement des six appels ; une nouvelle reprise exige des préconditions corrigées et des preuves nouvelles.

RETEX de méthode : un contrôle de la copie conservée a été lancé une fois par erreur dans le canonique ; sa sortie nommée T2-candidate-final est conservée mais ne qualifie pas le résultat natif. Le reçu utilisable est T2-retained-candidate-final, avec cwd et hashes vérifiés. Une transposition de patch ayant du contexte différent a été diagnostiquée avant tests, sans rejet forcé ni nouvelle copie. Vérifier cwd, producteurs et noms trouvés par rg avant chaque commande ; les recherches de noms supposés qui échouent constituent un défaut de notre propre méthode.

Qualification canonique de cette deuxième boucle :34 tests principaux et53 variantes PASS14,423s ; import8cas sans mutation partielle, cycles produits par judge/renew, quota0 sans plafond et legacy nil conservés. Une ancienne fixture de quota injectait Cycle3 sans historique : son échec est conservé, puis fixture corrigée par deux transitions publiques. Une erreur de fermeture de test a causé un FAIL setup sans aucun test exécuté ; corrigée puis reçu qualified distinct. Vet du paquet, workflows12skills et diff check passent. Revue source indépendante favorable, sans verdict natif. Corrections critiques séparées :9e747fa feedback, a44dc5c admission du juge, fe3e04f aliases plats ; manifeste de livraison dépend de ces trois deltas.

Un verrou Git temporaire a refusé les premiers ajouts/commits ; aucun verrou supprimé. Après observation de sa disparition, les index/diffs réels ont été vérifiés puis les commits manquants exécutés, sans répéter un état inchangé. Une séquence de mutation doit s’arrêter au premier échec pour éviter de poursuivre avec un index incomplet. Installation puis nouveau résultat T2 attribué et revue native restent nécessaires ; compteur88, six appels de réponses invalides conservés.
