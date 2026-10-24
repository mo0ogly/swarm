# Numeric planning budget verification / Recette des budgets numériques

Run the declared targeted checks with the permanent manifest:

```sh
python3 tools/verification/recipe.py run --recipe tools/verification/recipes/planning-budgets.json --output /NEW/PATH
python3 tools/verification/recipe.py verify --evidence /NEW/PATH
python3 tools/rust-migration/verify_planning_budgets.py --output /ANOTHER/NEW/PATH
```

Output parents must exist. The last command builds actual Go and native numeric
probes plus a separate default production CLI, checks production excludes fixture
features, and compares49observations including valid witnesses, exhaustion,
invalid-state refusals and recovery. It retains complete outputs and exact binaries.
No global watchdog/retry, live database, provider, lease, identity or financial
reservation is exercised. A failing run stays incomplete; inspect its original
outputs before correcting the cause and using a fresh path. Do not rerun into old
proof directories. Full effective-source qualification remains a separate step.

FR : la recette permanente vérifie les mêmes49observations numériques Go/Rust,
avec témoins avant refus et reprise. Les sources restent dans le dépôt ; chaque
exécution conserve ses sorties dans un nouveau répertoire. Profil de production
sans fixtures contrôlé séparément. Aucun claim, fournisseur, état vivant ou droit
n'est exécuté ou établi. Une parité réussie ne qualifie pas toute la migration.

The rejection-precondition companion is maintained in
`verify_planning_rejection.py` and `tools/verification/recipes/planning-rejection.json`.
It compares62fingerprint/wait observations, including lease/counter neutrality,
owned changes, reviewer policy and null/empty shapes. No rejection is persisted
or delivered. Effective-source qualification uses `qualify_planning_budgets.py`
with `--component rejection --output NEW_PATH --parity PRIOR_PARITY_PATH`.
The historical command name is retained; component selection changes its actual
probe, expected inventory and baseline. A source-review.json bound to the current
inputs and original evidence is mandatory before graph closure: missing review
returns3 (REVIEW_REQUIRED), never PASS. Final component PASS is written only after
source stability, profile identity and exact-byte review guards pass. The review
is same-session and does not establish independent acceptance.

FR : la recette compagne couvre62empreintes/attentes Go/Rust. La qualification
sépare le composant numérique et les préconditions de refus ; elle refuse de
conclure sans revue liée aux octets. Une revue absente reste ouverte (code3).
Aucun diagnostic n'est stocké/transmis par ces API pures, aucun retry n'est autorisé.

Financial companion: `verify_financial_budget.py --output NEW_PATH` compares67
actual Go/native codec/ledger/reservation observations; `financial-budget.json`
retains8engine and3public CLI regressions. Use `--component financial` for effective
qualification on its own frozen baseline. Four-ledger estimate admission is not
actual spending, complete claim or authenticated identity. `budget apply` is ported;
show/preview/native UI remain unported. Every financial result must retain its
fixture/default production binary distinction and source review. Financial probe
rows include released history in the raw diagnostic sum; that sum is not the
active monetary view. No provider or live database is used.

FR : compagnon financier67comparaisons,8tests moteur et3parcours CLI ; aucun coût
réel inféré ni état vivant utilisé. La somme brute des lignes de fixture inclut
l'historique libéré et ne constitue pas la vue des engagements actifs. La revue
reste liée aux sources/commandes exactes ; le claim complet reste ouvert.

Pricing companion : `verify_pricing.py --output NOUVEAU_REPERTOIRE` compare101cas
explicites contre les vraies fonctionsGo et le probe natif featuretest-fixtures.
Les octets des catalogues inclus dans les deux cadres sont identiques (sinon les
empreintes diffèrent malgré des objets égaux). Témoins/refus/reprises et attentes
explicites, ancien devis, cacheinconnu/zéro, absence/nullproducteur, types/dates,
montants/volumes élevés et overflow ; aucun prix réel ou provider. La parité porte
les DTO/read/quote ; sauvegarde, concurrence et pannes ont leurs tests natifs.
Le profilexécutableproduction est construit séparément et exclut les fixtures.
`qualify_planning_budgets.py --component pricing --parity REPERTOIRE --output NOUVEAU`
élargit la vérification à tous les contrats Rust affectés par la projectionJSON
commune, puis exige la revue exacte. La clôture différée lit la référence de parité explicitement enregistrée et
hachée ; aucune convention de nom de répertoire ne remplace cette preuve.
Native pricing uses explicit101contract cases and identical raw catalogue bytes
on Go/native inputs; this is not HTTP/Admin or provider billing acceptance.
