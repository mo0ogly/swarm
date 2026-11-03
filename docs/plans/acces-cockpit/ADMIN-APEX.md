# Administration des accès — APEX, 2 novembre 2026

English: [../../en/plans/acces-cockpit/ADMIN-APEX.md](../../en/plans/acces-cockpit/ADMIN-APEX.md)

## Besoin

Demande de l'opérateur : comptes, habilitations et journaux d'accès au moins
équivalents au menu **Outils** de Wattson (`wattson_devcontainer/flaskProject`,
Python), refaits aux standards Swarm (Rust, démon d'autorité, FR/EN, deux thèmes,
mobile, clavier, ressources locales, réglages dans l'Admin).

Décisions de l'opérateur (2 novembre 2026) :
- grain des droits : **familles d'API + un droit « voir » par entrée de menu** ;
- **groupes de comptes** (rôles attribués au groupe, droits = union).

Point de départ : phase B (`PHASE-B-AUTORITE.md`, entrées 14 à 16), vue cockpit
`/?view=authority` (commit 5c9ca0fe8). L'existant n'avait pas été comparé à Wattson.

## Référence Wattson (inventaire du 2 novembre 2026)

Sources : `admin/admin_rbac_routes.py`, `admin/admin_rbac_roles_routes.py`,
`admin/admin_security_routes.py`, `admin/admin_logging_routes.py`,
`core/audit_logger.py`, `core/rbac.py`.

- Comptes : aucun compte local (SSO Keycloak) ; aucune liste ni fiche utilisateur.
- Groupes SSO : créer, renommer, dupliquer, supprimer, statut admin, rôle ;
  garde « dernier admin » et « soi-même ».
- Rôles : intégrés (`admin`, `analyste`) et personnalisés ; matrice de 83
  ressources de menu en 11 sections ; Tout cocher/décocher, compteur, clés
  orphelines, bandeaux lecture seule ; refus de suppression d'un rôle attribué.
- Journal d'accès par requête : compte, groupe, IP, réseau /24, agent utilisateur
  (non affiché), méthode, chemin, menu, action, statut, durée.
- Page Connexions : filtres période, compte, réseau, « sans admin » ; détail par
  session, filtre actions ; 1 000 lignes sans pagination.
- Tableaux de bord : période 7/30/90/365 ; 4 indicateurs ; 6 graphes ; 20 dernières.
- Purge : confirmation armée 4 s, 5 appels/min, purge automatique à 365 jours.
- Niveau de log applicatif (POST sans protection CSRF).

Défauts Wattson à ne pas reproduire : retrait de rôle impossible, pas d'export ni
de pagination, actions RBAC tracées seulement en fichier, niveau de log sans CSRF.

## Matrice d'écart

| Wattson | Swarm au 2 novembre | Exigence |
|---|---|---|
| Comptes (SSO) | CRUD comptes, mot de passe, désactivation, oubli | conservé ; recherche, tri, dernière connexion, groupes |
| Groupes CRUD + rôle | absent | ADM-01 |
| Matrice 83 ressources / 11 sections | 14 familles à plat | ADM-02, ADM-03 |
| Menu filtré par droits | menu complet | ADM-04 |
| Journal requête (UA, durée, menu) | méthode, chemin, statut | ADM-05 |
| Connexions + filtres + détail | sessions, période, compte | ADM-06 |
| 4 KPI + 6 graphes | tableau par jour et par compte | ADM-07 |
| Purge armée + automatique | purge manuelle | ADM-08 |
| Niveau de log | absent | ADM-09 |
| (absent) actions admin en base | intention/résultat bruts | ADM-10 |
| Aide RBAC | absente | ADM-11 |

IP et réseau : Swarm n'écoute que sur la boucle locale ; l'IP vaut toujours
`127.0.0.1`. Elle est remplacée par le **moyen d'accès** (navigateur ou clé API)
et l'**agent utilisateur** court.

## Exigences

- **ADM-01 Groupes** : groupes (clé, libellé, rôles, membres) dans le journal
  signé des comptes ; créer, renommer, dupliquer, supprimer, membres, rôles ;
  droits effectifs = rôles du compte ∪ rôles de ses groupes ; refus
  `authority-group-exists`, `authority-group-missing`, `authority-group-in-use`,
  `authority-last-admin`, `authority-self-lockout` ; retrait de rôle possible.
- **ADM-02 Droits de vue** : catalogue unique `view.<id>` (section, libellé,
  préfixes d'API exclusifs) couvrant chaque entrée du menu cockpit ; garde de test
  `web/index.html` ↔ catalogue ; API exclusive refusée sans le droit de vue ;
  politique schéma 3, conversion v2 sans perte d'accès.
- **ADM-03 Matrice** : sections, Tout cocher/décocher, compteur, clés orphelines,
  bandeaux lecture seule, usage du rôle (comptes, groupes).
- **ADM-04 Menu filtré** : `/me` renvoie les vues permises ; entrées et groupes
  vides masqués ; accès direct sans droit : message clair et API 403.
- **ADM-05 Journal enrichi** : durée, vue, moyen, agent utilisateur, détail ;
  anciennes lignes et chaîne toujours valides.
- **ADM-06 Connexions** : colonnes, filtres (période, compte, vue, moyen, sans
  administration, statut), recherche, tri, pagination serveur, détail, export CSV/JSON.
- **ADM-07 Tableau de bord** : période, 4 indicateurs, 6 graphes SVG locaux avec
  tableau alternatif, 20 dernières connexions.
- **ADM-08 Purge** : confirmation armée, limite de débit, rétention automatique
  `audit_retention_days` réglée dans l'Admin (vide = aucune purge automatique).
- **ADM-09 Journal du démon** : `log_level` réglable ; diagnostics du démon
  conservés dans un fichier borné et lisibles dans l'Admin.
- **ADM-10 Actions d'administration** : détail lisible de chaque mutation
  d'administration, onglet filtrable.
- **ADM-11 Aide et ergonomie** : aide repliable par section, confirmations sans
  `prompt()`, clavier, mobile, FR/EN, deux thèmes.

## Tâches

| T | Exigences | Périmètre | Acceptation | État |
|---|---|---|---|---|
| T0 | — | ce registre FR/EN | registre relu | fait |
| T1 | ADM-02 | `swarm-authority/src/policy.rs`, défauts, `tests/policy.rs` | catalogue, schéma 3, conversion, garde menu, 403 | fait (vérif. locale) |
| T2 | ADM-01 | `accounts.rs`, `admin.rs`, `tests/admin_api.rs` | groupes, union, dernier admin | fait (vérif. locale) |
| T3 | ADM-05, ADM-10 | `audit.rs`, `server.rs`, `admin.rs` | champs enrichis, compatibilité | fait (vérif. locale) |
| T4 | ADM-06, ADM-07 | `audit.rs`, `admin.rs` | filtres, pagination, export, KPI | fait (vérif. locale) |
| T5 | ADM-08, ADM-09 | `config.rs`, `server.rs` | rétention, niveau, diagnostics | fait (vérif. locale) |
| T6 | ADM-03, 04, 06–11 | `web/*authority*.js`, `admin.css`, traductions | interface complète | fait (vérif. locale) |
| T7 | toutes | `tests/authority_admin_ui.cjs`, recette | navigateur FR/EN, thèmes, mobile, clavier | fait (vérif. locale) |
| T8 | — | documentation FR/EN | comportement réel et limites | fait |
| T9 | — | `./swarm.sh authority install` (sudo opérateur), `./swarm.sh restart` | binaire vérifié, parcours live | à faire |

## Limites

- Les API partagées entre vues restent régies par leur famille (pas de filtrage par objet).
- Pas de SSO ni d'alerte temps réel ; IP non pertinente en boucle locale.
- L'installation du démon exige `sudo`, lancée par l'opérateur.

## Preuves

### T1 (2026-11-02)

- Commande : `cargo test -p swarm-authority --test policy` puis
  `--test accounts --test audit --test assertion --test check --test admin_api`.
- Résultat : policy 8/8, les autres suites 6/6. Le run complet porte un échec
  préexistant non lié : `tests/front_e2e.rs` exige `SWARM_ENGINE_BINARY`
  (binaire précompilé absent de l'environnement de développement).
- Contenu vérifié : `VIEWS` (16 vues), `SCHEMA_VERSION` 3 avec conversion
  v1/v2 (`view_family_set`, règle any-of, une vue s'obtient par ses familles
  mais n'accorde jamais la famille), refus distinct `policy-no-view`
  (403, raison dans le corps), étage Biba sur les clés de vue (lecture),
  `/me` renvoie `views`, `/policy` expose le catalogue de vues.
- Garde de test `views_match_menu` : bijection entre `data-view` de
  `web/index.html` (12 entrées) + l'entrée `authority` injectée par
  `web/cockpit-authority.js` + les 3 pages autonomes, et le catalogue ;
  unicité des préfixes exclusifs.
- Garde `v2_conversion_preserves_access` : le défaut v2 archivé (commit
  562953dba3) converti égale le défaut v3 livré ; demandes v2 permises
  conservées ; clé inconnue v2 refusée.
- Défauts de test corrigés en cours de route (tomes à retenir) : le POST
  `/api/v1/agent-logs/clear` sans droit tombe sur `unclassified.write`
  (étage Biba 3) et non `policy-no-grant` — sujet remonté à I3 ; les
  spot-checks de
  conversion sans perte ne doivent couvrir que ce que le rôle tenait
  effectivement en v2.
- Aucune écriture sous `/tmp` par les tests (temporaires via
  `CARGO_TARGET_TMPDIR`).
- Limite non couverte ici : le refus 403 côté serveur HTTP avec le corps
  `{"error":"authority-policy-denied","reason":"policy-no-view"}` sera prouvé
  par la recette navigateur T7 (la décision `decide()` est déjà vérifiée).

### T2 (2026-11-02)

- Commande : `cargo test -p swarm-authority` depuis `rust/` (dont
  `--test admin_api`, 120 s, 1/1 vert). Même échec préexistant non lié que
  pour T1 : `tests/front_e2e.rs` exige `SWARM_ENGINE_BINARY`.
- Journal signé : groupes en enregistrements `group/<clé>/<nonce>`
  (`GroupBody` schéma 1, `deny_unknown_fields`), même clé de signature et
  même mutex série que les comptes ; pas de génération — le groupe courant
  est l'enregistrement actif de plus haute `enrolled_sequence` ; toute
  modification (libellé, rôles, membres) écrit un nouvel enregistrement et
  révoque les autres ; `group-body-mismatch` si clé/schéma incohérents.
- Union des droits effectifs : `Accounts::current()` et `Accounts::list()`
  renvoient les rôles **effectifs** (propres ∪ rôles des groupes actifs
  où le compte est membre, triés dédupliqués). Sessions, clés API,
  assertions signées, décisions de politique et `/me` héritent sans
  changement ; un changement de membership s'applique à la requête
  suivante, sans fin de session.
- API `…/authority/groups` : GET liste, POST crée
  (`group-exists` 409), `…/K/profile` (rôles/membres, retrait de rôle
  possible), `…/K/rename {key}` (nouvelle clé, ancienne lignée révoquée,
  ancienne clé réutilisable — testé), `…/K/duplicate` (copie rôles et
  membres — testé), `…/K/delete` (`group-missing` 404 si absente).
  Refus d'entrée : `group-key-invalid`, `group-label-invalid`,
  `group-roles-invalid`, `group-members-invalid` (400).
- Gardes de sécurité (évaluées sur l'état candidat des droits effectifs
  contre la politique courante) : `authority-self-lockout` (409) quand le
  demandeur perdrait l'administration (testé : dan ne peut pas supprimer
  son propre groupe admin), `authority-last-admin` (409) quand plus
  aucun compte n'administrait. Le garde « dernier admin » est une défense
  en profondeur : non atteignable en test car `authority-self-lockout`
  empêche déjà de rétrograder le seul autre admin — documenté, non couvert.
- Constat de conception retenu : le garde lit `list()` (rôles déjà
  effectifs) puis retranche la contribution courante des groupes pour
  retrouver les rôles propres avant de réunir avec l'état candidat —
  un bug initial passait `&[]` et rétrogradait tout le monde (repéré par
  le test, corrigé, garde de non-régression = le test e2e).
- Aucune écriture sous `/tmp` par les tests ; documentation du module
  (`accounts.rs`, `admin.rs`) mise à jour dans le même incrément.

### T3 (2026-11-02)

- Commandes : `cargo test -p swarm-authority --test admin_api` (1/1 vert)
  puis `--test audit` (2/2). Même échec préexistant non lié que T1/T2 :
  `tests/front_e2e.rs` exige `SWARM_ENGINE_BINARY`.
- Champs enrichis (rétrocompatibles) : `duration_ms`, `view`, `client`
  (`browser`|`api-key`), `user_agent` (tronqué à 120), `detail` — tous
  optionnels avec `serde(default, skip_serializing_if)` : une ligne ancienne
  décode à l'identique et la chaîne reste valide (test
  `pre_enrichment_lines_still_verify` : ligne pré-enrichissement puis ligne
  enrichie enchaînée, `prev` reste le dernier champ).
- Capture : structure `Meta` dans `server.rs` (UA, client, instant
  d'arrivée) construite une fois par requête ; `outcome` porte
  `duration_ms` ; `intent` et `outcome` portent `view`/`client`/`user_agent` ;
  `note` (refus, consultation) porte `view`/`client`/`user_agent`.
- Événements de session : `sign-in`, `sign-in-refused`, `sign-out`,
  `reset-requested` portent `client`+`user_agent` (le chemin `/login` n'est
  pas une vue exclusive → `view` reste absent).
- Détail lisible (ADM-10) : `audit::admin_detail(méthode, chemin)` décrit
  l'action depuis le chemin seul (jamais le corps → aucun secret) : «
  account created » (sujet dans le corps non lu), « password reset: X »,
  « group renamed: X », « policy applied », etc. Porté par la ligne
  `intent` (écrite avant exécution) ; les relais moteur gardent `None`
  (le chemin sert déjà de détail).
- `audit::view_of(chemin)` : préfixe exclusif → sa clé `view.*`, sinon
  `view.authority` pour l'API d'administration, sinon `none`.
- Amélioration de contrat trouvée par le test : les échecs
  d'infrastructure du relais (moteur injoignable, 502) ne passaient pas
  par le bloc de journalisation — la consultation d'une page n'était pas
  tracée quand le moteur était absent. Corrigé : la note consultation est
  écrite aussi sur les sorties anticipées 502 ; l'assertion e2e
  (navigation `Accept: text/html` sans moteur → 502 + ligne consultation
  `view: "none"`) sert de garde de non-régression.
- Refus de politique : la ligne `refused` porte maintenant vue, client et
  UA (utile pour distinguer navigateur et clé API dans les connexions).
- Aucune écriture sous `/tmp` par les tests ; Rustdoc des fonctions
  modifiées (`note`, `intent`, `outcome`, `Meta`, `admin_detail`, `view_of`)
  mise à jour dans le même incrément.

### T4 (2026-11-02)

- Commandes : `cargo test -p swarm-authority --test admin_api` (1/1 vert,
  150 s) puis `--test audit --test accounts` (3/3). Même échec préexistant
  non lié que T1–T3 : `tests/front_e2e.rs` exige `SWARM_ENGINE_BINARY`.
- Connexions (ADM-06) : `SessionSummary` gagne `clients` (moyens vus sur les
  lignes de la session : `browser`, `api-key`) et `views` (clés `view.*`
  touchées, `none` inclus). Route `GET …/audit/sessions` : filtres
  `subject`, `client`, `view`, `no_admin=1` (sessions des comptes sans
  décision `administer` sur POST …/accounts sous la politique courante),
  `status=active|signed_out|refused`, recherche `q` (compte ou identifiant,
  insensible à la casse), tri `sort=last_at|first_at|subject|actions|refusals`
  avec `dir=asc|desc` (desc par défaut, asc pour `subject`), pagination
  serveur `page`/`per_page` (défaut 50, max 200) avec `total` inchangé,
  export `format=csv|json` **sans pagination** (en-tête
  `Content-Disposition: attachment`). `status` inconnu → 400
  `authority-request-invalid`.
- Tableau de bord (ADM-07) : `GET …/audit/dashboard` renvoie en plus
  `kpis` (`sign_ins`, `refused_sign_ins`, `actions`, `refusals` — les 4
  indicateurs), `views` (`audit::view_usage` : consultations/actions/refus
  par clé de vue, `none` regroupe les pages moteur sans vue exclusive,
  tri consultations puis actions décroissants) et `recent` (20 lignes les
  plus récentes de la période, ancre de purge exclue). Les 6 graphiques SVG
  et le tableau alternatif sont construits côté UI (T6) à partir de
  `days`/`views`/`kpis` — mêmes données, aucun appel externe.
- Filtre `no_admin` : décision réelle `policy::decide(POST …/accounts)` par
  compte (rôles effectifs, étages Biba inclus), pas une supposition sur un
  rôle ; liste des comptes lue sur le pool bloquant.
- Aucune écriture sous `/tmp` ; Rustdoc (`sessions`, `kpis`, `view_usage`,
  docstring de module `admin.rs`) mise à jour dans le même incrément.
- Limite : `recent` renvoie les lignes brutes (événement, sujet, chemin,
  statut) — c'est l'équivalent Swarm des « 20 dernières connexions » de
  Wattson, à raisonner en visites plutôt qu'en sessions.

### T5 (2026-11-02)

- Commandes : `cargo test -p swarm-authority --test admin_api` (1/1 vert,
  171 s) puis `--test sessions` (2/2). Même échec préexistant non lié que
  T1–T4 : `tests/front_e2e.rs` exige `SWARM_ENGINE_BINARY`.
- Configuration (ADM-08/09) : deux champs `audit_retention_days: Option<i64>`
  (absent/`null` = aucune purge automatique ; ≥ 1 sinon, testé avec 0 refusé)
  et `log_level: String` (`error|warn|info|debug`, défaut `info`), tous deux
  `serde(default)` : les documents `authority.json` antérieurs décodent
  (testé), la validation refuse un niveau inconnu. Défaut embarqué : 365
  jours, `info` (asserté dans `tests/sessions.rs`).
- Purge armée (ADM-08) : la purge manuelle `POST …/audit/purge` est limitée
  à 5 appels par minute glissante (`Front::consume_purge_slot`, horodatages
  en mémoire) — le 6e appel renvoie 429 `authority-purge-rate` avec
  `Retry-After: 60` (testé : 5 × 200 puis 429 consécutifs). Échec testé en
  e2e seulement après correction : les deux premiers appels du test étaient
  invalides (non comptés) et la suite e2e s'étend sur plus d'une minute —
  six appels consécutifs prouvent la fenêtre.
- Purge automatique : tâche de fond du démon (`spawn_maintenance`, purge au
  démarrage puis toutes les 6 h) selon `audit_retention_days` ; acteur
  `automatic` dans l'ancre de purge ; échec (chaîne rompue, IO) tracé dans
  le journal de diagnostics, jamais bloquant. Non testé en temps réel
  (fenêtre de 6 h) — la purge sous-jacente est la même que la manuelle,
  déjà testée.
- Journal du démon (ADM-09) : nouveau module `diag.rs` — un fichier JSONL
  `authority.log` borné (8 MiB, troncature par le début à moitié, écriture
  atomique via `.tmp`+rename+fsync), niveau minimum filtrant les lignes,
  `GET …/authority/log` renvoie les 200 dernières lignes + niveau courant +
  niveau appliqué ; le niveau suit la configuration appliquée en direct
  (testé : `debug` appliqué → `/log` le reflète). Écritures best effort :
  un échec de diagnostics n'affecte jamais une réponse.
- Aucune écriture sous `/tmp` (vérifié) ; Rustdoc et docstrings (`diag.rs`
  complet, `config.rs`, `sessions.rs`, `admin.rs`) mises à jour dans le
  même incrément.
- Limite : la purge automatique hérite de la limite de débit ? Non — elle
  appelle `audit.purge` directement (pas le endpoint), aucune limite ne
  s'applique ; seul le chemin manuel est limité.

### T6 (2026-11-03)

- Interface complète (ADM-01/03/04/06–11), trois fichiers : `web/native-authority-admin.js`
  (montage autorité), `web/cockpit-authority.js` (filtrage du menu),
  `frontend/engine-settings/admin.css` (styles), plus `locales/en.json`
  (+74 clés) et `web/i18n-en.js` régénéré (`npm run i18n:build`, insertion
  pure vérifiée : 0 ligne supprimée).
- Groupes (ADM-01 UI) : table Créer/Modifier/Dupliquer/Supprimer appelant
  exactement les contrats serveur (`POST /groups`, `/profile`, `/rename`,
  `/duplicate`, `/delete`) ; la modification de l'identifiant en édition
  devient un renommage puis un profil.
- Matrice (ADM-03) : sections du catalogue avec Tout cocher/Tout décocher
  (rôle admin exclu par construction), lignes `view.<id>` affichées, clés
  orphelines signalées en avertissement, compteurs par rôle (droits tenus
  + usage réel : N comptes, M groupes) ; rafraîchissement différé quand un
  autre panneau recharge les données.
- Menu (ADM-04) : `cockpit-authority.js` masque les entrées non permises
  et les groupes de navigation vidés (les groupes contenant des liens
  `<a>` — préparation/confiance/PAEA — restent toujours visibles) ;
  `cockpit-native.js` refuse l'accès direct à une vue non permise avec le
  message clair et l'API reste l'autorité (403 serveur éprouvé en T1).
  Garde `views_match_menu` (bijection index.html ⇄ `policy::VIEWS`)
  toujours verte après ajout de l'entrée `authority` : 8/8
  (`cargo test -p swarm-authority --test policy`).
- Journal d'accès (ADM-06/10) : connexions avec filtres (dates, sujet, vue,
  moyen, admin exclu, statut, recherche), tri cliquable, pagination
  (« ‹ › », 50/page, total serveur), export CSV/JSON via
  `Content-Disposition` (pagination ignorée à l'export, contrat T4) ;
  détail enrichi (N°, date, événement, requête, vue, moyen, résultat,
  durée, détail).
- Tableau de bord (ADM-07) : 4 KPI (`sign_ins`, `refused_sign_ins`,
  `actions`, `refusals`), 6 graphiques SVG locaux (barres, `viewBox`,
  info-bulle par barre) — connexions/actions/consultations/refus par jour
  et consultations/actions par vue (`none` affiché « pages moteur »),
  tableau alternatif dépliable par graphique, comptes, 20 dernières
  entrées.
- Actions d'administration (ADM-10) : onglet alimenté par `GET /audit`
  filtré côté client sur `event === 'intent'` + fenêtre de dates des
  filtres (le serveur n'expose pas de paramètre `event` — limite
  documentée, les volumes restent bornés par la période choisie).
- Journal du démon (ADM-09) : onglet `GET /log` (200 dernières lignes,
  `pre` à défilement borné) et changement de niveau en direct (édition
  structurée de la config courante + `POST /config` avec `revision`,
  relecture après écriture).
- Purge et rétention (ADM-08) : purge manuelle (regex + confirmation en
  dialogue + limite de débit serveur) et champ `audit_retention_days`
  (vide = suppression du champ = désactivation, entier ≥ 1 sinon, relecture
  après écriture).
- Aide (ADM-11) : un bloc dépliable par section (`<details>`,
  hauteur de tactile ≥ 44 px).
- Vérifications : `node --check` vert sur les trois fichiers JS ;
  `cargo test -p swarm-web` — 15 suites, 0 échec (compile lib verte après
  correction de l'initialiseur `Event` de `authority_ledger.rs` avec les
  cinq champs T3, et test `native_admin.rs` remis en accord avec le
  contrat réel : `/` sert la page cockpit depuis 0aa7335e18, son
  assertion `app.js` obsolète remplacée par `cockpit-native.js`, et son
  fixture déplacé de `std::env::temp_dir()` vers `.swarm-fixtures/`
  conformément à l'interdiction `/tmp`).
- Limites : l'échec `blp-revision-conflict` de `tests/paea_git_call.rs`
  lancé en vrac (`cargo test --test paea_git_call`) est une course entre
  tests parallèles sur la fixture déclarée partagée — chaque test isolé
  (ou la recette permanente, qui exécute chaque test séquentiellement avec
  `--exact`) est vert ; défaut de l'autre lot, non modifié ici (fichier en
  cours d'édition par une session concurrente au moment du diagnostic).
- Vérification visuelle (thèmes, mobile, clavier) : reportée à T7 (recette
  navigateur) — cette tâche ne prouve pas le rendu.

### T7 (2026-11-03)

- Parcours navigateur réel (`tests/authority_admin_ui.cjs`) : démon
  `swarm-authority serve` réel devant un moteur réel embarquant les actifs
  web, connexion par `/login` puis — création de compte avec restauration
  du focus, variante refusée, clé API (secret affiché une fois, Bearer
  ensuite), matrice des habilitations avec rôle personnalisé + enregistrement
  + offre à la création de compte, journal d'accès (connexions → détail
  action → tableau de bord → purge refusée sans jours valides), FR/EN ×
  clair/sombre (captures par carte, traduction anglaise intégrale vérifiée
  clé par clé), 390 px sans débordement global, connexion mobile.
- Défaut corrigé au passage (cause conservée) : le prédicat d'attente de
  rendu exigeait que **tous** les blocs `<details>` soient remplis, y
  compris « Aide » (jamais une table) — latence masquée par le timing
  jusqu'à l'instrumentation ; le contrat correct (tous les blocs **ouverts**
  remplis) est dans `cardReady()` avec une borne de 60 s justifiée par
  l'observation (rendu ≤ 26 s, Chrome froid, interrogation 500 ms) et un
  diagnostic d'échec conservant l'état rendu (détails/ouvert/rempli,
  boutons présents).
- Recette permanente `tools/verification/recipes/authority-admin-ui-rust.json`
  (i18n `--check` puis qualification binaire gérée via le wrapper
  `tests/authority_admin_ui_recipe.py` — binaires fournis, jamais reconstruits ;
  empreintes SHA-256 du moteur et du démon figées dans `result.json` ; sortie
  fraîche obligatoire ; /tmp refusé avant toute allocation) :
  `recipe.py run` PASS (2 étapes) puis `recipe.py verify`
  `{"integrity":"VERIFIED","recorded_status":"PASS"}` — preuve conservée sous
  `.swarm-fixtures/` (sorties de qualification plus anciennes supprimées après
  conservation de la preuve retenue ; aucun résidu projet sous /tmp).
- 41 contrôles nommés, 0 échec (la preuve antérieure à 42 contrefaits
  « debug-after-login » retiré — compteur de preuve aligné dans le wrapper).
- Limites : vérification locale sur binaires qualifiés fournis — ni
  acceptation indépendante, ni installation (T9) ; la recette n'exerce pas
  les parcours distants (relais) ni la rotation des clés au-delà du
  secret-affiché-une-fois.

### T8 (2026-11-03)

- Documentation produit FR/EN : nouvelle section « Administration “Comptes
  et accès” (mode authority) » dans `docs/architecture/NATIVE-LOCAL-LAUNCHER.md`
  et `docs/en/NATIVE-LOCAL-LAUNCHER.md` — périmètre réel (comptes, clés API
  à secret unique, groupes/union, matrice 30 lignes verrouillée admin,
  journal/filtres/export/KPI, purge et rétention, réglages du démon avec
  `revision`), contrat API `/api/v1/authority/*`, granularité
  familles + `view.<id>`, deny-by-default et planchers Biba, journal signé
  chaîné, recette permanente et ses limites (pas d'acceptation
  indépendante, relais et rotation hors périmètre).
- Registre du plan : sections T7/T8 ajoutées ci-dessus et tableau mis à jour.
