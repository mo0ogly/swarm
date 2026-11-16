# Contrôles inertes de rétention Git/stdio — proposition non exécutée

Ce harnais vérifie le contrat du consommateur `run_git_stdio`, qui appelle son
`stdio_probe` réel, sur deux sources fournies explicitement et épinglées. Il
remplace `subprocess.run` avant tout chargement du candidat ; aucune commande
réelle, Biba, Git, driver, fournisseur, mission ou instance installée ne tourne.
Le module fixture et le publisher sont des doubles explicites : cela ne qualifie
ni la configuration Admin, ni le writer atomique, ni le confinement système.
Les marqueurs PASS du candidat restent capturés dans les preuves privées.

## Matrice proposée

Huit tests déclarés, sans quota d'exécution : témoin sain ; processus nonzero
avec drift ou refus de lecture du posthash ; JSON malformé avec ces deux mêmes
issues ; première publication après achèvement avec ces deux issues ; seconde
publication avec drift. Chaque injection est précédée et suivie d'un témoin sain
sur un nouveau répertoire, même lorsque les assertions de l'injection échouent.
Chaque appel simulé doit être unique. L'achèvement est matérialisé avant le défaut
par `owned-completion.json`. Les canaux attendus sont comparés octet par octet.
Les sous-tests séparent les défauts : absence de postcheck, perte d'un canal,
cause primaire perdue, refus de publication masqué ou reçu structuré absent.

Même fichier et mêmes assertions sur ancien/nouveau candidat. Aucune branche de
version, symbole propre au correctif ou `expectedFailure` ne transforme RED en
PASS. L'injection de première publication vise l'ancien stdout et le nouveau
reçu du processus ; la seconde vise ancien stderr/nouveau reçu de postcontrôle.
C'est un défaut logique commun après achèvement, pas un faux rejeu du même appel
bas niveau (les nouveaux canaux sont directement écrits, sans publication).

## Exécution ultérieure, après autorisation distincte

Lire `source-closure.json` du paquet privé et fournir `--candidate`, son
`--candidate-sha256`, `--outcome-source`, son `--outcome-sha256`, puis
`--evidence-parent` existant et admis par le propriétaire de stockage. Lancer le
même `tests/test_biba_stdio_retention.py` pour chaque candidat. Ce fichier et sa
recette ne constituent pas une autorisation d'import/exécution.

La recette permanente proposée utilise la source canonique et les variables
`SWARM_STDIO_RETENTION_CANDIDATE_SHA256`,
`SWARM_STDIO_RETENTION_OUTCOME_SHA256`,
`SWARM_STDIO_RETENTION_EVIDENCE_PARENT`. Elle exige un résultat positif, sans
convertir un refus en succès. Le lanceur conserve les sources effectivement
chargées, leurs pins avant/après, tous les essais, la sortie unittest et un
résumé. Il refuse Python optimisé et toute collection autre que huit tests.
Le nombre huit est un inventaire de cas, pas une limite produit ou un timeout.

## Limite trouvée à la lecture

Le candidat corrigé appelle `outcomes.execute`, puis parse stdout. Si execute
lève sur le posthash, le parsing ne s'exécute pas : la cause JSONDecodeError
n'est pas établie dans le groupe, même si les octets malformés sont conservés.
Les deux tests JSON + posthash exigent les deux causes et pourraient donc encore
échouer sur ce candidat. C'est une prédiction de lecture, pas un test exécuté.
Conserver cette exigence ; voir OPEN-SOURCE-GAP.md. Ne pas annoncer une recette
verte ni un résultat Biba réel à partir de ce paquet.

## Révision des assertions typées — non exécutée

Les causes sont reconnues uniquement par les nœuds structurés `type`,
`exceptions`, `cause` et `context`. Un diagnostic n'est comparé qu'au message
du nœud portant le type attendu. Le texte d'un RuntimeError qui nomme
JSONDecodeError ne constitue donc pas une erreur de parsing. Les reçus doivent
contenir les champs structurés exacts : code entier 7 pour le processus nonzero
(0 pour JSON malformé), tentative d'exécution vraie, signal nul, postcontrôle
tenté et explicitement en échec, ainsi que son erreur typée. Les empreintes
des deux canaux sont comparées à leurs octets attendus. Les refus de publication
sont exigés à leur propre frontière : reçu initial ou reçu diagnostic secondaire.

Une garde pure, incluse dans le test sain sans ajouter de test nommé, compare
le faux texte à de véritables objets d'exception, vérifie les branches groupées
et causales, et rejette le code incorrect, un booléen remplacé par 1, un reçu
seulement textuel ou une erreur de publication portant le mauvais type.
Cette garde n'a pas été exécutée. Les huit noms, témoins, récupérations, doubles
et arguments ancien/nouveau sont conservés. Les deux cas JSON + postcheck
restent ouverts : aucune exigence n'est assouplie et aucun candidat figé n'est
modifié par cette révision du harnais.
