# Adaptation de Swarm après la relecture de l’article v6

[English](../../en/plans/actions-confiance/ARTICLE-V6-ADAPTATION.md)

## Besoin et autorité

Fabrice demande, le 17 novembre 2026, une réalisation dans ce chat : corriger
les résultats après publication Git, rapprocher aussi la référence lors de
Reconcile, étudier un bail d’expiration, produire un mini-modèle TAMARIN de PR,
observer un retrait concurrent par barrière et appliquer l’oracle aux 83 cas A0.
Swarm00 reste directeur ; il a réservé ce lot au présent exécutant et suspendu
les nouveaux appels GLM concurrents. Les originaux scientifiques restent intacts.
Cette réservation ne vaut ni revue indépendante, ni acceptation, ni publication.

## État initial et preuves

Checkout partagé `claude/security-design`, HEAD initial
`a4b20f3bad6988dba5d4198307c533b6c192812e`, nombreux changements préexistants.
Pas de bascule ni ajout global à l’index. Les quatre entrées P1 sont inchangées
depuis le diagnostic et identiques au commit historique `230d4b2`.

Le dossier privé de preuve P1 est
`/run/media/fpizzi/SWARM/swarm-evidence/article-v6-p1-r0m8kuv9` : allocation,
empreintes, copies des entrées, consigne et appel direct. Montage contrôlé ext4/rw,
UUID `144f255c-3c71-43aa-9ae6-c46a6e5b8a9f`. Un descripteur du montage est
conservé pendant l’appel. La proposition GLM P1 est terminée et intégrée après corrections de revue ;
les contrôles et la revue indépendante restent décrits ci-dessous.

Le diagnostic source repère dans `paea_git_promote_call.rs` les refus tardifs
après Git, la cause COMMIT jetée et plusieurs nettoyages jetant un résultat connu.
Reconcile ne consulte que le journal. Aucun de ces constats n’est une reproduction
du parcours corrigé. Une erreur n’établit jamais l’absence d’effet.

## Plan et critères observables

| ID | Livraison attendue | État réel et preuve requise |
| --- | --- | --- |
| P1 | État d’effet typé indépendant de l’erreur et de l’autorité ; unknown après effet possible ; causes et résultats connus conservés | Candidat intégré ; perte de résultat reproduite puis corrigée sur composition figée ; six régressions moteur couvertes (cinq PASS r2 et une PASS r3 après correction du seul libellé attendu). Contrôles ciblés CLI/HTTP/texte et revue 09 ciblée terminés ; sauvegarde WIP e90b2d436b2cc8310fb00a6f67639be1d3acd503 par Swarm00, sans fusion ni installation. Parcours Git natif non vérifié |
| P2 | Reconcile relit la référence liée aux témoins physiques, sans publication ni nouvelle autorité Commit | Proposition GLM terminée ; candidat source intégré sur six chemins, dont deux ancres d’export autorisées par Swarm00. Dix gardes moteur en cours sur composition figée. Cas requis : journal absent, intention seule, effet durable, référence ancienne/candidate/autre et erreurs de lecture |
| P3 | Expiration par bail dont les hypothèses sont explicites | Non réalisé ; un timeout du client ne borne pas le point d’effet Git. Justifier horloge, borne et récepteur avant toute garantie PA/PR |
| P4 | Retrait concurrent pendant réservation partagée, ordonné par barrière | Non réalisé ; protocole préparé dans ARTICLE-V6-CONCURRENT-WITHDRAWAL.md, dépend des contrats P2/P3 et d’une fixture native neuve qualifiée. Témoin valide, demande, contention, effet, libération et retrait effectif requis, sans sleeps comme preuve |
| P5 | Deux modèles TAMARIN avec/sans section et contre-exemple figure 3a | Modèles écrits ; six verdicts attendus obtenus avec Tamarin 1.12.0 / Maude 3.5.1. PR falsifiée sans verrou, vérifiée avec verrou ; témoins positifs vérifiés. Voir annexe PR et preuves P1/tamarin ; aucune conformité Rust/Git déduite |
| P6 | Annexe oracle sur les 83 cas A0, verdict par propriété et attribution explicite | Inventaire original 2324 fichiers relu sans écart. Proposition GLM terminée puis corrigée ; outil et documentation FR/EN intégrés. 22 tests synthétiques PASS ; première évaluation réelle 83 lignes, zéro error, PT 1 fail et PV2_intention 1 fail. Revue indépendante et acceptation ouvertes, preuves article-v6-oracle-kjevtgz4 |
| P7 | Documentation FR/EN, revue indépendante, version, livraison locale et RETEX | Non réalisé ; dépend des candidats et vérifications précédents |

## Contrat et limites à préserver

