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

## Correction16 proposée après échec réel de prepare-W

État16 : SOURCE PRIVÉE SEULEMENT, non importée/non exécutée/non acceptée. La
campagne unique gi-wf-root-20261116-6azg223q a été lancée après rétablissement des
préconditions CI061b ; le paragraphe de dérive15 ci-dessus reste une observation
historique, pas l'état de cette campagne. Elle a échoué au premier planning step
pendant prepare-W. Work w-786d8f2435c51d7a0e927ec5 est réellement révision4,
activation1/6, décisions0, tâches vides, reviewer0/2, avec rejet en inbox. Le budget
est encore0/0, coût réel inconnu : le setup10/1 ultérieur n'a pas été atteint.
Aucun F ni revueW n'a été lancé. Le prepareur416488 est récolté/validé/fermé, sans
pending. Ces faits ne signifient pas rollback ou absence d'autres effets.

Le receipt SR1 original a un body SHAe4222126fddff2d44294140fe1f715f681a263038c10eb3836b9bb3f545b7f55,
exit1 et stderr non tronqué : codex ligne9 lit planning-proposal.json relatif,
FileNotFoundError errno2. Le fichier existe àRoot ; le child416703/parent416689 a
écrit ses marqueurs dans la feuille I0 du claim f7c84e0f520c740f80ae5e3d. Le prompt
original SHA934df05b... correspond au job. CLIstdout est vide ; le stdout brut du
provider n'est pas archivé par ce receipt, donc non établi en tant que canal brut.
planning history renvoie[] malgré ce receipt natif ; ne pas en inférer aucun run.

Proposition16 : program_with_proposal remplace exactement la lecture Root par
une valeur JSON littérale DANS le programme inerte, avant chaque claim. Le chemin
legacy proposal()/PROGRAM n'est pas migré implicitement pour les anciens replays.
Le flow source utilise planner_step pour les trois décisions : frame SR1, claim,
Work/scope/génération native, dev/inode privé, PID/PPID, prompt et proposition exacte
sont liés aux marqueurs de la feuille réelle. Origines copiées après retour public
réussi, exit/grant/completion du receipt ; pas de re-wait OS indépendant annoncé.

Le worker garde le workspace explicite créé par Fixture. Son programme/format
REPORT_TEMPLATE est figé avant launch ; le template est une oracle de données de
fixture et ne reconstitue jamais une sortie absente. Après run_and_assert synchrone
et agent show completed/native/exit0, lecture entière du vrai report/markerI0,
comparaison au prompt/route/format déclaré, conservation de ces octets et identité,
puis relecture avant/après publication. Le superviseur publie uniquement ces octets
réels vers Root/handoff.md, par temporaire adjacent vérifié et hardlink EXCLUSIF,
fsync et relecture. Source I0 intacte ; destination existante, même égale, refuse.
Erreur tardive peut laisser une destination visible et un temporaire conservé :
ni rollback ni retry ne sont inférés. Task submit reste public et reçoit le SHA
original, afin qu'un report public modifié ne soit pas accepté sous un nouveau hash.

Arbitrage du coordinateur : tasksubmit accepte aussi un chemin I0 directement,
mais les contrôles W/F préautorisés lisent Root/handoff.md. Le transfert superviseur
minimal préserve ces contrats ; il n'accorde aucune lecture Root au provider et
ne prouve pas une publication native autonome. La branche READY avant worker
reste sans admission worker/transfert ; le consommateur mission_manual_review
adopte la même rétention après son vrai worker. Les autres anciennes campagnes
ne sont ni exécutées ni déclarées requalifiées. L'API de retour READY et le refus
return_fixture sans before_worker sont conservés.

Vérifications FUTURES à attribuer, aucune exécutée dans16 :

- Témoin réel complet de source publique sur le même profil/I0 avant tout défaut ;
  les trois décisions doivent consommer leurs vrais prompts/outputs et créer les
  bons effets, aucun Root-relative read provider. Le contre-exemple061b réel est
  conservé ; ni nouvel incident W/F ni succès nouveau n'est fabriqué ici.
- Avant chaque variante, témoin natif/FD valide avec mêmes préconditions. Refuser
  receipt/claim/PID/prompt altéré ou clôture inconnue, sans nouvelle activation ;
  les doubles sur lecteur ne prouvent pas Authority ni OS. Refuser un report I0
  altéré avant première lecture grâce au format déclaré, ou changé entre lectures,
  sans le reconstruire ; destination existante, lien, ENOSPC et close refusée
  préservent original et causes, jamais PASS vide.
