# Swarm 3 — Timeouts et réconciliation

Préparé 2026-11-08T21:01:06.651155+00:00. État : réservé pour reprise future, aucun travail exécuté.

## Démarrage différé et droits

Discussion créée pour être retrouvée et reprise par Fabrice lorsqu’il a de la capacité. À la création : ne pas lancer d’analyse, de commande ou de sous-agent ; accuser réception et attendre. Une future demande « reprends le cadrage » autorise lecture et documentation dans ce seul dossier d’axe. Elle n’autorise pas développement, test, build, installation, service, provider, scan, changement des droits/budgets ou publication. Une mise en œuvre demandera un périmètre explicite, puis les règles de livraison applicables.

Au démarrage réel, lire AGENTS.md, tools/agent-workflows/CONTRACT.md, la skill replan et la méthode de cadrage pertinente. Résoudre /home/fpizzi/workspace/swarm ; checkout canonique /home/fpizzi/Documents/ChatGPT/swarm. Lire CONCURRENCE.md dans le dossier parent. Relever HEAD, état dirty et activités concernées sans arrêter autrui. Les constats ci-dessous sont un index documentaire du 8 novembre, pas une preuve actuelle de comportement. Relire doc/code et preuves originales avant toute affirmation ; distinguer documentaire, code observé, effet vérifié, inconnu.

## Remise attendue après reprise autorisée

Un fichier ANALYSE.md dans CE dossier : besoin/source X, état réel et preuves datées, couverture existante réutilisable, inconnues, plus petit écart justifié, scénario témoin/variante/reprise, dépendances et fichiers candidats. Avant défaut injecté prévoir un témoin valide. Ne pas exécuter le scénario. Conserver erreurs, portée et conditions de reprise. Aucun taux de confiance inventé. Un article futur exige une nouvelle relecture des preuves et une autorisation de publication.

## Sujet

POC-03, X-IDEE-07. Séparer intention, émission, timeout, effet, attribution et décision de reprise. Conserver événement original et réconciliation additive ; une clé ou état identique ne suffit pas à prouver causalité.

Sources X : Mikhail https://x.com/i_mika_el/status/2119462379687997652 et https://x.com/i_mika_el/status/2119459613724578163 ; Aiden https://x.com/open_is_free/status/2119399556341461439. Textes sources historiques dans docs/communication/x-swarm-20261107/lot14-x-sources.json, lot15-x-sources.json, lot16-x-sources.json et registres des relances.

## État à revalider

Plan et inventaires déjà largement cadrés, BF-L0-A contient des résultats sur miniatures ; ne pas les confondre avec qualification des argv natifs ou d’un service externe. Ne pas étendre le périmètre à un paiement réel.

## Lectures ciblées, pas inventaire exhaustif

- /home/fpizzi/workspace/swarm/docs/plans/AGENT-CONTROL-POCS.md — sections Reprise X-IDEE-07, POC-03-F/03-B et BF-L0-A
- /home/fpizzi/workspace/swarm/docs/architecture/PAEA-GIT-TRANSACTION.md
- /home/fpizzi/workspace/swarm/docs/architecture/TRACEABILITY.md

## Scénario proposé

Témoin effet/reçu liés ; réponse perdue après commit, tiers produisant état identique, clé avec payload différent, notification tardive ; attribution inconnue sans preuve fiable, reprise sans double effet. Aucun test exécuté par cette préparation.

## Périmètre de sortie

Écrire seulement ici. Signaler au coordinateur les changements proposés aux plans, recettes ou sources partagés ; ne pas les effectuer.


## Priorité à la stratégie et aux propriétaires existants

Lire ../STRATEGIE.md et ../DISCUSSIONS.json avant reprise. Ce dossier est une réserve, pas un remplacement des discussions existantes. Identifier le propriétaire et le checkout actuels, rapprocher les résultats déjà obtenus et ne pas relancer leur travail. Une évolution d’état postérieure à cette préparation prime sur les constats historiques.
