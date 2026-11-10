# M24 — liaison exacte des usages du résumé extractif

17 octobre 2026, reprise « on fait la suite ». Propriétaire : Codex mémoire,
sans délégation. `replan`, puis APEX borné. Les résultats IA3b sont consultés en
lecture seule : noyau humain vérifié, enrôlement protégé et admission publique
encore ouverts. Le chantier identité conserve ses sources et ses décisions.

## Décision et périmètre

M23 a livré les cachets, sans les transformer en attestations. M20 attend encore
la persistance propriétaire/auteur et les autorités publiques, notamment avant
plan. Une activation mémoire globale à partir d'un cookie, du rôle Admin ou d'un
plan absent créerait une autorité implicite. Elle n'est pas choisie.

Deux voies : attendre toute l'intégration identité, ou réaliser le contrat interne
indépendant sur les producteurs mémoire déjà vérifiés. Retenir la seconde :
résoudre la sélection M19 depuis un même `Work`, reconstruire ses octets avec
`memorySummaryAttachment`, et lier les contrôles à cette proposition exacte.
Cette liaison est de la donnée de demande, jamais une autorisation. Elle ne
retourne ni texte source ni contexte à un appelant public.

Le lot ajoute uniquement `memory_usage_binding.go` et ses tests. Aucune migration,
route, champ Launch public, activation, changement d'autorité ou d'asset web.
Pas de modification des fichiers identité, des producteurs mémoire existants
ou des processus utilisateur.

## Contrat retenu

- Ressource exacte par source : `memory:v1:` suivi du SHA-256 du tuple JSON
  versionné projet local/travail/ID/kind/role. Le digest de chemin du projet
  reste une liaison locale, sans devenir une identité globale ou une autorité.
- Proposition liée au projet, à la révision du travail, à la portée du plan,
  à la tâche, au principal/à l'instance destinataire, au paquet source, à la
  sélection et aux octets de l'attachement reconstruits par M19.
- Lecture et synthèse requises sur toutes les sources effectivement chargées
  par la préparation extractive ; transmission seulement sur les sources
  retenues dans le reçu M19. L'instantané opérateur n'est jamais transmissible
  par cette liaison. Ces contrôles décrivent ce pipeline, pas une implication
  universelle entre les trois permissions.
- Conversion en demandes `agentIdentityRequest` exactes, avec événements enfants
  stables et distincts. Le noyau décide ensuite ; aucun octroi n'est créé.
- Un résultat de compilation/décision pure ne prouve ni identité du destinataire,
  ni engagement atomique, ni réception. Le consommateur courant devra encore
  fournir les cadres protégés, vérifier l'autorité du destinataire à recevoir,
  réévaluer sous les verrous et enregistrer l'effet. Aucune réentrée SQL ajoutée.

## Critères observables

| ID | Preuve attendue |
| --- | --- |
| M24-01 | La projection de plan peut rester identique lorsque la réponse change ; la liaison mémoire doit changer ou refuser une sélection périmée. |
| M24-02 | Les demandes couvrent chaque lecture/synthèse et chaque transmission réellement retenue ; pas de transmission du snapshot ni d'une omission. |
| M24-03 | Principal, instance, tâche, portée, projet, sélection ou octets différents rendent une décision antérieure incompatible. |
| M24-04 | Entrées invalides ou absence de sélection/destinataire/portée donnent une liaison vide, jamais des demandes partielles ; le noyau refuse précisément le contrôle sans permission ou avec octroi expiré/révoqué. Aucune atomicité d'effet n'est déduite. |
| M24-05 | Les mêmes entrées donnent les mêmes demandes ; aucune mutation des entrées ou lecture SQL supplémentaire. |
| M24-06 | Sources liées aux preuves, tests ciblés nommés, compilation/statique appropriées, docs FR/EN et RETEX. |

Plan : conserver les préimages ; écrire d'abord les cas ; implémenter le
constructeur privé ; vérifier les cas ciblés contre le noyau pur avec fixtures
explicites ; inspecter le diff ; actualiser les contrats et résultats. L'état
des autres critères M20 reste ouvert. Si les entrées de compilation changent
pendant les contrôles, ne pas déclarer la preuve liée au candidat courant.

Réalisation : cinq tests de premier niveau et 27 sous-cas du contrat passent,
ainsi que deux gardes publiques M19. Le contrôle statique et la compilation
finale sont liés aux entrées dans
`evidence/m24-20261017/frozen-verification-pack.json`. Le premier
`verification-pack.json` est conservé comme commandes réussies avec entrées
concurrentes, sans verdict valide sur un état unique. La reprise utilise un
overlay limité aux 22 fichiers identité, sans remplacer les fichiers du checkout.
[Résultat et RETEX](RESULTATS-MEMOIRE-20.md). M20.2 reste partiel : ce résultat
local n'active pas les droits sur les parcours publics et ne ferme aucun des
dix critères de conformité globale M20-U01–U10.
