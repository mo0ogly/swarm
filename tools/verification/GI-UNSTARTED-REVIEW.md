# Revue Gi.eth : propriétaires de preuves et campagne W/F

Intégration WIP : sources14 revues pour une portée bornée et trois tests sur doubles
exécutés par le coordinateur. Ni récupération genuine, installation, acceptation
produit ni résolution du claim historique377 n'est établie par ces résultats.
Version anglaise : [GI-UNSTARTED-REVIEW.en.md](GI-UNSTARTED-REVIEW.en.md).

## Deux graphes distincts

Le graphe des trois tests utilise tests/test_rust_unstarted_review_parent.py,
rust_unstarted_review_parent/process/retention.py, la fermeture des helpers réels et
la configuration physique de l'assemblage. Le préparateur
`tests/rust_unstarted_claim_prepare.py` n'est pas importé par ces tests.
`fixture_area`6f lit/cache sa configuration à l'import : un chdir ne la recharge
pas. Ne pas remplacer le resolver ou les helpers par des stubs pour obtenir PASS.

Le graphe campagne appelle EN PLUS ce préparateur par un vrai enfant Python et le
CLI normal, puis le SDK cfg(test). Le fichier06 est déjà identique dans la base6f ;
sa présence ne prouve pas qu'il a été exécuté. Un test vert du premier graphe ne
qualifie ni le second graphe ni l'OS/I0/Authority/reviewer/public recovery.

## Résultats existants et preuve de causalité

Même test14, trois assemblages isolés, helpers/config extraits du commit
6f358346cf2dd05f49b525cdbfb8360b6b88b894, interpréteur Python3.13.7, -I -S -B :

| Assemblage | Résultat observé | Cause / portée |
|---|---|---|
| parent12 | 3 méthodes en échec, 6 assertions en échec, sous-cas inclus, 0 erreur | F lancé après ownerW pending ; faux fermé ; troisième wait consommant |
| parent13 | A passe, B/C en échec, 3 assertions, 0 erreur | retour terminal sans rétention avant changement des faits |
| parent14 | 3 PASS, 0 échec, 0 erreur | handback, clôture et consommation conservées sur doubles |

Les témoins sains précèdent chaque variante ; les échecs ne sont ni un import
manquant ni AttributeError sur le helper14. Les pauses, closes et waitid sont
contrôlés. La libération du double change uniquement des faits mémoire APRÈS
conservation de l'owner/cause/snapshot ; aucune résolution OS réelle n'est inférée.
Le terminal réel proposé conserve les propriétaires tant que pending, sans retry
wait/close/CLI ; aucun canal d'arbitrage runtime n'est livré par cette intégration.

Preuves originales conservées sous
`/run/media/fpizzi/SWARM/swarm-evidence/gi-controlled-assembly-root-20261116-fw5rv4mt/`
(red12/red13/green14-run-01 : command.json, result.json, module-origins.json, stdout,
stderr). Le premier contrôle d'origine avait omis le launcher __main__ ; refus
conservé, consommateur rectifié avec hash/chemin explicites, SANS rejouer les tests.
La revue14 APPROVE_BOUNDED_SOURCE_ONLY et la réception indépendante09
APPROVE_BOUNDED_RETAINED_CONTROLLED_BEHAVIORAL_EVIDENCE sont conservées séparément
(gi-parent14/REPORT.json et gi-parent14-controlled-runtime/REPORT.json dans le
répertoire de revue09 du16novembre). Cette réception couvre les preuves de doubles,
pas le parcours genuine. Le statut numérique de sortie RED12 n'a pas été persisté :
NON ÉTABLI, ne pas reconstruire1 depuis les assertions ou le code du wrapper.
RED13=1 et GREEN14=0 disposent de statuts explicites. Amélioration de collecte
transmise à root/CI : persister le wait/statut et hashes des canaux AVANT validation
faillible des origines, puis conserver son refus séparément ; effet non vérifié ici.

## Recette permanente ciblée, préparation sans relance

`recipes/gi-controlled-ownership.json` reprend exactement les trois méthodes et le
wrapper déjà exercé, copié sous `gi_controlled_ownership.py`. La recette proposée
elle-même n'a pas été exécutée ; son invocation par runpy diffère du lancement
original. Son PASS couvrirait ces assertions, jamais toute la campagne.

Avant exécution, root/CI réserve un assemblage physique figé et une sortie privée
fraîche. Définir SWARM_GI_CONTROLLED_ASSEMBLY et SWARM_GI_CONTROLLED_RECEIPT (fichier
neuf dans un parent déjà admis, sans secret). Vérifier l'interpréteur effectif,
source/config/UUID/espace/verrous, absence physique de .swarm, contenu de chaque
module et fermeture des imports, STDlib/extensions et origins avant/après. Le
wrapper n'assemble rien et sa receipt.write_text suppose une cible neuve ; il
n'autorise pas d'écraser une preuve. L'audit Python refuse certains événements de
processus/réseau ; ce n'est pas un confinement OS ni une preuve d'absence d'effet.
Le coordinateur compare la receipt aux octets attendus ; le simple PASS recette
ne vérifie pas à lui seul une origine externe fournie par environnement.

