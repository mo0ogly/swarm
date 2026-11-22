# FIFO entre trois missions et reprise du conducteur

[English](../en/SAME-ROOT-FIFO-RECOVERY.md) · [Architecture Swarm](OVERVIEW.md#12-concurrence-entre-missions-dans-un-root)

Qualification P1.2 du 22 novembre 2026. Les trois Work partagent un Root
isolé. Le conducteur Web, les routes protégées et les baux de mission sont
réels. Les planificateurs et workers sont des programmes locaux inertes.
Le coût réel est nul. Les réservations estimées sont des valeurs de fixture.

Binaire exercé : `v0.1.1-dev.389`, commit déclaré `50f1daee`, `modified:true`,
SHA-256 `2fa03ed1ffe14d9243f681027085d0e6f61cbc018fd3c4102e85128f00a9567c`.
La qualification est liée à cette image et aux archives ci-dessous.

## Témoin et interruptions du conducteur

| Parcours | Précondition | Effet observé |
| --- | --- | --- |
| Témoin sans crash | A détient `shared` au tour 1. B demande `shared/nested` au tour 2. C utilise `separate` au tour 3. | B attend. C démarre pendant qu’A tourne. Après libération du tour 1, B acquiert le tour 2. |
| Arrêt coopératif | A et C exécutent les workers. B attend A. | `SIGINT` arrête le serveur de fixture. A et C survivent. Après réouverture et libération d’A, B acquiert son tour d’origine pendant que C continue. |
| Crash entre transactions | A et C sont actifs, B est encore en attente. | `SIGKILL` du conducteur et sortie `-9`. La réouverture retrouve le même bloqueur et le même rang. Le nouveau conducteur reprend le bail de B en génération 2 après la péremption du bail. La fixture sauvegarde une valeur de 30 s, sans établir un délai de reprise garanti. B acquiert le tour 2 à la libération d’A, avec un seul Agent. |
| Crash avant le commit d’admission | A a déjà libéré le tour 1. C est actif. L’admission de B est entrée dans la transaction. | Le point d’arrêt est atteint après l’écriture du tour acquis et de l’événement `acquired`, dans `agent_launch::stage`, avant `COMMIT`. Seul le conducteur de fixture reçoit `SIGKILL` et sort `-9`. |

## Relecture durable et reprise après le crash d’admission

Dans l’image testée, le point d’arrêt est `workspace_turn_write::acquire+0xec4`. Le journal `crash/debugger.log` confirme son franchissement après l’écriture de l’événement `acquired`. L’insertion de l’Agent et celle de la réservation suivent ce retour dans `agent_launch::stage`. L’offset dépend de l’image testée.

Avant reprise, A est libéré et B est toujours `waiting` au tour **2**, avec
le même instant de requête. Le journal de B contient seulement `enqueued`.
Aucun Agent ni réservation B n’est enregistré. C reste actif. Ces observations
établissent l’absence d’effet durable de l’acquisition interrompue au point testé.

Après réouverture, le nouveau conducteur de B obtient le bail de génération
**2**. B acquiert le **même tour 2** et conserve l’instant de requête initial.
C garde son PID et son Agent pendant cette admission. La lecture de fin retrouve
trois tours libérés et aucun bloqueur. Pour B, le journal contient exactement
`enqueued → acquired → released` sur le tour 2, avec le même identifiant d’Agent
pour l’acquisition et la libération, et une seule réservation estimée.

Ce scénario qualifie la fenêtre **avant COMMIT** sur cette image. Les
états finaux décrivent les tentatives et la file de la fixture.

## Pendant COMMIT et avant le démarrage après COMMIT

Sur la même image, un témoin sans coupure a terminé. Dans le cas interrompu
pendant COMMIT, l’écriture d’une page avait réussi et le journal SQLite
existait. La première sonde en lecture seule a refusé ce journal chaud. Au
redémarrage normal, B est revenu en attente au tour 2 avec l’horodatage
d’origine, sans Agent ni réservation persistés. Avec un bail de génération 2,
B a acquis puis libéré le même tour, avec une seule réservation estimée.
C a gardé son Agent et son PID. Les trois tours ont été libérés.

Après le retour réussi de COMMIT et avant le lancement, la relecture a
retrouvé B en file d’exécution avec son tour acquis, son Agent et sa
réservation. Le témoin sans coupure a terminé. Après la coupure, le worker
de B est sorti avec le code 2 au redémarrage et le conducteur s’est arrêté
en conservant l’intention active. La reprise de bout en bout reste ouverte.
La perte d’un worker, l’équité en charge et l’inter-Root restent à qualifier.

Le catalogue associe ces résultats aux reçus de
`fifo-commit-crash-20261122.VHnoWq` et conserve le premier échec de
`fifo-commit-crash-20261122.AWWUvx`.

## Origines des preuves

| Archive du 22 novembre | Pièce principale |
| --- | --- |
| `same-root-fifo-20261122.schtsm` | `06-persisted-settlement.json` et `04-sigint-worker-survival.json` |
| `same-root-fifo-sigkill-20261122.gE9V3k` | `06-persisted-settlement.json` et `readback.json` |
| `fifo-admission-crash-20261122.1H4Abh` | `control/08-persisted-settlement.json`, `crash/05-transaction-interruption.json`, `crash/06-crash-durable-readback.json`, `crash/07-b-original-turn-and-c-continuity.json`, `crash/08-persisted-settlement.json` et `crash/debugger.log` |

[Empreintes et portée](../diagrams/architecture-contracts.json) ·
[Écriture de la file](../../rust/crates/swarm-engine/src/workspace_turn_write.rs) ·
[Transaction d’admission](../../rust/crates/swarm-engine/src/agent_launch.rs)
