# Fermeture différée / Deferred qualification closure

Si le lanceur retourne3 avec `REVIEW_REQUIRED`, les tests ne sont pas un PASS de
qualification : la revue et les graphes restent ouverts. Après revue des octets,
fournir `source-review.json` dans le dossier existant puis lancer :

```sh
python3 tools/verification/close_qualification.py --evidence CHEMIN_QUALIFIE --component financial
```

Le dossier `finalization01` doit être neuf. Ce second processus vérifie toutes
les sources encore présentes (Go généré/toolchain, registre Rust inclus), les
archives, graphes et profils. Il vérifie les sorties et inventaires existants,
sans présenter les tests antérieurs comme réexécutés. Les méthodes de fermeture,
leurs tests et cette page doivent figurer dans `proof_methods` de la déclaration,
avec SHA et quatre axes reviewed ; leurs originaux sont archivés séparément.
Aucun ancien résultat/échec n'est écrasé. Un refus demeure ouvert ; diagnostiquer
et conserver un nouveau dossier de preuve avant toute reprise. Ce contrôle local
ne fournit ni signature d'auteur, ni revue indépendante, ni déploiement.

When the launcher returns3/REVIEW_REQUIRED, runtime tests are retained but review
and graph closure are still pending. After exact-byte review, use the command
above. The finalization directory must be new. This separate method phase checks
all effective inputs, archives, graphs and actual production/fixture profiles,
then records original test counts without rerunning or relabelling them. Its own
source/test/document hashes and four-axis review must be in `proof_methods` and
archived references. Original failures remain intact. Failure never becomes PASS;
local consistency is not signed authenticity, independent acceptance or deployment.

Les arguments conservés sont des données : seules les commandes exactes Go/Cargo
d'inventaire sont acceptées, sans exécutable ou option supplémentaire. Le triple
hôte est validé comme argument technique. Cette garde ne certifie pas l'identité
ou le confinement du toolchain local.
Recorded argv is data: only exact inventory commands are accepted, without extra
executables/flags; the compiler host triple is validated. This does not authenticate
or confine the local toolchain. The third isolated regression covers malicious
recorded commands and flags; it executes none of those rejected candidates.

Le composant `pricing` couvre le catalogue/devis et la CLI, sans HTTP/Admin natifs
ou couplage automatique aux réservations. `parity-reference.json` est obligatoire :
chemin relatif contrôlé et SHA du résultat, originaux et binaires inchangés. Aucun
nom de dossier de parité n'est deviné. Les preuves anciennes sans cette référence
se lisent avec leur méthode originale archivée ou demandent une nouvelle capture ;
ne pas fabriquer une référence rétroactive. Les inventaires Go doivent être nommés,
non vides, frais (count1) et cohérents ; cached/skip/mismatch ne sont pas une exécution.
Pricing covers native catalogue/quotes/CLI only. Closure requires an explicit
hashed in-root parity reference and unchanged original binaries; old results are
never renamed or overwritten. Go execution inventory must be nonempty, fresh and
match its declared tests; this proves no human or independent acceptance.

L'auteur de la déclaration doit être indiqué honnêtement : une revue locale
Codex n'est pas une rédaction/validation humaine. Le libellé ancien de la
docstring « human-authored » était inexact ; aucune approbation humaine n'est
fabriquée par ce contrôle. Authorship must be truthful: local Codex review is
neither human authorship nor independent acceptance.
