# Orchestration séquentielle — nuit du 8 au 9 novembre 2026

Autorité humaine : lancer les axes utiles cette nuit sans carambolage, travail sur
main, commit et push des lots vérifiés, sans nouvelle branche. Fenêtre retenue
jusqu’au 9 novembre à 08 h Europe/Paris ; ne pas lancer de nouveau lot après.
Ne pas interrompre une opération en cours à cette heure : laisser une remise sûre,
conserver état et indiquer la suite. Ce n’est pas un timeout de vérification.

## Condition initiale réellement observée

main b4e07cb26066cffdb20b36ac2fc9c60f758cb23e, suivi local 0/0 ; beaucoup de dirty
non attribuables globalement et index vide. Gi.eth et Design UX actifs dans le
même checkout ; demandes de remise après leur lot courant envoyées. Pas de nouvel
exécutant tant que leurs activités conflictuelles ne sont pas libérées. Les autres
automations PAEA et facturation gardent leurs droits distincts et leurs travaux
existants ; coordonner toute future mutation de sources/notes communes.

GitHub API lue : enforce_admins=true, PR obligatoire, checks/install stricts.
Le push direct de nouveaux commits main ne peut pas être promis. Pas de force,
suppression de protection, nouvelle branche ou merge aveugle. Les commits locaux
vérifiés restent distingués des commits poussés. Si publication refuse, conserver
les commits et la cause ; poursuivre les analyses indépendantes et le bilan,
mais ne pas empiler de nouvelles modifications moteur derrière un lot bloqué.

## Ordonnancement

Le coordinateur est le chat de communication/recherche courant. Il lit NUIT-QUEUE.json, les messages récents et états réels.
Les états et noms ne prouvent pas la libération : vérifier sorties, ressources,
processus concernés et remise du propriétaire sans lancer un job dupliqué.
Un seul travailleur de nuit mutateur à la fois, avec son ID de tâche, droits,
fichiers/contrats exacts, ressources, livrable, preuves et condition de remise.
Ce choix est une consigne d’organisation pour cette nuit, pas une politique codée
ni une limite de capacité Swarm mesurée. Le registre ne fait pas office de verrou.

Reprendre d’abord un worker existant déjà désigné, jamais un autre parce qu’il
semble lent. wait_threads compact, diagnostiquer absence de progression, sans
interrompre autrui ou répéter un échec inchangé. Les réserves A1/A2/A3 ne sont pas
lancées comme doublons. A4 apporte l’axe de consignes ; les autres propriétaires
historiques sont réutilisés selon la file. Un chat actif avec une autre mission
ne reçoit pas un nouveau développement avant sa remise.

## Exécution autorisée et limites

Readme/STRATEGIE et skill swarm-research restent applicables ; cette autorisation
nocturne remplace le report seulement pour les lots bornés attribués. Commencer
par existant et preuve ; ne réaliser qu’un écart démontré, réutiliser recettes.
Aucun gros redesign, nouvel accès, hausse de budget, appel fournisseur réel,
scan externe, paiement ou publication sociale implicite. PAEA conserve sa
restriction de développement par GLM ; ne pas la contourner via un autre axe.
Les qualifications vrai modèle déjà bloquées ne sont pas relancées par défaut.

Fixture distincte, témoin sain puis variante et reprise ; entrées figées pendant
contrôle. Si les préconditions manquent, conserver cadrage et action concrète.
Pour un changement livré : tests adaptés, revue explicite politiques/fonctions/
erreurs liée aux octets, documentation FR/EN/version, installation locale par
parcours pris en charge et vérification réelle, sous droits applicables.
Ne pas installer un ensemble de dirty non qualifiés pour livrer un petit lot.
Une auto-revue ne devient pas indépendante parce qu’un autre chat existe.

Commit uniquement les fichiers/hunks attribués, vérifiés et stables ; pas git add .,
pas de reset/stash global, pas de checkout/merge destructeur, pas de commit du
travail d’autrui sans remise et preuve. Inspection index avant/après. Push normal
uniquement après contrôles applicables ; refus enregistré, jamais réussite inférée.
Conserver les origines, SHA/dirty effectif et reçus ; préserver sources/archives.

## Clôture

Mettre à jour la file atomiquement à chaque remise. Bilan lisible : réalisé,
vérifié, installé, committé, poussé séparément, blocages/critères ouverts et preuves.
L’article futur raconte discussion → recherche → preuve/limites, pas de garantie
inventée ; brouillon seulement. Local : ordinateur et application doivent rester
disponibles, une automation n’est pas une exécution cloud ni une garantie de suivi.

## Plan directeur ultérieur — décision du 8 novembre

Avant toute attribution, consulter [FILE-APEX](../../FILE-APEX.md) et
[REPRISES-PAR-LOTS](../../REPRISES-PAR-LOTS.md) : les chats sont des lots d'un
programme commun, pas une autorisation de relancer tous les plans. Facturation
explicitement différée à la fin, pour plus tard, suivi PAUSED : aucune attribution
nocturne. Le cloud est de côté. Cette organisation ne change pas les droits de
PAEA/GLM ni les périmètres des propriétaires déjà actifs.
