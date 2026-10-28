# Laboratoire de simulation — R17

Le laboratoire exécute un corpus fermé versionné contre le moteur Trust, dans
une fixture propre créée par le serveur. Aucun fournisseur, paiement ou dossier
réel ne reçoit d’action. Cette portée ne qualifie pas les providers sensibles.
L’état de qualification se lit dans le reçu de vérification, pas dans cette page.

Le corpus v1 comprend cinq scénarios : refus d’écriture directe/élévation Biba,
accord exact, effet unique malgré un second engagement, réponse perdue puis
réconciliation et altération d’original sous ancre. Son manifeste expose version,
digest, identifiants, connecteur simulé, horloge/politique de fixture et portée.
L’horloge est celle de la simulation ; aucune entrée ne change l’horloge métier
réelle. La graine est une chaîne décimale uint64 canonique, pour conserver tous
ses bits dans Go, CLI et navigateur. Les entrées libres, chemins, exécutables,
URL, destinations et connecteurs de production sont refusés par le schéma.

## Administration et exécution

Namespace CLI `laboratory`, API `/api/v1/trust-laboratory` : `show`,
`policy-preview`, `policy-apply`, `run`, `result`, `replay`. Session opérateur,
Origin/CSRF et identité métier I3 administrateur sont requis comme dans le pilote.
Le client distant `--server` utilise les mécanismes locaux authentifiés existants,
sans repli automatique sur une racine locale.

Politique persistée `laboratory` : `enabled` booléen strict, `timeout_seconds`
entier positif (proposition30s), `max_concurrent_runs` entier positif
(proposition1). Zéro interdit, y compris politique inactive ; omission du bloc
conserve la valeur précédente. La révision de configuration partagée protège les
mises à jour. Les bornes techniques sont exposées séparément de la consommation
`active_runs` et du total historique `attempts`. L’aperçu ne modifie rien.

`run` exige `schema_version:1`, `event_id`, `expected_revision`, motif et `input` :

```json
{"corpus_version":1,"corpus_digest":"DIGEST_LU_DANS_SHOW", "seed":"42",
 "scenario_ids":["biba-denials","exact-approval"],
 "connector_id":"local-ledger-simulation-v1"}
```

Version/digest et scénarios sont contrôlés avant admission ; aucune valeur du
fichier à tester ne devient automatiquement autorité. La tentative est réservée
avant le lancement ; un appel identique relit son reçu sans nouvelle exécution.
`result` prend `run_id`. `replay` prend les champs d’événement/révision/motif et
`run_id` : il crée une autre tentative liée à la précédente, avec les mêmes
entrées. Le digest déterministe exclut heure, PID et chemins d’exploitation.
Un résultat divergent n’est pas accepté comme rejeu réussi.

## Isolement, interruption et limites

Le parent lance seulement son helper fixe par pipes, avec environnement minimal.
Le helper refuse un répertoire occupé, ferme les descripteurs étrangers, applique
les14 contrôles Landlock/seccomp, puis exécute le corpus sur le thread d’opération
verrouillé. Le réseau et les exécutables directs sont refusés ; les appels Trust
restent dans ses sous-racines. Aucun code, callback ou goroutine utilisateur n’est
accepté. Ce confinement reste celui du helper fixe, pas un bac à sable général.

Le délai administré annule et ferme le groupe de processus. Le travail se déroule
hors verrou métier ; la publication exige encore identité, révision et politique
courantes. Une invalidation ferme la tentative comme échec sans publier de succès.
Le propriétaire PID/démarrage est enregistré ; sa disparition rend la tentative
interrompue, sans effacer sa consommation. Une nouvelle tentative est explicite.
Seules les racines temporaires possédées par cette exécution sont nettoyées.
Un arrêt brutal du serveur peut laisser une fixture temporaire orpheline ;
son contenu est simulé, mais son nettoyage historique n’est pas automatisé par
un scan global. La tentative interrompue reste visible et consommée.

Les rapports conservent corpus, entrées, politique, observations et limites ; ils
ne contiennent ni jetons bruts, ni clé de fixture, ni chemin de mission. Les
sources du corpus et le moteur évoluent séparément : un ancien résultat reste
lisible même si son corpus n’est plus exécutable ; aucun rejeu ne l’adopte comme
version courante. La collecte eBPF et les fournisseurs réels restent distincts.

Sources de processus : [Go1.24.3 SysProcAttr](https://pkg.go.dev/syscall@go1.24.3#SysProcAttr). Le parent reste sur son thread pendant la vie de l’enfant ; signal de mort et annulation sont vérifiés séparément.

Recettes isolées :

```sh
go test -race -count=1 -run '^TestActionsConfianceT6Laboratory' -v ./internal/engine
python3 tests/actions_confiance_laboratory.py --binary BIN --evidence-dir DIR
node tests/actions_confiance_laboratory_ui.cjs BIN DIR
```

Le parcours affiche un verdict, les comptes de scénarios/contrôles OS et les cinq
libellés lisibles. Le rapport JSON reste replié, ouvert seulement à la demande.