PV2 dans la v6 exige qu’une réponse refused pour `i` n’ait aucun engagement
antérieur attribué à ce même `i`. Il faut définir si `i` représente une intention
ou une tentative. A0 `E1-V06` conserve un committed puis un refused
`approval_consumed` pour l’intention `invoice` : contre-exemple sous le premier
mapping, pas un PASS implicite. Le cas `E1-Q_after_commit` conserve unknown et
un effet observé. Les 83 statuts historiques ne sont pas les verdicts de l’oracle.

Une référence actuellement au candidat ne prouve pas que cette intention l’a
publiée ; un retour à OLD n’efface pas l’histoire. L’absence de ligne de journal
ne prouve pas l’absence d’effet. Un état connu ne doit pas devenir unknown lors
d’un simple nettoyage et unknown ne doit pas devenir refus par absence de ligne.

P1 précède P2 : son achèvement seul ne permet aucune revendication universelle
sur PV2. La proposition 2 de la v6 ne prouve pas PV1/PV2/PL. Une observation de
verrouillage ne prouve ni médiation globale ni garantie d’expiration.

## Revue continue et reprise

Chaque lot lie la revue aux empreintes modifiées : politiques et bornes,
contrats des fonctions, causes/états/flush/annulation/reprise. Les constantes
de protocole restent distinctes des choix opérateur persistés. Aucun watchdog
global arbitraire ni baisse de réserve disque. Les sources testées restent figées
pendant compilation/exécution ; profil fixture et profil livré restent distincts.
Tests, revue indépendante, installation et acceptation sont des résultats séparés.

Reprise actuelle : intégrer et vérifier P2 après la remise GLM, faire revoir l’oracle,
puis poursuivre P3/P4 et la livraison P7. Toute erreur déclenche replan.


## Incrément P1 et prérequis conservés

Le premier contrôle du reproducer sur l’ancienne cible partagée a échoué avant
collecte (E0422/E0599 Biba, zéro test exécuté). Les symboles sont présents dans
les sources actuelles ; les mtimes préservés et anciennes métadonnées rendent
un cache ancien plausible, sans établir la cause ni les octets historiques du
producteur consommé. Aucune modification Biba, aucun clean/touch effectué.
Une cible neuve a été enregistrée par `build_storage.register` et une composition
locale de 2109 entrées a été figée avec les ressources littérales incluses.
Le reproducer y compile et échoue réellement (1 test), faute de résultat connu
après cleanup. L’inventaire avant/après n’a aucun écart. Cette composition ne
prouve pas un graphe natif complet ni le profil livré.

La remise GLM P1 (457,6 s, fin stop, usage déclaré 120555 tokens) avait quatre
chemins inférés incorrectement, une expression booléenne invalide, un test qui
figeait la perte historique et plusieurs pertes de causes/résultats. Les originaux
sont conservés ; `integration-review.txt` trace les corrections. Un script de
préparation a échoué sur sa syntaxe avant toute mutation produit. Une compilation
candidate a ensuite révélé deux erreurs de types, corrigées avant reprise. La
régression r2 donne 5 PASS et 1 FAIL sur un libellé obsolète ; r3 modifie seulement
cette attente et exécute le cas concerné avec succès. Aucun de ces résultats ne
constitue un test d’effet Git ou de réponse HTTP réel.

Le principal a suppléé ponctuellement la production des deux mini-modèles pendant
le raisonnement P1, pour faire avancer le livrable indépendant demandé. Les preuves
Tamarin, entrées et versions sont conservées, et les sorties exportées contiennent
les preuves closes et le contre-exemple. [Annexe proposée](ARTICLE-V6-ANNEXE-PR.md).
Le binaire officiel Tamarin a été comparé au digest de la release ; Maude et ses
bibliothèques ont été extraits de paquets aux empreintes vérifiées dans
`~/.local/share/swarm-research-tools/tamarin-1.12.0`, sans paquet système modifié.
La première vérification a révélé libgmpxx manquante ; sa fourniture a permis le
contrôle Maude version et installation OK. Les commandes et erreurs initiales
restent dans le dossier P1. Le chemin isolé n’est pas ajouté au PATH global.

La revue indépendante P1 est attribuée par Swarm00 au chat 09 sur la composition
r3. Les quatre candidats et documents architecture sont gelés pendant sa lecture.
Un correctif de sortie IO CLI reste identifié : `line()` conserve actuellement
un code sûr mais pas sa cause originale. L’ajout doit être revu et testé, sans
présenter P1 comme entièrement clos avant ce contrôle.

## Reprise après revue P1 et première application P6

