# E6 — reprise APEX du banc de recette

## Contrat et choix

Conserver E6 et ses critères : décisions réelles des responsables, défaut métier
injecté, refus observable, correction bornée, contrôle et revue du même candidat,
acceptation fraîche et fermeture des périmètres. Aucun état de mission édité.

Deux options examinées : continuer le script temporaire de la cinquième tentative,
ou maintenir le banc versionné existant. Le script temporaire décide lui-même la
délégation et perd sa recette en fin de producteur. Le banc versionné conserve les
décisions dans le moteur et contrôle les preuves : il devient l’entrée de référence.

## Séquence et responsabilités

1. Implémentation du banc (Codex) : interruption SIGTERM/SIGINT, publication atomique
   des observations, nettoyage, refus de réutilisation du dossier.
2. Contrôle local (Codex) : régression sur ancien code, tests de cycle de vie,
   campagne CLI réelle avec fournisseur déterministe. Aucun appel IA.
3. Essai IA distinct : responsables et producteurs du banc, dans les limites
   enregistrées. Observer les preuves sans décider à leur place.
4. Vérificateur indépendant : examiner le candidat et ses contrôles. Les tests
   locaux et l’auto-relecture ne valent pas cette acceptation.

## Entrée unique

`tests/controlled_autonomy_campaign.py prepare DOSSIER_NEUF --engine BINAIRE --providers CONFIG --provider claude`

La préparation ne lance aucun modèle. Elle fige le moteur, les outils et les
limites. `run DOSSIER_NEUF` lance la campagne au premier plan ; ne pas la mettre
en arrière-plan sous un producteur éphémère ou utiliser un réveil différé.
Un dossier contenant `run-started.json` ne peut pas être relancé.

## Preuves attendues

| Exigence | Preuve |
|---|---|
| Arrêt observable | result.json FAIL avec interruption, jamais PASS |
| Nettoyage | commandes publiques d’arrêt, agents/revues terminés avant serveur |
| Écriture interrompue | ancienne version JSON conservée, pas de fichier tronqué |
| Pas de double dépense | seconde exécution refusée avant toute commande |
| Autonomie réelle | décisions des responsables, injection/refus, SHA identiques, périmètres clos |

SIGKILL et perte machine ne sont pas interceptables : un résultat RUNNING ancien
ne prouve aucune activité ni réussite. Un nettoyage non confirmé reste FAIL et
nécessite examen des processus. Les preuves de l’essai historique ne qualifient
pas une nouvelle version. Aucun nouvel essai IA n’est compté comme effectué ici.

## Validation réalisée

- Ancien code : régression reproduite dans un sous-processus isolé, sortie -15
  sur SIGTERM ; nettoyage non exécuté.
- Nouveau code : 3 tests de cycle de vie passent, couvrant SIGTERM, SIGINT,
  interruption clavier, erreur d’observation, arrêt non confirmé et écriture
  atomique échouée. Rejeu refusé avant nouvelle commande.
- 11 tests controlled_* passent (22,197 s), dont campagne publique utilisant
  le binaire Swarm installé et un fournisseur déterministe. Aucun modèle réel.
- `node tests/engine_acceptance.cjs --case real` : succès. Tests déterministes.
- `git diff --check` : succès. Auto-relecture effectuée, pas de revue indépendante.

La mission reste non validée sur E6. Ces résultats concernent le banc de recette
versionné ; ils ne remplacent ni un essai IA sur le candidat ni sa revue.
