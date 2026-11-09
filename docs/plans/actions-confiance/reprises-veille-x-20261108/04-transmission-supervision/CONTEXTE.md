# Swarm 4 — Consignes et supervision

Préparé 2026-11-08T21:01:06.651155+00:00. État : réservé pour reprise future, aucun travail exécuté.

## Démarrage différé et droits

Discussion créée pour être retrouvée et reprise par Fabrice lorsqu’il a de la capacité. À la création : ne pas lancer d’analyse, de commande ou de sous-agent ; accuser réception et attendre. Une future demande « reprends le cadrage » autorise lecture et documentation dans ce seul dossier d’axe. Elle n’autorise pas développement, test, build, installation, service, provider, scan, changement des droits/budgets ou publication. Une mise en œuvre demandera un périmètre explicite, puis les règles de livraison applicables.

Au démarrage réel, lire AGENTS.md, tools/agent-workflows/CONTRACT.md, la skill replan et la méthode de cadrage pertinente. Résoudre /home/fpizzi/workspace/swarm ; checkout canonique /home/fpizzi/Documents/ChatGPT/swarm. Lire CONCURRENCE.md dans le dossier parent. Relever HEAD, état dirty et activités concernées sans arrêter autrui. Les constats ci-dessous sont un index documentaire du 8 novembre, pas une preuve actuelle de comportement. Relire doc/code et preuves originales avant toute affirmation ; distinguer documentaire, code observé, effet vérifié, inconnu.

## Remise attendue après reprise autorisée

Un fichier ANALYSE.md dans CE dossier : besoin/source X, état réel et preuves datées, couverture existante réutilisable, inconnues, plus petit écart justifié, scénario témoin/variante/reprise, dépendances et fichiers candidats. Avant défaut injecté prévoir un témoin valide. Ne pas exécuter le scénario. Conserver erreurs, portée et conditions de reprise. Aucun taux de confiance inventé. Un article futur exige une nouvelle relecture des preuves et une autorisation de publication.

## Sujet

X-IDEE-12, recoupements POC-04/06 et X-IDEE-06/11. Préserver destinataire, portée et autorité d’une consigne via les relais. Distinguer remis/reçu/appliqué et statut bloqué réellement diagnostiqué. Son témoignage n’établit pas un défaut Swarm.

Sources X : Kaelan https://x.com/kaelanyiu/status/2119504309708882341. Textes sources historiques dans docs/communication/x-swarm-20261107/lot14-x-sources.json, lot15-x-sources.json, lot16-x-sources.json et registres des relances.

## État à revalider

Piste conservée, code/runtime de la variante non qualifiés par notre veille. Ne pas confondre arrêt demandé avec annulation des effets déjà engagés ni créer de nouveau protocole sans examiner l’existant.

## Lectures ciblées, pas inventaire exhaustif

- /home/fpizzi/workspace/swarm/docs/architecture/AUTORECURSIVE-RECOVERY.md
- /home/fpizzi/workspace/swarm/docs/architecture/RUN-SUPERVISION.md
- /home/fpizzi/workspace/swarm/tools/agent-workflows/CONTRACT.md
- /home/fpizzi/workspace/swarm/docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md — X-IDEE-12

## Scénario proposé

A/B autorisés fonctionnent ; stop A via deux relais, relais déforme la portée ; aucune nouvelle action A après application effective, B valide, effets engagés conservés ; reprise A sous décision distincte. Aucun test exécuté par cette préparation.

## Périmètre de sortie

Écrire seulement ici. Signaler au coordinateur les changements proposés aux plans, recettes ou sources partagés ; ne pas les effectuer.


## Priorité à la stratégie et aux propriétaires existants

Lire ../STRATEGIE.md et ../DISCUSSIONS.json avant reprise. Ce dossier est une réserve, pas un remplacement des discussions existantes. Identifier le propriétaire et le checkout actuels, rapprocher les résultats déjà obtenus et ne pas relancer leur travail. Une évolution d’état postérieure à cette préparation prime sur les constats historiques.