La revue 09 a clos les incidents IO/CLOSE puis TEXT, avec verdict final
`APPROVE_TARGETED_TEXT_DELTA_ONLY`. Les deux tests texte passent en FR/EN après
deux échecs témoins. Swarm00 a sauvegardé les dix chemins P1 dans
`e90b2d436b2cc8310fb00a6f67639be1d3acd503`, branche
`codex/wip-article-v6-p1-20261117`, sans fusion main ni installation. Le checkpoint
exclut deux constantes HELP étrangères ; il ne remplace pas la composition testée
ni une qualification du binaire. Le gel de revue P1 est levé.

P6 : le principal a suppléé la remise GLM inutilisable en l’état (syntaxe, schéma
réel du manifeste, confusion égalité PT/existence PV1, vacuité, retrait historique,
double usage, trous de trace et publication des sorties). Les originaux sont
conservés ; aucune erreur ni preuve antérieure n’est réécrite. Le pilote archivé
A0 et le source lu sont identiques (SHA-256
`ad2c203ee71ed5f3e47c4574a0efcb1861587969bd84a9db7e05fed0e4c53557`).
Vingt-deux tests nommés passent, dont refus/reprise après ENOSPC synthétique.
Le code ne lance aucun processus/réseau/SQLite ; il lit et hache les preuves.
Les littéraux sont des schémas, projections explicites ou entrées de test, sans
politique opérateur ajoutée. Les nouvelles fonctions décrivent leur contrat ;
les causes d’IO sont conservées et les sorties incomplètes restent sans marqueur.
La revue indépendante de ces octets et l’acceptation scientifique restent ouvertes.

Recette permanente P6 : `tools/verification/recipes/a0-property-oracle.json`,
commande `recipe.py run` sur dossier neuf `article-v6-oracle-kjevtgz4/permanent-recipe-r1`,
22 tests nommés exécutés, état PASS. La vérification retenue par `recipe.py verify`
contrôle les preuves déclarées ; elle n’est pas une revue scientifique indépendante.
Les cinq chemins P6 soumis à 09 restent gelés ; la recette est un ajout distinct.

### P2 — intégration et revue de la remise

Remise GLM fin stop, 380,727 s, usage déclaré 137747 tokens, dossier
`article-v6-reconcile-c7z03olb`. Le principal corrige avant test l’absence de
comparaison repository_id, la cause UTF-8 perdue et le None ambigu sur référence
symbolique. La lecture Git réelle est extraite dans le helper de production testé
sur dépôt bare neuf (absence, mouvement, suppression, symref, erreur de dépôt).
Le helper ne constitue pas le parcours signé Root complet. Les exports publics
sont les seules ancres modifiées dans root_store.rs et engine/lib.rs ; leur
substitution inverse restitue exactement les préimages de Swarm00. Aucun Git
local ni installation. Le test original journal-seul via le parcours natif reste
à reproduire sur fixture neuve ; les tests de projection ne le remplacent pas.

### P6 — revue indépendante demandant correction

09 identifie cinq contrats ouverts : contenu d’effet sous un même ID ; identité
du run entre captures ; absence d’approbation avec préfixe incomplet ; marqueur
visible malgré échec de fsync final ; index haché et analysé en deux lectures.
Les cinq empreintes antérieures, les 22 PASS et le tableau réel sont conservés
comme preuves historiques provisoires, sans acceptation. Paquet de correction
unique `article-v6-oracle-repair-bpscx4x9`, originaux et témoins à conserver.
Une analyse additionnelle en lecture seule de 382 captures n’observe aucun
changement du contenu d’un effet sous même ID ; ce résultat limité ne prouve
pas la correction de l’outil ni l’absence des autres défauts.

## Terminal handoff: P2 candidate and repaired P6

P2: engine-r1 ten PASS; real dangling-symbolic-ref guard FAIL before correction,
PASS after; consumer-check-r2 CLI/web types PASS; 2109-entry frozen compositions,
zero drift. Native target --no-run compiled before the final helper change,
not a native execution. Current code/docs/test/recipe pins and scoped limits:
`article-v6-reconcile-c7z03olb/final-pins.json`, `review-scope.md`.

P6: five defects reproduced; original targeted baseline 8 FAIL plus one runtime
refusal ERROR (corrupt second index buffer), with healthy controls. A separate
valid-second-index substitution FAILs before correction and PASSes after.
Corrected ten new guards and 22 prior cases pass together (32 named tests).
Full maintained target selected because index/trace/output shared contracts
changed across every path, not solely to update a counter; selection rationale
is retained. Permanent recipe r2 PASS/VERIFIED before final prose-only updates;
code/tests/recipe bytes unchanged by those prose updates.
Real offline evaluation-r3: 83 rows, zero error verdicts, same verdicts as the
first application (PT/W_sensitivity fail, PV2_intention/V06 fail; unprojected
PV1/PV2 inconclusive). Successful process exit AND marker hashes verified.
No receiver campaign replay. Current six paths frozen for corrective review:
`article-v6-oracle-repair-bpscx4x9/final-pins.json`.