- Reprise contrôlée du consommateur seulement après décision : restaurer les
  vrais octets du défaut dans la NOUVELLE fixture attribuée, garder l'échec initial,
  utiliser un nouveau contexte de preuves du consommateur et destination fraîche,
  relire l'agent natif existant, sans relancer provider/claim ni rembourser appel.
  Ne pas écraser une destination déjà visible ; root arbitre son sort séparément.
  Modifier ensuite le report public doit refuser sous le SHA original au submit.
- DEFAULT W/F et READY True/True sont des chemins distincts ; vérifier le second
  sans worker démarré, puis son consommateur réel. Les modules14 inchangés gardent
  leur preuve de source, mais l'ancienne receipt d'import ne qualifie pas la
  nouvelle fermeture/import cyclique : root/CI doit l'attribuer explicitement.

Les sources sont retenues hors CI ; root a repris son gel pour la baseline Biba.
Aucune écriture dans la fixture échouée, tentative/budget remis à zéro, nouveau
claim ou relance W/F autorisée par ce texte. Revue indépendante, fermeture sources,
import/interpréteur, source/profile/OS/I0/Authority/storage et allocation demeurent
les gates du coordinateur. Historique377 ouvert ; aucun installé/global accepté.

## Correction17 privée du premier import (proposée, non exécutée)

La revue source09 a établi une initialisation anticipée possible de16 : entrée
directe process→watch→flow→originals→from process import Failure/check, avant leurs
définitions. Entrée directe retention ajoute la même dépendance avant read_at.
Les entrées DEFAULT/READY ne sont pas déclarées en échec : leur ordre peut déjà
charger ces symboles. Ce constat source est distinct du vrai FileNotFoundError061b.

17 retire seulement les deux from-import globaux du helper originals et les
résout dans chacun de ses trois consommateurs, au moment de l'appel. Classes14,
fonctions read_at/Failure/check, Biba/TTL/caps, visibilité des modules canoniques et
contrats des fonctions restent inchangés. Aucun alias Failure/read_at n'est
maintenu dans originals pour accommoder un test. Aucun global/default/annotation
consommant ces symboles n'a été déplacé aveuglément. L'import du helper ne les
requiert plus avant définition ; leur emploi en fonction reste explicite.

Gardes SOURCE séparées préparées dans fixture17-tests-source-only : même script
sur16/17, nouveaux processus pour process, retention, parent DEFAULT, préparateur06,
flow READY, mission_manual_review et helper direct. Canonical symbols seulement,
pas sys.modules supprimé/stub/module remplacé ni API17 absente16 comme faux RED.
Aucun import/AST/syntaxe/test/enfant exécuté pendant préparation. Les gardes sont
à revoir puis attribuer par root avec fermeture43/defaultJSON physique ; union44
si consumerREADY également présent. Les imports réels des programmes inertes sont
une autre couche. Ni une garde ni huit tests à doubles ne qualifient vraiW.

Origine16/GLMrequest restent figées ;17 est une autre candidate. Les tests lisent
Failure canonique et injectent par seamOS partagé16/17, sans réclamer le réexport
retiré. Revue indépendante/allocation avant import, puis nouveau témoin réel avant
repriseW/F ; Workrévision4 et toutes preuves originales préservés.

## Delta17 : diagnostic de publication (R09-16-DIAG)

La relecture du temporaire, auparavant hors enveloppe, conserve maintenant la
cause originale avec son nom aléatoire, son identité de création si observée et
l'étape tentée. Un fstat du descripteur nouvellement créé collecte cette identité ;
son propre refus est conservé et ferme une seule fois le descripteur. L'identité
de création n'affirme pas l'identité actuelle du chemin. Les erreurs de lecture,
fermeture et comparaison restent imbriquées dans Failure canonique.

Le diagnostic distingue tentative de lien et retour réussi du lien ; aucun refus
ne prouve absence ou rollback, y compris si la cible préexistait. Pas de retry,
écrasement, suppression supplémentaire ni changement d'octets, hash ou Authority.
L'unlink historique du temporaire après validation reste inchangé : un refus de
fsync ultérieur peut donc laisser la cible visible et le temporaire déjà supprimé.
Les gardes proposées dans GI-SOURCE-TESTS couvrent témoin, refus et reprise vers
un nouveau nom en conservant le temporaire refusé lors de la relecture. Elles ne
sont pas exécutées ici ; aucune qualification de17 ni reprise W/F n'est acquise.
