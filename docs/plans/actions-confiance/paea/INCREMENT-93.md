# PAEA93 — surface navigateur des états d'effets (PA-08)

Continuité glm-resume-20261029T125222Z après l'incrément 92 (consommateur CLI
du settlement call, installée v256). Limite levée : la capacité de l'appel Git
gardé (91) et le rapprochement des journaux d'effets (86) n'étaient
consommables que par HTTP brut ; aucun opérateur ne pouvait lire ces états
réels depuis le navigateur. Décision de conception réversible argumentée (non
soumise à arbitrage opérateur, conforme PRODUCT-ARCHITECTURE/UI-DESIGN) :
nouvelle page dans la famille « Vérifications PAEA », pas le cockpit —
cohérence produit (deux surfaces d'observation, pas des commandes), réutilisation
du contrat UI qualifié de la famille, surface strictement lecture seule
réversible.

## Réalisé

- `web/paea-effects.html` + `web/paea-effects.js` : consommation lecture seule
  de `GET /api/v1/paea/git-call` (capacité : `bound`, `scope`, `reason`,
  `service`, `operation`, `root_digest`, `action_digest` quand préparé) et
  `GET /api/v1/paea/reconcile-effects` (rapport PA-09 : par domaine
  `uninitialized`/`consistent{effectCount, findings}`/`corrupt{codes}`,
  findings par `intent_id` avec `verified` et refus exacts). Aucun POST,
  aucun bouton d'effet, aucune garde contournée ; le rapport brut complet est
  replié dans un `details` natif. Chargement/ vide/erreur/succès explicites,
  jamais un faux succès : un refus (501/409/500, code exact) reste affiché.
  Lien d'évitement en premier tab, bouton thème ≥ 44 px, FR/EN, thèmes
  etat/sombre, largeurs mobile.
- `web/paea.html` : lien hub vers la nouvelle page.
- `rust/crates/swarm-web/src/service/native_admin.rs` : deux entrées d'assets
  embarqués (`text/html` + `application/javascript`) après les entrées
  paea-lab.
- `locales/en.json` : 24 clés FR→EN nouvelles ; `web/i18n-en.js` régénéré
  (`npm run i18n:build`, statut pass).

## Tests

- `rust/crates/swarm-web/tests/paea_effects_page.rs` (2/2 PASS) : page et
  script servis 200 sous session réelle ; le script consomme les deux GET ;
  aucun POST dans le script ; hub lié ; 403 sans session sur l'asset ; clés EN
  présentes.
- `tests/paea_effects_ui.cjs` (qualification navigateur réelle) : binaire
  frais, racine fixture 0700 en mkdtemp, `init` réel, serveur web réel,
  connexion session réelle par formulaire ; boucles FR/etat/1280 et
  EN/sombre/390 : capacité non préparée = état vide explicite avec message
  FR/EN exact ; refus PA-09 visible avec code exact (fixture sans grand livre
  accessible : « path » du `RootStore`, jamais un succès vide) ; thème réellement
  commuté ; pas de débordement horizontal ; premier tab = lien d'évitement ;
  bouton thème ≥ 44 px ; captures par langue. Contexte navigateur séparé sans
  session : page 403 ET API 403 avec `web-access-refused` en texte. Tri
  explicite des refus attendus (`expected_denials`, listener URL porteur) :
  `unexpected_diagnostics: 0`, `expected_denials: 8`. Preuve
  `evidence/20261029T131839Z-paea-effects-ui-mvfdts57` (captures fr/en +
  paea-effects-ui.json).

## Recette permanente

`tools/verification/recipes/paea-effects-ui.json` (4 étapes : catalogue i18n
`--check`, tests Rust de page, reconstruction binaire frais avec re-touch de la
table d'assets embarqués, qualification navigateur réelle via
`tests/paea_effects_ui_recipe.py`) → PASS 4/4, `recipe.py verify` → VERIFIED,
preuves `evidence/20261029-paea-effects-ui-recipe-mvfdts57`.

## Installation locale et vérification du service réel

Build supporté `swarm_local.py build` → candidat qualifié v0.1.1-dev.257
(sha256 `47d31746…`) ; `restart --prepared-build 47d31746…` → « Entrées
compilées modifiées : 0 », ELF vérifié `/proc/<pid>/exe` == `bin/swarm` ==
v257. Vérification sur le service réel installé
(`evidence/20261029-paea-effects-live-mvfdts57`) : page et script servis 200
sous session réelle ; `git-call` 200 `bound:false` (état réel) ;
`reconcile-effects` 200 rapport réel (les deux journaux `uninitialized` sur la
racine de grand livre adoptée) ; navigateur réel — hub `paea.html` lie la page,
navigation aboutit, état vide explicite, état `success` avec domaines réels
`uninitialized` et limites affichées, textes FR et EN rendus, capture d'écran.

## Docs

- `docs/architecture/PAEA-GIT-TRANSACTION.md` § « Surface navigateur des états
  d'effets (incrément 93) » + miroir EN (« Browser surface for effect states
  (increment 93) »).
- Note version_history FR/EN.

## Politiques et erreurs (revue)

- Politiques codées en dur : aucun nouveau plafond ni default ; les codes de
  refus affichés sont lus des réponses réelles, pas dupliqués en constantes
  locales ; le tri des refus attendus dans le test est un choix de test justifié
  (fixture sans grand livre), pas une politique produit.
- Erreurs : les refus 403 anonymes (texte `web-access-refused`, transport) et
  500/501/409 (JSON `failure.code`) sont suivis jusqu'à l'affichage visible ;
  aucune conversion refus→succès vide (assertions dédiées dans les deux tests).

## Limites restantes

La page reflète les rapports réels ; elle n'établit ni authenticité des
journaux (recomputation PA-09 seulement), ni enrôlement, ni acceptation.
Séparation UID, enrôlement humain, approbation indépendante, PA-09 complet
(ancrages externes, revue indépendante), PA-11/12 (E3), 17 invariants globaux
et acceptation globale restent ouverts ; ils ne sont pas fabriqués ici. Aucun
commit/push ; installation locale v257 effectuée dans le même incrément.
