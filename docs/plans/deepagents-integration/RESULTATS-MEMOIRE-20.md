# M24 — liaison des demandes mémoire aux sources et au destinataire

17 octobre 2026, Codex seul. Reprise `replan`, réalisation bornée APEX, revue
locale de l'auteur ; aucune revue indépendante, mission native ou acceptation
globale n'est déclarée. Le besoin complet M20 reste ouvert.

## Résultat du lot

`internal/engine/memory_usage_binding.go` prépare les contrôles du pipeline
extractif existant M19 à partir de ses vrais producteurs. Il ne crée aucune
autorité. Les sources sont résolues et l'attachement est reconstruit depuis le
même `Work` fourni au constructeur ; aucune lecture SQL supplémentaire.

La proposition lie projet local, travail/révision, portée/tâche, principal et
instance destinataire, paquet source, sélection et attachement exact. Chaque
source obtient une ressource opaque exacte, indépendante des préfixes de chemin
ou de motifs glob. La préparation exige des demandes distinctes de lecture et
de synthèse sur ses sources chargées ; seules les sources reçues par le worker
selon M19 obtiennent une demande de transmission. Une omission et l'instantané
opérateur ne deviennent pas transmissibles. Les mêmes entrées et le même
événement parent produisent les mêmes événements enfants, distincts par contrôle.

Le cas de fixture contient cinq sources : dix demandes lecture/synthèse et trois
demandes de transmission. Toutes portent le digest de la proposition complète.
Un changement de réponse peut préserver la projection de plan, mais invalide
la sélection précédente ; une nouvelle sélection produit une autre liaison et
le noyau refuse de réutiliser l'ancienne décision. Une autre instance destinataire
rend elle aussi la décision périmée.

Une demande valide n'est pas une permission. Le constructeur ne retourne aucun
texte source, ne vérifie pas l'adoption d'un plan ou l'attestation du destinataire,
ne publie pas d'octroi et n'applique aucun effet. Aucun handler, commande ou
lanceur public ne le consomme encore. Propriété et auteur historiques ne sont
ni modifiés ni rétroactivement attestés. Les cachets M23 restent inconnus pour
les droits et l'identité, et le serveur utilisateur est conservé.

## Sources et preuves

Le suivi contient `evidence/m24-20261017/baseline.json`, cinq préimages et l'état
sale initial, avec la branche et le HEAD. Neuf contrats mémoire/identité lus
sont suivis en lecture seule. Les modifications préexistantes et l'autre
conversation sont préservées ; aucun message inter-chat n'est envoyé.

Les tests sont écrits avant l'implémentation de la nouvelle brique. Le premier
passage donne **32 noms PASS, zéro FAIL**, avec 1 068 entrées de compilation
inchangées : `targeted-first.jsonl`, `targeted-first-run.json` et les deux
manifestes `targeted-inputs-*.json`. Il ne s'agit pas d'un défaut public reproduit
puis réparé : le contrat est nouveau, et la séparation plan/contenu est une
propriété déjà observée du moteur que le raccord doit respecter.

La revue locale renforce ensuite les assertions : refus exact de décision
périmée après changement du texte ou du destinataire, permission absente et
octroi inactif après expiration/révocation. La version de test initiale reste
dans `memory_usage_binding_test.first.go.txt`. Le passage courant comprend
également les deux gardes existantes du parcours public de sélection M19 et
de l'export source CLI/API. Verdicts, noms exacts et limites sont conservés dans
`targeted-final.jsonl`, puis `frozen-targeted.jsonl`,
`frozen-verification-pack.json` et `audit-final.json`.

Le passage final compte **34 noms PASS, zéro FAIL et zéro SKIP** : 32 noms pour
le nouveau contrat (cinq tests de premier niveau et 27 sous-cas), plus les deux
gardes publiques existantes. La revue ne se limite pas à l'existence d'une erreur :
les codes attendus sont `agent_identity_decision_stale`,
`agent_identity_permission_denied` et `agent_identity_grant_inactive` selon le cas.

Le premier pack final, `verification-pack.json`, contient trois commandes
réussies, mais trois fichiers identité ont changé pendant ces vérifications :
`agent_identity_registry_types.go`, `agent_identity_issuer_linux.go`,
`agent_identity_receipt_linux_test.go`. **Ce pack ne valide pas un état unique
du dépôt.** Les refus et résultats originaux sont conservés, sans leur attribuer
une validité de compilation courante.

Reprise M24-I01 : figer uniquement les 22 fichiers `agent_identity_*.go` dans un
overlay Go temporaire, avec capture stable vérifiée. Les sources mémoire et les
autres entrées restent dans le checkout et sont contrôlées avant/après. Aucun
stub, fichier de contrôle, base réelle ou copie complète du dépôt. La vue
identité réelle est conservée dans `identity-view.zip`, avec empreintes et
mappage dans `identity-view.json`. Les commandes `frozen-*` vérifient cette vue
figée, pas le dernier état du chantier identité. Les neuf contrats suivis en
lecture seule ne sont pas présentés comme inchangés : deux ont évolué dans
l'autre conversation et sont réinspectés, sans modification de notre part.

Le binaire candidat est isolé sous `/tmp`, son empreinte est conservée ; il
n'est pas lancé sur le serveur utilisateur. L'entrée `cmd/swarm/main.go` et la
version de Go sont capturées avant compilation. Aucun appel modèle ou fournisseur
réel. La construction ne renouvelle aucune validation du moteur identité courant.

Le pack courant `frozen-verification-pack.json` passe : **34 noms PASS, zéro FAIL,
zéro SKIP, `go vet` et compilation réussis, 1 070 entrées effectives inchangées**.
Il correspond aux sept tests principaux et 27 sous-cas sur cette vue précise.
Les 22 copies identité sont vérifiées contre le manifeste et conservées dans
l'archive ; les autres sources, dont les deux fichiers mémoire, viennent du
checkout courant. Le hash du binaire est dans le pack ; aucun binaire candidat
de ce lot n'a été déployé ou présenté comme le serveur utilisateur.