Invocation future seulement après allocation :

```sh
python3 tools/verification/recipe.py run --root "$SWARM_GI_SOURCE_ROOT" \
  --recipe tools/verification/recipes/gi-controlled-ownership.json \
  --output "$SWARM_GI_RECIPE_OUTPUT"
```

Sorties fraîches, aucun watchdog global ajouté, aucune reprise automatique.
Conserver comptes/noms exacts, canaux bruts, identités chargées et refus secondaires.
Réutiliser les preuves existantes tant que leurs entrées restent applicables.

## Entrée complète W/F proposée, campagne non allouée

L'entrée tests/rust_unstarted_review_parent.py exige les arguments réels suivants :

```sh
python3 -B tests/rust_unstarted_review_parent.py \
  --normal "$SWARM_GI_NORMAL" --normal-profile "$SWARM_GI_NORMAL_PROFILE" \
  --sdk "$SWARM_GI_SDK" --sdk-profile "$SWARM_GI_SDK_PROFILE" \
  --sdk-qualification "$SWARM_GI_SDK_QUALIFICATION" \
  --collection "$SWARM_GI_COLLECTION" --collection-receipt "$SWARM_GI_COLLECTION_RECEIPT" \
  --output "$SWARM_GI_WF_OUTPUT"
```

Ce bloc est une proposition, pas une autorisation ou commande exécutée. Le parent
doit être réellement colocalisé avec ses helpers dans le checkout producteur du
profil normal (contrat normal_checkout), profils et inputs relatifs inchangés.
Il lie SDK original/copie qualifiée, collection réelle et deux noms libtest
`root_store::review_preview::reviewer_candidate::unstarted_sdk::` suivis de
`unstarted_expired_claim_producer` ou
`unstarted_genuine_claim_refusal_rollback_retirement_and_replay`, uniques et --exact.
Pas de zéro-test accepté ni de nouveau --list/rebuild automatique.

Parcours proposé : W préparé par CLI et contrôles réels → revue normaleW +originaux
I0/SR1/certificat/capture et reconcile → F frais équivalent → producteur exact et
wait intégral/PID/stamp/host/job → préface departed du genuine existant → refus,
rollback, retirement, replay et vrai follow-on normal → état public/histoire/appels.
Minimum3 demandes review W1/F2, plus planner/worker/contrôles ; caps2/activations6 et
budget fixture fictif10USD/réserve1USD existants ne sont pas un coût mesuré ni une
allocation supplémentaire. Aucun renouvellement TTL, remboursement, kill, faux
verdict ou remplacement historique. Le signal SIGCHLD, sole waiter Linux,
pidfd/waitid, montage/locks/admission, droits OS/I0 et coûts exigent vérification
root/CI avant première écriture/import/exécution.

## Blocage de source constaté avant allocation

Les images normal5f et SDKd63 relues gardent leurs SHA ; profils normal4core features[]
test=false et SDKlib features[] test=true restent des preuves historiques distinctes.
Mais le checkout producteur déclaré `.swarm-fixtures/git-integration`, observé à
HEAD df1f38b78af8324fb7927ffef8f1ce8a4f14f30d, présente29 différences sur787 entrées
normal et24 sur558 SDK, dont les sources unstarted absentes. L'inventaire, les hashes
attendus/observés et les paths figurent dans campaign-prerequisite-observations.json
et campaign-input-drift-diagnosis.json du paquet integration15-WIP. Ce constat
bloque leur réutilisation campagne ; les tests isolés6f ne sont pas invalidés par
ce changement d'un AUTRE checkout. Le coordinateur confirme une séquence CI/root
autorisée pour Biba, avec gel courant à respecter ; l'allocation du profil prévoyait
déjà restauration/revalidation exacte6f avant une future campagne.

La lecture5f de ce chat est terminée et ses FDs libérés ; aucune exécution5f
allouée ici. Pour la WRlease stockage du coordinateur, ne pas relire cet original.
La preuve/image5f non installée peut être archivée comme preuve, sans exécution
sur support externe. Toute future utilisation genuine exige restauration LOCALE
par le propriétaire de la livraison, puis contrôle d'empreinte avant lancement.

Propriétaire : coordinateur/CI. Reprise : assemblage source/profile/image/import
réellement applicable réservé et qualifié : futur repositionnement propre6f par CI
avec overlays non compilateur ou nouveau baseline collationné/profil consommateur
vérifié, sans reset/écrasement du travail d'autrui
ni réécriture des preuves historiques. Pas de relance campaign dans ce paquet.
Le RETEX est de distinguer identité du binaire, applicabilité de ses sources et
conservation des propriétaires jusqu'au dernier consommateur.
