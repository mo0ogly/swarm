# MEM-P1 — suite qualifiée de M33

Origine : matrice M32, reprise demandée le 17 octobre 2026. Propriétaire de
l’objectif : Codex, mémoire/Store ; autorité protégée au chantier identité.

Les 19 appels booléens et sept gardes directes sont dans consumer-inventory.json.
La matrice n’assimile pas lecture cohérente, identité attestée et droit d’usage.

| Suite conservée | Valeur / critère observable | Propriétaire et état |
| --- | --- | --- |
| kernel_attempt_source_linux.go : pause jointe et enveloppes Agent/Work | La requête refuse déjà une pause non booléenne ; qualifier aussi l’identité SQL/JSON et un diagnostic sans valeur privée. Établir un témoin valide puis injecter divergences et pause illisible, aucune observation recevable ni agent inventé. | Store pour cohérence, coordination identité pour autorité ; OUVERT, source observée et pas de correction dans M33. |
| lifecycle.go : prédicat de supervision sur pause/autonomie | Le SQL n’accorde pas l’autorité si la pause est non nulle. Distinguer absence légitime, état inconnu et ligne incohérente sans fabriquer un heartbeat actif. Témoins, incident reçu, refus puis reprise exigés. | Store/supervision ; OUVERT, à traiter avec les enveloppes liées. |
| Replis/vues M29/M31 : conduct/reconcileKnownMissionResult/retryIndependentReview, compléments missionCLI, console_panels/dialogKey, aide et visite | Erreur attribuée ou enrichissement facultatif explicite ; pas de succès incomplet. Les lectures de pause de ces vues ont été raccordées, leurs autres données ne sont pas qualifiées par M33. | Codex ; OUVERT, prochaine tranche de P1. |
| Enveloppes Agent et dérive pendant probe M30–M32 | Relier clés SQL, WorkID et JSON ; injecter un remplacement entre lecture et réservation, refus sans agent/réserve puis reprise valide. Préserver les droits de création. | Store/mémoire avec identité ; OUVERT. |
| Recorders d’usage void M32 | Reproduire une écriture de consommation refusée et suivre son diagnostic au consommateur, sans remboursement ni réussite vide. Aucun dépassement budgétaire prouvé. | Moteur/budgets ; hypothèse OUVERT, fixture d’écriture refusée comme reprise. |

Pour les deux lectures jointes, modifier seulement le booléen serait insuffisant :
il faut traiter l’enveloppe et son autorité. Comparaison retenue : migration locale
immédiate des 26 lectures simples contre refonte d’autorité non spécifiée. La
première est réalisée ; la seconde reste attribuée avec son critère, sans réclamer
un droit ou une conformité supplémentaire. Les gardes existantes ne sont pas retirées.

Leçons : mesurer les appels dans les sources d’origine ; distinguer appel et garde ;
refuser un patch avant écriture si sa cardinalité diverge. Préserver la source au
premier rouge et établir un témoin avant injection. Dans un contexte partiel,
Missing doit nommer le Work et le fait manquant ; le départ reste indisponible.
Ne pas transformer l’erreur d’un cycle en effacement de son état antérieur.

Ces leçons sont persistantes et liées depuis le plan. Aucun contexte d’agent déjà
chargé ni processus en cours n’est renouvelé par cette note. MEM-P1 reste ouvert ;
P2–P5 et les droits M20 ne sont pas livrés par M33.
