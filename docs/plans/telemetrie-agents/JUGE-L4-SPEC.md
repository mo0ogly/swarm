# Juge L4 par modèle — spécification proposée (non adoptée)

Statut : **proposé**, 22 octobre 2026. Auteur : Claude (auto-revue seulement).
Propriétaire de la réalisation : **Codex** (chemin de lancement et budget du moteur).
Contexte : LangWatch L4 « monitors ». Les évaluateurs déterministes sont livrés
(`attempt_evaluations.rs`, réglage v37). Le juge par modèle ne l'est pas : un appel
payant ne peut pas partir du serveur web sans passer par les portes du moteur.

## Problème

L'opérateur veut un avis qualitatif sur une tentative terminée (« les critères de
la tâche sont-ils remplis ? ») sans relire chaque transcript. Seul un modèle peut
le donner ; il coûte, il peut se tromper et il ne doit voir que ce qui est permis.

## Exigences

- **JUGE-R1** Quand une tentative terminée est échantillonnée et que le juge est
  activé, le moteur doit préparer **une** exécution de juge, liée à cette tentative,
  par le chemin de lancement existant : préparation persistée, portes de fraîcheur,
  d'identité et de processus, `agent run`. Aucun lancement depuis le serveur web.
- **JUGE-R2** Avant le lancement, le moteur doit réserver le coût estimé dans un
  registre de budget de la mission (nouveau registre ou registre existant, décision
  ouverte D2). Budget épuisé → refus explicite `judge-budget-exhausted`, aucune
  exécution, la tentative reste « non jugée ».
- **JUGE-R3** Entrée du juge : critères et livrable de la tâche, issue et résumé
  déterministe de la tentative (évaluations L4, résultats de scénario L6). Le
  transcript ou la sortie d'outils n'est transmis qu'avec un réglage explicite
  distinct (D3), désactivé par défaut.
- **JUGE-R4** Sortie attendue : JSON strict `{"verdict":"pass|fail|unsure","score":0..1,"reasons":[code…]}`.
  Toute autre forme → `judge-reply-invalid`, conservée comme échec, jamais
  interprétée. Aucun texte libre du modèle n'est stocké (les `reasons` sont des
  codes d'une liste fermée).
- **JUGE-R5** Une tentative est jugée au plus une fois par version de consigne de
  juge ; une reprise après échec technique réutilise la même préparation
  (idempotence par identifiant d'événement).
- **JUGE-R6** Le résultat est un évaluateur `model-judge` de la table `evaluations`
  (magasin console v6), avec provenance « déclaration de modèle, non authentifiée ».
- **JUGE-R7** Annulation : arrêt de l'exécution du juge par la commande d'arrêt
  existante ; réservation libérée par le chemin de réconciliation existant.

## Réglages Admin (version à réserver par Codex)

| Réglage | Défaut | Null |
|---|---|---|
| `agent_judge_provider` (fournisseur enregistré) | null | juge désactivé |
| `agent_judge_sample_percent` (1–100) | 10 | — (exigé si fournisseur) |
| `agent_judge_reserve_usd` (réserve par jugement) | 0,05 | refusé si fournisseur |
| `agent_judge_include_transcript` | false | — |

## Refus et états visibles

`judge-disabled`, `judge-budget-exhausted`, `judge-provider-unavailable`,
`judge-reply-invalid`, `judge-stopped`. Le harnais affiche « non jugée » /
« jugement en cours » / verdict, avec le code d'un refus.

## Tâches (Codex)

1. Registre de réservation (D2) et contrôle d'admission.
2. Préparation « juge » (rôle `judge`, consigne versionnée, entrée JUGE-R3).
3. Décodeur strict JUGE-R4 et écriture JUGE-R6.
4. Réglages + catalogue FR/EN + validation.
5. Déclencheur : boucle d'alertes serveur (v35) → demande de préparation au
   moteur (pas d'exécution directe).
6. Recette : faux fournisseur (verdict valide, réponse invalide, budget épuisé,
   arrêt), aucun appel payant.

## Décisions ouvertes

- **D1** Déclencheur : boucle serveur ou commande CLI explicite (`swarm-rust judge run`).
- **D2** Registre : `reservations` existant (rôle `judge`) ou nouveau registre.
- **D3** Transmission du transcript au juge : jamais, ou réglage explicite.
- **D4** Liste fermée des codes `reasons`.

## Vérification attendue

Témoin valide avant chaque refus ; budget réservé puis libéré mesuré ; aucune
chaîne de prompt ou de sortie dans `.swarm/telemetry.db` ; recette PASS sur
instantané figé ; revue indépendante avant acceptation.
