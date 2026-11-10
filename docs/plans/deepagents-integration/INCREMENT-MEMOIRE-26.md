# M30 — MEM-P1, captures de restauration

17 octobre 2026, reprise « next », Codex seul, replan/audit-pdca en correction.
Baseline courante et sources originales conservées dans `evidence/m30-20261017`.
Le travail M29 non commité et le chantier identité restent préservés.

## Observation et arbitrage

Les boucles mission quittent sur erreur d'inventaire ; leur diagnostic reste à
qualifier. La capture dynamique de corbeille expose une frontière préalable :
`trashRestorePreview` ne vérifie que le JSON, `applyRestore` le schéma externe,
puis réinsère les tables sans lier leur Work et leurs propriétaires SQL à la
mission demandée. Le jeton d'aperçu décrit tailles/comptages, sans empreinte de
la capture. Hypothèses à reproduire avant correction, priorité à cette intégrité
persistante ; les boucles restent une suite explicite de MEM-P1, pas clôturée.

Deux solutions : restaurer puis contrôler seulement le Work avant commit, ou
valider la capture dès l'aperçu et la relire à l'engagement. Retenir la seconde,
avec validation répétée dans la transaction et liaison aux octets exacts. Le
contrôle final seul laisserait un aperçu trompeur et les lignes liées étrangères.
Pas d'écriture, même temporaire, pour calculer un aperçu.

## Séquence et critères

1. Reproduire sur vraies suppressions/restaurations de fixtures : Work incohérent,
   ligne liée étrangère, absence/duplication de table Work et capture remplacée.
2. Extraire un validateur de capture et le décodage des cellules SQL, réutiliser
   le noyau Work M28 et les périmètres du producteur `lifecycleSpecs` ; conserver
   les captures anciennes dont seules des tables optionnelles sont absentes.
3. Lier le jeton à l'empreinte de la capture et recontrôler sous réservation SQL,
   avant insertion, suppression de corbeille ou reçu de succès.
4. Vérifier refus sans mutation, reprise valide, conflits, migration/import
   automation concernés, CLI/API ; docs FR/EN et RETEX dans le même incrément.

| Critère | Observable |
| --- | --- |
| M30-01 | Contre-exemples originaux stables ; aucun trigger supprimé ou base réelle utilisée. |
| M30-02 | Une capture a un seul Work correspondant à la clé, révision et titre de corbeille ; enveloppe Work commune, structure/cellules et rattachement des lignes contrôlés. |
| M30-03 | Changer les octets de capture invalide la confirmation, y compris à tailles égales ; recontrôle avant effet dans la transaction. |
| M30-04 | Refus sans insertion, perte de corbeille ou faux reçu ; cas valide, historique, relations et échecs atomiques restent vérifiés. |
| M30-05 | Noms attendus réellement exécutés, entrées stables, tests ciblés/race, statique/compilation, documentation et RETEX ; limites explicites. |

Ce lot ne crée aucun droit d'identité, ni nouvel acteur propriétaire/auteur M20.
Il contrôle la portée SQL d'une restauration existante. Pas de migration,
politique, compte système, agent natif, appel réel, délégation ou déploiement.
Les tables/références d'identité ne deviennent pas une autorité par restauration.
Sans borne contractuelle globale, Go `-timeout=0` et subprocess sans timeout ;
progression suivie. Les limites produit existantes restent intactes.

## M30-I01 — preuves et reprise de compilation

La série rouge exécute 19 cas précis en échec (20 noms avec leur parent
agrégateur), plus une capture ancienne valide qui passe. Entrées stables :
quinze captures invalides fournissent un aperçu utilisable ; une substitution
à taille égale et une modification dans la transaction sont acceptées, la panne
SQL devient une absence de corbeille et le producteur accepte une enveloppe
incohérente. Les originaux et la source du test sont conservés.

Le premier `green` ne collecte aucun test : la nouvelle variable `err` de la
branche de désarchivage rend une déclaration courte SQL invalide. Correction
de cette déclaration après diagnostic ; reçu original conservé. La portée du
contrôle n'est pas réduite. Les règles de rattachement suivent les sélections
du producteur ; une réservation de préparation peut avoir un work_id nul ou
vide uniquement si sa préparation appartient à la capture, sans convertir un
type de cellule incorrect en valeur vide.

## M30-I02 — confidentialité des diagnostics de conversion

La revue du nouveau consommateur d'aperçu identifie les erreurs strconv qui
recopient la valeur de cellule. `red-private` reproduit deux cas précis,
int/float : cause conservée, mais valeur factice privée exposée dans Error.
La correction utilise une erreur enveloppante au message public fixe ; Unwrap
conserve l'erreur originale pour errors.Is/As. La validation partagée de cellules
est utilisée par restauration locale et import automation. Les reçus des séries
précédentes restent liés à leur source, puis les contrôles concernés sont
réexécutés sur ce candidat. Ce n'est pas un nouveau droit de lecture.

## Livraison locale et reprise

Sur le candidat final : 131 noms normaux PASS (53 parents, 78 sous-cas), 38
sous race (15 parents, 23 sous-cas) entièrement compris dans la série normale ;
vet et compilation passent. Quatre assertions sur onze commandes du vrai
binaire, HTTP authentifié/rejeu/réouverture et quatre rendus Mermaid FR/EN
complètent les preuves. Les empreintes de sources restent stables pendant les
contrôles, puis sont confrontées aux sources courantes avant livraison.

Contrats Work, mémoire et référence CLI actualisés en FR/EN ; suivi et RETEX
dans `RESULTATS-MEMOIRE-26.md`. Pas de contexte agent ou d'asset web embarqué
modifié ; pas de génération produit requise. Revue par l'auteur uniquement.
Les sources du chantier identité et des boucles mission restent inchangées.

MEM-P1 reste en cours. Les captures dynamiques sont qualifiées dans ce contrat
de Work et de périmètre SQL ; elles ne prouvent pas toutes les enveloppes JSON
des objets liés. Prochain travail autorisé : diagnostics des boucles mission,
callbacks et fallbacks M29, puis qualification des enveloppes liées et de
l'injection pendant la sonde de lancement. Chaque suite conserve son propriétaire
et son critère dans le rapport ; aucune attribution ou autorisation M20 ajoutée.