## Critères et portée de vérification

| Critère | Vérification | Limite |
| --- | --- | --- |
| M24-01 Texte et plan distincts | Même projection de plan après réponse changée ; sélection périmée refusée ; ancien accord refusé après nouvelle liaison. | Fixture pure, sans adoption réelle ni effet. |
| M24-02 Sources et droits séparés | Treize demandes valides et uniques, dix lectures/synthèses, trois transmissions, aucune pour l'omission/snapshot opérateur. | Prépare des demandes ; ne les consomme pas atomiquement. |
| M24-03 Substitution | Principal, instance, tâche, contrat, portée, projet, sélection et révision changent le digest ; ancien accord refusé sur autre destinataire. | Le destinataire doit encore provenir d'un résolveur protégé courant. |
| M24-04 Refus | Entrées invalides, sélection absente/périmée/dupliquée, liaison modifiée ou vide ne produisent pas de résultat partiel ; codes exacts du noyau pour droits manquants, expiration/révocation. | Aucun engagement ou effet distribué testé. |
| M24-05 Déterminisme et isolation | Mêmes demandes pour mêmes entrées, entrées JSON inchangées, aucun Store/SQL appelé par le constructeur. | Le consommateur futur doit posséder son snapshot pendant la résolution et l'engagement. |
| M24-06 Livraison | Résultats courants, entrées liées, statique, compilation, diff, docs et schémas FR/EN dans l'audit final. | Ni revue indépendante ni déploiement. |

Les cinq tests de premier niveau et leurs sous-cas couvrent le nouveau contrat.
Les deux gardes publiques réutilisent de vrais Store isolés et les producteurs
CLI/API en processus de test ; elles ne remplacent ni recette navigateur ni
preuve du binaire déployé. Aucun écran, comportement public ou synchronisation
n'est modifié par ce lot : les recettes navigateur M23 ne sont pas relancées,
et aucun contrôle `-race` n'est présenté comme nécessaire à cette fonction pure.

## Replan, apprentissages et suite

L'état IA3b a changé depuis M20.2 : son noyau humain est vérifié, mais l'enrôlement
protégé et l'admission publique ne sont toujours pas disponibles. Options :
attendre toute IA3, activer un raccourci implicite, ou réaliser le contrat privé
indépendant. Le troisième choix préserve le besoin et ne confond pas données
de demande et autorité. Aucun raccourci n'est implémenté.

La revue de la spécification a aussi précisé M24-04 : une liaison correcte peut
exister sans permission ; c'est le noyau qui refuse le contrôle non autorisé.
Assimiler une liaison à un accord aurait contredit le contrat lui-même. Cette
précision est conservée dans le plan sans affaiblir les critères d'effet M20.
Les appels et les assertions de refus vérifient cette distinction.

La compilation du paquet partagé a duré environ deux minutes alors que les tests
purs s'exécutent en moins d'une seconde dans le premier passage. Deux compilateurs
actifs ont été observés ; les entrées sont restées inchangées pendant ce passage.
Cela ne prouve pas un défaut de runtime ni la causalité exacte du coût sous charge.
La coordination des fenêtres de compilation reste utile ; l'envoi d'un message
à l'autre conversation a été proposé à l'utilisateur, avec autorisation explicite
requise par l'outil. Aucun accord n'est déduit de l'absence de réponse. Le suivi
identité conserve déjà IA3b-I07 sur le couplage des ressources ; ne pas créer un
nouveau chantier structurel ou une copie complète du dépôt pour ce lot.

Cette observation est ensuite devenue un défaut effectif de liaison des preuves
dans le premier pack final. La mesure choisie change la précondition : octets
des seuls fichiers identité figés, autres sources et copie vérifiées. Répéter la
même compilation sur des sources mobiles n'aurait pas fourni de preuve nouvelle.
Propriétaire de l'isolation M24 : conversation mémoire ; coordination générale
des deux conversations en attente de l'autorisation de message demandée.

Apprentissage transmis dans le contrat d'usages et les pages mémoire FR/EN :
le digest de plan ne suffit pas, un droit de lecture ne donne pas les autres
usages, et le destinataire doit être lié au contenu exact. La présence de cette
brique dans le dépôt ne prouve pas sa réception ou son adoption par l'autre
conversation ou par un processus déjà lancé.

M20.2 est partiellement implémenté ; les dix critères de conformité publique
M20-U01–U10 restent ouverts. Le raccord suivant doit recevoir du chantier identité
l'enrôlement/checkpoint protégés et l'autorité explicite avant plan, puis résoudre
demandeur et destinataire depuis le contrôle courant, vérifier les droits du
destinataire à recevoir, persister propriété/auteur et consommer l'ensemble des
contrôles dans l'engagement autorisé. Il doit préserver l'ordre Trust → SQLite
→ runtime et enregistrer décision/application/réception sans réentrée SQL.

Propriétaires : conversation identité pour ses autorités, opérateur pour la
désignation du principal humain initial et de la frontière indépendante,
conversation mémoire pour persistance et consommateurs. Condition de reprise :
contrat d'autorité utilisable et prouvé, avec attribution de propriété/auteur,
validation des refus sans mutation et preuve d'effet. Les codes privés
`memory_usage_binding_invalid` et `memory_usage_sources_invalid` devront aussi
recevoir leurs messages métier FR/EN au raccord public ; aucun message de cette
brique n'est actuellement affiché par l'interface. Le lot ne clôt ni IA4/IA5 ni
la conformité globale de l'application.