P3 lease and P4 concurrent barrier remain unimplemented. P4 preparation is
recorded in ARTICLE-V6-CONCURRENT-WITHDRAWAL.md. No native allocation, build,
installation, main merge or global PV2/H2 acceptance is claimed.

## P3 — contrat de bail à implémenter, avant toute garantie d’expiration

Sources lues : `Identity::expires_at` dans
`swarm-enrollment-authority/src/control/paea_action.rs` est la limite signée
contrôlée avant/après callback ; `paea_git_agreement_match` valide le mandat
à l’instant de sa lecture ; `trust::operations::delegation_narrows` impose
l’expiration du fils au plus tard à celle du parent. Le runner
`git_reference_transaction` vérifie déjà son échéance monotone avant les octets
COMMIT ; son callback synchrone doit rester sous contrôle de cette échéance.
Le helper Git ordinaire ne reçoit actuellement que le timeout opérateur.

Candidat de contrat, non réalisé : sous le verrou partagé déjà pris, retenir
la plus proche expiration entre autorité signée et mandat validé de l’accord.
Associer UTC et monotone sans rajeunir la validité lors d’un recul de l’horloge.
La limite effective des commandes doit rester le minimum du bail et de la
politique Git persistée (aucun délai fixe ni nouvelle marge arbitraire).
Recontrôler le bail avant décision de Commit et après les lectures imbriquées ;
la même limite monotone doit atteindre le runner de transaction et ses lectures.
Une erreur après publication possible conserve unknown ou l’effet connu, sans
refus impliquant une absence. Reconcile reste une lecture sous ses droits propres.

Critères : horloge injectée en test pour bail futur/passé/frontière exacte,
recul UTC, expiration monotone, expiration UTC plus rapide, minimum mandat/autorité,
minimum opérateur/bail, erreurs de représentation et causes conservées. Une
lecture Git imbriquée ne doit pas repartir avec une nouvelle durée complète.
Les valeurs de dates/durées de ces tests sont des entrées de scénario, pas
une politique produit. Le parcours natif sur fixture neuve reste requis.

Limite substantielle : une échéance côté client ne garantit pas que le
processus Git publie avant l’expiration après réception de COMMIT (suspension,
ordonnancement, stockage). Aucun message de Git update-ref ne porte ici de bail
authentifié imposé au point d’effet. Le contrat proposé borne l’admission et
l’attente contrôlées, sous hypothèses d’horloge, pas le point physique de publication.
Une garantie PA/H2 plus forte exige un contrat du récepteur au point d’effet
et ses hypothèses ; elle ne sera pas déduite d’un timeout ou d’un test vert.
Ce point reste ouvert et doit être explicite dans l’article.

P3, précision issue de la lecture du producteur :
`prepare_paea_git_repository` lance déjà un contrôle bare Git avant de rendre
le dépôt, et l’inspection du candidat compose d’autres observations. Le bail
proposé ne doit pas être présenté comme un watchdog du callback complet ou
des verrous FS/SQL. Après tout prévol lent, une expiration passée interdit
la transaction ; recalculer le temps restant depuis l’expiration absolue ne
doit jamais recréer une durée de validité. Le runner de transaction et sa
lecture sous verrou Git constituent le périmètre à raccorder explicitement,
avec erreurs observées et limites des opérations amont conservées.


Historique du checkpoint P1, conservé :


Checkpoint Git WIP P1 : branche isolée depuis main230d4b2, sixsources et documents P1 seulement. Les constantes HELP étrangères ne sont pas importées. Résultats conservés sur la composition de qualification2109, pas une exécution du profil de ce commit isolé ; aucun rejeu effectué pour cette sauvegarde. Tamarin/oracle/annexes et P2Reconcile/bail/concurrence restent des lots séparés, pas une présence ou conformité garantie par cette branche. Revue09 P1-TEXT-01 close, sans admission Git/réseau, installation ni PV2globale. Preuves/index/restauration : /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p1-checkpoint-6jdcin6i.

Checkpoint P2 WIP : source Reconcile relue09, recordhistorique distinct refGitactuelle, exports/types/testsignatures adaptés. Dix gardes initiaux puis même gardeGit étendu FAIL/PASS, pas11tests ; --no-run antérieur typecompatibilité seulement. Branches/HELP étrangers exclus du commit ; preuves issues de composition2109, pas qualification du profil de ce commit isolé. Reproduction signée/native finale, bail/P4, runtime/install/main/PV2globale restent ouverts. Index /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p2-checkpoint-qqdlrdiw.
