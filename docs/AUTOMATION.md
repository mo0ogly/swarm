# Automatisation durable

## Service disponible dans le moteur

Le moteur possède un service interne de demandes de reprise durables, de
programmes horaires et une réception HTTP locale d'événements externes. Le CLI
et l'administration web des programmes appartiennent au lot C04.

Une demande C01 contient :

- une clé d'idempotence stable choisie par l'appelant ;
- une source bornée, une mission cible existante et l'action unique
  `request_resume` ;
- un digest SHA-256 du contenu canonique ;
- une identité de demande et une identité d'occurrence distinctes.

Les tables transactionnelles du `Store` sont l'unique autorité. Un rejeu avec la
même clé et le même contenu retourne l'enregistrement existant ; la même clé avec
un contenu différent retourne `idempotency_conflict` sans seconde occurrence.

## Autorisation, attribution et attente

La réception exige une autorisation locale explicite de la mission. La prise en
charge la revalide, ainsi que l'autonomie, la pause, les plafonds de planification
et la clôture de la cible. Une mission clôturée est enregistrée comme rejetée avec
`terminal_target` ; aucune mission ni tentative n'est créée.

Une occurrence est attribuée par bail à un seul conducteur. Le réglage versionné
actuel est `version: 1`, avec `claim_lease_seconds: 30` par défaut et une plage
admise de 5 à 300 secondes. Une occupation de l'espace produit l'état `waiting`,
la cause `workspace_wait`, l'acteur et l'action suivante
`retry_after_resource_release`, sans consommer de tentative.

## Reprise après arrêt

L'effet `request_resume` est un signal durable du moteur, identifié de façon
stable à partir de la demande et enregistré dans la même base que la mission. Si
le processus s'arrête après l'effet mais avant la clôture de l'occurrence, le
redémarrage retrouve cette identité et termine la même demande sans répéter
l'effet. Si la présence ou l'absence de l'effet ne peut pas être démontrée, le
service conserve `uncertain_effect` et demande une réconciliation opérateur ; il
ne promet pas une exécution exactement une fois chez un fournisseur.

L'état `executed` signifie que la demande a été traitée. Il ne signifie ni qu'une
mission est terminée, ni que son résultat est accepté.

## Programmes horaires C02

Un programme cible toujours une mission existante et l'action unique
`request_resume`. Les formes admises sont `once`, `daily` et `weekly`. Une
récurrence doit fournir à la fois `until_local` et `max_occurrences`; elle ne
peut donc pas être infinie. Le fuseau est un nom IANA explicite (`UTC` est
accepté, `Local` est refusé) et chaque prévisualisation rend les instants UTC.

La politique DST est volontairement stricte : une heure locale inexistante ou
ambiguë est refusée (`nonexistent_local_time` ou `ambiguous_local_time`) au lieu
d'être déplacée ou choisie silencieusement. Si l'horloge recule avant le dernier
instant observé, le programme est retenu et journalise `clock_moved_backward`.

La création produit toujours l'état `disabled`. L'activation, la pause et
l'archivage sont explicites et révisionnées; une archive ne peut pas être
réactivée. Au redémarrage, `missed_policy=skip` journalise chaque occurrence
passée sans demande ni rafale. Deux occurrences visant une même mission déjà
porteuse d'une demande active sont coalescées dans cette demande, avec toutes
leurs sources et motifs conservés.

Une reprise causale exige une nouvelle preuve SHA-256 et vérifie réellement que
la cause précédente a changé. Elle revalide l'autorisation, les plafonds et
l'espace, et relit le cooldown fournisseur. Elle ne modifie ni tentatives, ni
budgets, ni coûts, et ne lève jamais un cooldown.

Les réglages opérationnels sont versionnés dans `config/automation.json` :
`claim_lease_seconds=30`, `due_grace_seconds=60`,
`max_preview_occurrences=20` et `max_schedule_occurrences=366`. Une copie locale
`.swarm/automation.json` peut porter le même contrat versionné; les bornes sont
validées avant toute utilisation. Le moteur n'installe aucun poller autonome :
un hôte autorisé appelle explicitement le tick, qui soumet ensuite via le service
C01.

## Archivage et restauration des programmes

La corbeille conserve les programmes, leurs journaux, les origines des demandes
et les preuves de reprise causale ; la restauration sur place conserve leurs états.
Une archive ZIP contenant des données d’automatisation utilise le manifeste
version2 et exige un importeur compatible ; les archives version1 restent lisibles.
L’export lie ces tables à la même transaction que la mission et ses événements.
L’import refuse les cibles/références étrangères, les tables/colonnes dupliquées
et les tables non autorisées. Les programmes qui étaient activés sont importés
désactivés : l’archive ne donne aucune autorisation de lancement. Les identifiants
et journaux sont conservés, et un travail déjà présent n’est jamais écrasé.

## Événements externes C03

La route `POST /api/v1/automation/external` est uniquement joignable par le
serveur web loopback existant. Elle reçoit un document JSON borné contenant
exactement `schema_version`, `event_id`, `target_work_id` et l'action unique
`request_resume`. Les en-têtes `X-Swarm-Key-ID`, `X-Swarm-Timestamp` et
`X-Swarm-Signature` sont obligatoires. La signature vaut
`sha256=HMAC-SHA256(secret, timestamp + "\n" + corps_exact)` : changer les
espaces du JSON change donc le contenu signé et provoque un conflit si
`event_id` est réutilisé.

Les clés sont lues depuis `.swarm/automation-external-secrets.json`, qui doit
être un fichier local régulier sans permission groupe/monde. Chaque clé active
énumère ses missions et actions autorisées. Rotation et révocation sont relues
à la réception, au claim et juste avant l'effet ; une ancienne signature ne
crée aucune autorisation durable. Ne placez jamais ce fichier, une vraie clé ou
un corps reçu dans un rapport, une commande, un journal public ou un dépôt.

La politique versionnée définit par défaut
`external_max_payload_bytes=65536`, `external_rate_limit=60`,
`external_rate_window_seconds=60` et
`external_timestamp_window_seconds=300`. Les bornes admissibles sont
respectivement 256..1048576 octets, 1..10000 événements, 1..3600 secondes et
1..3600 secondes. Le journal conserve seulement identités/digests, résultat et
code de cause ; ni corps ni secret. Un événement accepté signifie seulement
qu'une demande durable existe. Son traitement, le résultat de la mission et
son acceptation sont trois états distincts.
