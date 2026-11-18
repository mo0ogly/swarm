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
| P2 | Reconcile relit la référence liée aux témoins physiques, sans publication ni nouvelle autorité Commit | Candidat revu : dix gardes composants PASS ; garde symref pendante étendue, FAIL puis PASS après correction. Typage bibliothèque CLI/web par défaut PASS. Revue 09 APPROVE_SOURCE_AND_TARGETED_COMPONENT_CHECKS_ONLY ; sauvegarde WIP ef431735e142d2cd535171cf59d8357c0401e662. Ni installation ni acceptation globale |
| P3 | Expiration par bail dont les hypothèses sont explicites | Douze contrôles composants PASS ; typage CLI/web et Rustdoc PASS. Revue 09 APPROVE_SOURCE_AND_TARGETED_COMPONENT_CHECKS_ONLY ; sauvegarde WIP 9735bf483ec2b411451d7ceade85b1fa40d7ee38. Borne admission/attente, aucune garantie du point d’effet Git ou borne FS/SQL globale |
| P4 | Retrait concurrent pendant réservation partagée, ordonné par barrière | Q3B-R1 : un test natif nommé PASS ; recette réelle deux étapes PASS, intégrité VERIFIED. Revue 09 APPROVE_BOUNDED_NATIVE_FLOW_AND_SOURCE_ONLY sur vingt empreintes. Scénario coopératif borné ; composition du futur commit, installation et acceptation globale distinctes |
| P5 | Deux modèles TAMARIN avec/sans section et contre-exemple figure 3a | Modèles écrits ; six verdicts attendus obtenus avec Tamarin 1.12.0 / Maude 3.5.1. PR falsifiée sans verrou, vérifiée avec verrou ; témoins positifs vérifiés. Voir annexe PR et preuves P1/tamarin ; aucune conformité Rust/Git déduite |
| P6 | Annexe oracle sur les 83 cas A0, verdict par propriété et attribution explicite | Version r4 : 83 lignes relues, zéro error et verdicts identiques à r3 ; 2324 fichiers historiques inchangés, aucun rejeu. Treize contrôles sélectionnés PASS sur les octets corrigés ; 32 PASS antérieurs de portée conservée, suite complète de 34 non exécutée. Revue 09 source/delta ciblé approuvée ; WIP cf2423fed7082ac2e3db1de4852b2d20b703b532. PT et PV2_intention gardent un FAIL chacun ; aucune PV2 globale acceptée |
| P7 | Documentation FR/EN, revue indépendante, version, livraison locale et RETEX | Admission source revue (17 packages/342 cibles) ; profils standard et fixture c421 compilés, audit supplémentaire corrigé et cycle public neuf revus. Test concurrent exact c421 : 1 PASS, recette deux étapes PASS, intégrité VERIFIED et revue indépendante bornée. Outils de vérification : 37 gardes PASS, sauvegarde WIP b0c5ea2b… séparée. Graphe C/système/proc-macro complet, version gérée, installation, fusion main et RETEX restent ouverts |

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

Reprise actuelle du 18 novembre : revue combinée P4 reçue ; corriger le suivi
bilingue puis sauvegarder le lot attribué dans le worktree isolé. P7 reste ouvert.
Les sections chronologiques ci-dessous conservent leurs états historiques.


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

## Remise terminale : candidat P2 et correction P6

P2 : dix PASS moteur initiaux ; garde réel de référence symbolique pendante
FAIL avant correction, PASS après ; contrôle de types CLI/web r2 PASS, sur
compositions figées de 2109 entrées sans dérive. La cible native a compilé avec
`--no-run` avant le dernier changement du helper : aucune exécution native.
Empreintes courantes et limites : `article-v6-reconcile-c7z03olb/final-pins.json`
et `review-scope.md`.

P6 : cinq défauts reproduits. Le premier lot ciblé avant correction produit
8 FAIL et 1 ERROR de refus à l’exécution (second tampon d’index corrompu),
après témoins valides. Un second index valide substitué fait échouer le garde
avant correction, puis le garde passe après correction. Dix nouveaux gardes et
les 22 cas précédents passent ensemble : 32 tests nommés. Le lot complet est
justifié par les contrats partagés index/trace/IO modifiés sur tous les parcours,
pas par une mise à jour de compteur ; la justification est conservée.
La recette r2 est PASS/VERIFIED avant les seuls ajouts de prose ultérieurs ;
les octets du code, des tests et de la recette n’ont pas changé lors de ces ajouts.
L’évaluation réelle hors ligne r3 conserve 83 lignes, zéro verdict error et les
mêmes verdicts que la première application : PT/W_sensitivity et
PV2_intention/V06 en échec, PV1/PV2 non projetées inconclusives. Code de sortie 0
et empreintes du marqueur relus ensemble. Aucun rejeu de la campagne récepteur.
Six chemins gelés pour revue corrective :
`article-v6-oracle-repair-bpscx4x9/final-pins.json`.

P3 bail et P4 barrière concurrente ne sont pas implémentés. Le protocole P4 est
préparé dans ARTICLE-V6-CONCURRENT-WITHDRAWAL.md. Aucune allocation native,
installation, fusion main ou acceptation globale PV2/H2 n’est revendiquée.

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

## Sauvegarde Git P2 et prochain paquet de production borné

Swarm00 a sauvegardé et relu le push de
`ef431735e142d2cd535171cf59d8357c0401e662`, parent
`e90b2d436b2cc8310fb00a6f67639be1d3acd503`, sur
`codex/wip-article-v6-p1-20261117`. Le lot contient les dix chemins P2 et les
plans FR/EN capturés, avec les seules ancres d’export autorisées des fichiers
partagés. Il reste une sauvegarde WIP : ni fusion main, ni qualification du
profil livré, ni installation. Revue 09 :
`APPROVE_SOURCE_AND_TARGETED_COMPONENT_CHECKS_ONLY`. Les dix contrôles initiaux
et l’extension du garde Git réel ne représentent pas onze tests indépendants.
La documentation Rust a ensuite compilé sur 2109 entrées figées sans dérive ;
un avertissement préexistant de lien vers un élément privé reste distinct.

Le paquet de production sans outils `article-v6-lease-1ok7z3qz` est le seul appel
GLM actif. Périmètre : promotion attribuée et sources accord/dépôt géré actuellement
propres, avec préimages hachées. Il propose un bail dérivé des expirations absolues
signée et du mandat pour l’admission et l’attente contrôlées de la transaction,
y compris les lectures sous verrou Git et la politique de délai opérateur.
Aucun code P3 intégré ni testé. Il doit préserver les causes de lecture : l’ancien
fallback d’erreur générique sur OLD nul ne doit pas devenir un Commit ; les
observations P1/P2 sont conservées, sans garantie physique de publication Git
que le récepteur n’impose pas. Les six chemins P6 restent gelés séparément.

## Remise P3 et sauvegarde P6

P6 ROUND2 et son delta de prose ont reçu les revues ciblées de 09. Swarm00 a
sauvegardé/poussé `cf2423fed7082ac2e3db1de4852b2d20b703b532`, parent
`ef431735e142d2cd535171cf59d8357c0401e662`, même branche WIP. Treize contrôles
courants, 32 antérieurs, 34 prévus non tous exécutés ; r4 relit 83 rangées, pas
83 expériences rejouées. Pas de fusion main, installation ni acceptation scientifique.
Les deux phrases de documentation modifiées par Swarm00 sont liées dans son
DOC-DELTA.json ; les anciens manifests d’empreintes restent historiques.

P3 : proposition GLM terminée (378,741 s, 116825 tokens déclarés). Un ancien
extrait Rustdoc inexistant a bloqué la première substitution avant écriture
produit ; ligne réelle reprise. Le principal a aussi corrigé les fermetures
après refus de bail, l’échéance parente des lectures imbriquées, la cause tardive
conservée et les deux gardes tautologiques remplacés par des consommateurs réels.
Douze cas nommés passent sur composition déclarée figée de 2109 entrées.
Le typage CLI/web passe (41,67 s), Rustdoc passe (16,01 s), sans dérive source.
Un avertissement Rustdoc préexistant, hors lot, reste distinct.

Le contre-exemple de perte de cause RFC3339 échoue sur les trois préimages P2,
par le vrai matcher public et des opérations Trust de fixture. Le test compatible
ancien API retire seulement l’assertion du nouveau champ. Les octets JSON sont
restaurés avant l’assertion ; la fixture d’échec et les deux ELF de test sont
conservés avec hashes. Le scanner initial a classé neuf sorties de cette fixture
comme dérive source ; résultat original conservé, relecture des 2109 entrées
déclarées sans écart. La méthode corrigée distingue ces sorties et refuse
toute nouvelle source non admise. Un futur module P4 exige un nouvel inventaire.

Sources, contrats, politiques et causes : `article-v6-lease-1ok7z3qz/final-pins.json`
et `review-scope.md`. Six chemins gelés pour revue. Recette permanente créée,
non exécutée en entier ; commandes ciblées conservées. Le source suffit à tracer
le raccordement Admin/API/CLI du délai existant, pas son effet UI courant.
Aucun profil livré, parcours signé natif ou point de publication avant expiration
n’est qualifié. Le seul appel actif est le paquet P4 sans outils
`article-v6-withdrawal-f8vbbjjd` : proposition uniquement sur les ancres autorisées.
Son contexte a omis le point Commit après décalage de lignes ; omission conservée,
pas de seconde requête. Cette insertion sera dérivée des vrais octets P3 revus
par le principal, sans accepter un ancien texte inventé par GLM. Les autres
producteurs/interfaces P4 ont été fournis. Aucune intégration ni exécution P4.


### P4 — terminal / 2026-11-17T23:17:49.947989+00:00

Swarm00 a relu le résultat primaire P4 : réponse exploitable, fin `stop`, aucun retry automatique, 419,162 s ; 98 101 tokens déclarés par le fournisseur. Le propriétaire vérifie encore les barrières et fermetures : aucune exécution native, qualification ou acceptation nouvelle. La recette historique `paea-owned-git-promote.json` reste exclue de ce prochain lot ; une fixture neuve et une sélection nommée seront requises.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-terminal-p3onij2b/RECEPTION.json`.


## Reprise P4 — 18 novembre 2026

P3 sauvegardé par Swarm00 : `9735bf483ec2b411451d7ceade85b1fa40d7ee38`,
parent P6 ; WIP, sans fusion main ni installation. Gel P3 levé uniquement pour
l'insertion P4 autorisée au point Commit capturé. P4 GLM terminé, aucun appel actif.
Les défauts du candidat ont été conservés puis corrigés par le principal :
témoin Commit réel, contrôle EX hors section, canaux terminaux, jointure avant
assertion, ressources conservées et erreurs de fermeture indépendantes.
Cinq sources intégrées par CAS des préimages, inverses exactes des ancres
partagées ; FR/EN et recette du test mis à jour. Rustfmt du nouveau module et
de l'append valide la syntaxe seulement. Aucun build ni corps natif lancé.
Prochaine précondition : allocation par Swarm00 d'un inventaire effectif neuf,
profil natif par défaut et target de fixture distinct, puis première publication
native neuve et grants signés. Les 2109 anciennes entrées ne couvrent pas le
nouveau module. Critère H2 ouvert ; PA physique et PV2 globale non établies.


### P4 — fresh setup boundary / 2026-11-17T23:29:06.392310+00:00

Lecture de l’amorçage neuf : le helper first-publication exécute aussi des contrôles de refus, corruption/restauration, replay et collisions. Sa commande entière ne constitue donc pas une simple création de fixture. Le propriétaire doit déclarer le parcours public minimal et les opérations requises avant allocation ; aucun lancement effectué.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-fresh-setup-3z6ox16q/FRESH-SETUP-BOUNDARY.json`.


### P4 — source review / 2026-11-17T23:30:18.254841+00:00

Remise P4 intégrée par son propriétaire : huit empreintes vérifiées par Swarm00, revue indépendante 09 demandée sur ces sources, protocoles FR/EN et recette. Aucun build ni test lancé. Le graphe et l’amorçage public neuf restent à qualifier séparément.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/DISPATCH.json`.


### P4 — helper reservation / 2026-11-17T23:32:02.671152+00:00

Nouveau helper tests/paea_concurrent_withdrawal_source.py réservé au propriétaire P4 pour source seulement, depuis absence constatée. Défaut repéré dans la proposition : PASS écrit avant les comparaisons finales des empreintes. Correction demandée avant revue du parcours ; effet public déjà observé et qualification finale doivent rester distincts. Aucun lancement autorisé par cette réservation.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/FRESH-HELPER-RESERVATION.json`.


### P4 — REQUEST_CHANGES / 2026-11-17T23:33:03.598191+00:00

Revue indépendante P4 : REQUEST_CHANGES sur trois contrats — propriétaire de fixture d’autorité détruit au retour, causes de workers/paniques masquées par assertions, destructeur de callback sous emprunt mutable RefCell. Corrections transmises au propriétaire avec originaux et huit pins conservés. Les placements/cfg sont cohérents au source ; aucune preuve d’exécution acquise. Revue delta et qualification restent requises avant natif.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/REVIEW-09-REQUEST-CHANGES.json`.


### P4 — authority retention anchor / 2026-11-17T23:35:08.481117+00:00

Réservation complémentaire bornée : nouvelle méthode Fixture::retain_directory dans authenticated_fixture.rs, préimage propre épinglée ; aucun changement des constructeurs ni du close existant. Elle empêche la suppression automatique, sans prouver fermeture ou synchronisation. Le consommateur doit retenir le propriétaire avant open/use et réunir séparément les fermetures application/business/personal. Delta et qualification restent à vérifier.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/AUTHORITY-FIXTURE-RESERVATION.json`.


### P4 — revue source REQUEST_CHANGES et delta

Revue09 cursor90 : rétention d'autorité, conservation des causes lors des
assertions et destruction réentrante sous RefCell corrigées, non testées.
Swarm00 a réservé helper frais et seule méthode retain_directory du fixture
commun ; constructeurs/save/close préservés. Dix pins courants dans
article-v6-withdrawal-f8vbbjjd/candidate-pins-r2.json ; cinq inverses/prefixes
hors ancres conformes au source. 37 helpers Python statiques, 3125 candidats
pré-build déclarés ; aucun graphe effectif admis. Neuf gardes de module, deux
composants et un corps concurrent prévus, tous non exécutés. Premature-PASS
corrigé et reçu terminal exigé au consommateur ; publication observée reste
distincte de qualification. Échec du script d'édition conservé : première
insertion retain_directory appliquée, substitution native arrêtée sur une
ancre formatée différemment ; octets réels relus et reprise CAS sans répéter
la mutation déjà effectuée. Aucun lancement GLM/build/native supplémentaire.


### P4 — delta R2 / 2026-11-17T23:41:52.029175+00:00

Remise delta R2 : dix empreintes vérifiées puis transmises à la revue indépendante, incluant rétention d’autorité et helper frais supervisé. Trois corrections sont proposées, pas encore acceptées. Les neuf gardes, deux composants et le corps natif restent non exécutés ; les 3125 entrées déclarées ne qualifient pas le graphe effectif.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/DELTA-R2-DISPATCH.json`.


Delta de rédaction proposé, original DOCX intact : [revendications après adaptation](ARTICLE-V6-REVENDICATIONS-APRES-ADAPTATION.md). Les paragraphes distinguent tests ciblés, modèle, corpus A0, H2 non observée et absence de garantie PV2 globale.


### P4 — R2 residual / 2026-11-17T23:44:40.678119+00:00

Revue R2 : rétention et destruction de callbacks corrigées au source ; helper et reçu supervisé cohérents au source. R09-2 reste ouvert sur le témoin EX : deux unwrap perdent le résultat typé du probe/déverrouillage/retrait. Correction et garde ciblé transmis au propriétaire. Aucun test/build/native alloué ; graphe et profil restent non qualifiés.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/REVIEW-09-R2-REQUEST-CHANGES.json`.


### P4 — R3 source delta / 2026-11-17T23:46:39.515750+00:00

Delta R3 reçu et dix pins vérifiés : quatre fichiers changés, six inchangés. Conservation imbriquée du témoin EX et garde composant proposés à la revue indépendante. Aucun test/build/natif exécuté ; allocation de qualification du graphe et des cibles encore attendue.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/R3-DISPATCH.json`.


### P4 — Q1 allocation / 2026-11-17T23:47:31.099517+00:00

Revue R3 APPROVE_SOURCE_ONLY : trois défauts clos au source, aucune exécution déduite. Allocation P4-Q1 transmise : composition locale gelée, admission d’une seule cible fixture externe, neuf gardes et trois composants nommés après préconditions. Preuves effectives du graphe/profil/ELF requises ; aucun default build, bootstrap frais, corps concurrent ignoré ni installation dans Q1.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q1-ALLOCATION.json`.


### P4 — Q1 graph recovery / 2026-11-17T23:52:33.884526+00:00

Q1 arrêté avant rustc : Cargo exit101, zéro artefact/test ; agent_runtime_probe.rs omis de la composition. Le metadata de copie tronquée et la liste vide des racines manquantes ne qualifient pas la clôture. Replan : nouvelle composition depuis manifests réels, metadata du checkout et includes/supports, comparaison sémantique des targets avant reprise des seuls gardes/composants Q1. Aucun source produit ni manifeste contourné ; erreur originale conservée.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q1-GRAPH-REPLAN.json`.


### P4 — Q1 component results / 2026-11-18T00:00:47.927944+00:00

Q1 : neuf gardes et trois composants nommés PASS, sorties primaires/exit0/stabilité sources-registry et deux empreintes ELF relus par Swarm00. Le corps concurrent reste non exécuté. Reproducer minimal old-hook encore actif chez le propriétaire : aucune compilation doublée. Remise finale Q1, profil standard, naissance fraîche et natif restent ouverts ; graphe C/headers/system/procmacro non qualifié.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q1-COMPONENT-RECEPTION.json`.


### Q1 terminé — preuve de composants, pas H2 natif

R3 APPROVE_SOURCE_ONLY reçu09 cursor104. Neuf gardes + trois composants PASS ;
ancien module : un garde identique FAIL runtime RefCell, témoin sain préalable,
API compatible et ancien ELF conservé. Le cache unit a été remplacé par l'ancien
ELF ; seules copies candidates relues par SHA qualifient les PASS antérieurs.
La copie complète3498 et la clôture343targets remédient au premier101 de parsing
sans compilation, dont originaux et faux négatif metadata=[] sont conservés.
211archives/11887fichiers registry liés à Cargo.lock. Sourceprofil/ELF/depfiles
conservés sous article-v6-withdrawal-f8vbbjjd/q1, limites C/headers explicites.
Aucun natif/helper frais/default/installation. Gardes de refus de qualification
préparés dans un fichier privé, quatre cas mockés non exécutés, pour revue et
allocation suivantes. La recette permanente incluant le corps ignoré n'a pas
été exécutée intégralement ; les commandes Q1 seules sont des preuves.


### P4 — Q1 final handback / 2026-11-18T00:03:06.249792+00:00

Remise finale Q1 : contre-exemple ancien module 1FAIL runtime après build réussi, candidat neuf gardes+trois composants PASS réutilisés sans rejeu. Trois ELF conservés relus par SHA ; chemin du cache unitaire désormais ancien binaire, non réputé candidat. Revue indépendante des preuves et des seuls deux deltas docs demandée. Quatre gardes mockés de qualification réservés pour insertion/source-review ; aucune exécution nouvelle.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q1-FINAL-RECEPTION.json`.


### P4 — Q2 allocation / 2026-11-18T00:05:47.756872+00:00

Revue finale : Q1 APPROVE_COMPONENT_EVIDENCE_ONLY, quatre gardes maintenus APPROVE_SOURCE_ONLY. Allocation Q2A : quatre gardes mockés uniquement ; Q2B conditionnelle à leur PASS : qualification CLI standard sans features fixture sur une nouvelle cible externe distincte, sources/configuration refigées, image conservée localement. Aucun bootstrap public, corps concurrent, service/install/main autorisé par Q2. Les résultats restent à recevoir.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q2-ALLOCATION.json`.


### Q2 remis — préparation du natif, pas observation H2

Revue09 Q1 APPROVE_COMPONENT_EVIDENCE_ONLY ; gardes inertes SOURCE approuvés
cursor116. Q2A quatre PASS (38Python+JSON défaut effectif) sans enfant réel.
Q2B profil standard VERIFIED_DEFAULT_NATIVE_PROFILE, exit0,206artefacts dont
quatre centraux sans features et test=false,790entrées effectives archivées,
module fixture absent. Source3499/registry stables, nouvelle cible standard
séparée et six registrations conservant les cinq précédentes. Image locale
f429fde5cbe566cc1d741d60c15d53f6b4c5db46be2a4c71859f49e126e1f3fc,
installed=false. Preuves q2a/q2b sous article-v6-withdrawal-f8vbbjjd.
Pas de helper frais, corps natif ou service lancé. Q3 requiert allocation.
L'en-tête EN ancien était resté « préparation » (substitution initiale sans
assertion de concordance) : corrigé avec l'en-tête FR après lecture exacte ;
le source gelé antérieur conserve ses octets historiques. Pas de tests Q1
répétés pour ces seules phrases ou les gardes Python déjà exécutés.


### P4 — Q2 evidence reception / 2026-11-18T00:17:09.110044+00:00

Remise Q2 reçue : quatre gardes mockés exit0 ; record de profil standard VERIFIED_DEFAULT_NATIVE_PROFILE, processus exit0, source/registry stables. Empreintes du résultat et de l’image locale f429fde5… relues par Swarm00, onze pins actuels concordants. Revue indépendante des preuves et deux docs demandée. Aucun parcours natif métier, bootstrap frais, corps concurrent ni installation acquis.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q2-RECEPTION.json`.


### P4 — Q3A allocation / 2026-11-18T00:18:44.236607+00:00

Q2 approuvé indépendamment : quatre gardes simulés et preuve du profil standard uniquement, image locale f429fde5… non installée. Q3A alloué : un amorçage public frais supervisé par helper revu et cette image, fixture isolée originale et reçus retenus. Aucun ancien run/campagne, provider payant, SQL direct, corps concurrent ni installation dans cette allocation. Résultat primaire Q3A requis avant Q3B.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3A-ALLOCATION.json`.


### P4 — Q3A relay recovery / 2026-11-18T00:22:54.881258+00:00

Q3A échoue exit1 après worker manuel terminé : relais public NotApplicable sans native_mission/origine conducteur, producteur AutomaticWorker requis. Le seul watch attribué a reçu SIGINT ; fin externe exit1 conservée et sources/image/profil stables. task.submit exige déjà un résultat retenu, donc son existence ne démontre pas une alternative manuelle. Comparaison des parcours publics et correction du helper source seulement demandées, avec préflight d’éligibilité avant watch. La revue statique/mocks n’avaient pas établi ce raccord natif. Aucune relance, Q3B non alloué.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q3A-RELAY-REPLAN.json`.


### P4 — supervision incident / 2026-11-18T07:58:12.749828+00:00

Incident de supervision : à 07:56 UTC, chat propriétaire toujours actif mais cursor17 et dernier outil terminé inchangés, aucune preuve depuis environ 00:22 UTC. Cause moteur/app/quota non établie ; aucun cargo/rustc/swarm-rust observé par nom dans le snapshot. Diagnostic concret envoyé, livraison outil acquittée mais réception et reprise effective non établies. Aucun outil d’interruption/reprise de chat disponible ; aucun processus étranger arrêté ni appel/compiler dupliqué. Reprise sur nouvelle remise/progrès du propriétaire ou intervention explicite de l’application, sans mutation concurrente de ses sources réservées. Q1/Q2 conservés ; Q3A reste en correction.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/OWNER-PROGRESS-INCIDENT.json`.


### P4 — owner reception / 2026-11-18T07:58:48.796337+00:00

Réception de l’alerte maintenant observée : le propriétaire reconnaît l’absence de progrès prouvé, sans cause quota/outil établie. Il rapporte fin watch/child exit1 et pause publique, sans terminal qualifié. Route de correction source : un conducteur CLI authentique depuis TODO sans agent, planning en pause, préflight et arrêt événementiel sur handoff/exit0 du même worker. Aucune exécution allouée. Les événements drainés invalident l’inférence « stdout vide = aucun événement » ; la correction et sa preuve restent ouvertes.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/OWNER-PROGRESS-RECEPTION.json`.


### Q3A replan — défaut du helper, pas du droit moteur

Échec réel conservé q3a/result exit1, tous hashes stables, aucun terminal.
Watch arrêté normalement par pidfd lié au root/image ; receipt root/lease fermés,
zéro enfant actif ; pause publique0. Contrat manuel+relais automatique invalide.
Correction SOURCE : conducteur CLI d'une TODO fraîche, planning pausé, aucun
agent au préflight avant Popen ; admission native moteur puis handoff ET vrai
wait0 avant arrêt. Pas de serveur/SQL/origin forgé ou appels implicites de rôles.
Helper/test/recette/docs actualisés, huit gardes inertes prévus non exécutés,
relecture revue et nouvelle allocation requises. Rust/imagef429 inchangés.
Incident supervision >7h conservé ; reprise prouvée par écritures source datées,
aucune cause quota/outils établie et aucune clôture native. Erreur locale de
script (ancre __main__ espacée inexistante) conservée : helper déjà écrit,
tests non écrits ; vraie ancre relue, seul lot tests repris, pas mutation répétée.


### P4 — automatic helper source / 2026-11-18T08:05:49.008910+00:00

Reprise SOURCE effectivement observée : cinq fichiers changés, onze empreintes actuelles vérifiées ; helper automatique et huit gardes inertes remis puis transmis à la revue indépendante. Les Rust et image standard inchangés gardent leurs preuves, sans rejeu. Admission réelle/event keys/arrêt/cleanup sont examinés ; zéro nouvel essai natif, huit gardes non exécutés sur ces octets. Incident de supervision conservé, réparation runtime non acquise.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/AUTOMATIC-SOURCE-DISPATCH.json`.


### P4 — transient relay observation / 2026-11-18T08:08:48.881597+00:00

Revue automatique REQUEST_CHANGES : séquence valide started→waiting→exited0→submitted refusée prématurément ; managed-operation-busy et certaines absences de worker prêt sont transitoires selon producteur. Correction de classification et gardes d’ordre transmis, sans délai/retry ajouté. Conducteur/admission native/gates sous planning pausé cohérents au source. Aucune nouvelle exécution, Q3A historique reste FAIL.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/AUTOMATIC-REVIEW-REQUEST-CHANGES.json`.


Q3 source : finding09 cursor134 corrigé sans exécution : waiting/blocked restent transitoires même après wait0 ; seuls vrais événements terminaux/EOF refusent. Onze gardes inertes écrits, zéro nouveau run. Les pins et Finding initiaux sont conservés.


### P4 — automatic inert guards allocation / 2026-11-18T08:12:30.084436+00:00

Correction transitoire APPROVE_SOURCE_ONLY. Allocation bornée : un garde inerte contre ancien helper compatible, avec témoin valide, puis onze gardes sur composition corrigée figée. Résultats encore attendus ; aucune CLI/watch/fixture publique ni compilation. Rejouer les anciens quatre gardes est justifié par changement du helper et de son graphe d’import ; preuves Rust/default inchangées réutilisées. Nouvelle naissance publique attendra remise primaire.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3A-R1-GUARDS-ALLOCATION.json`.


Q3A-R1 gardes : source09 approuvé cursor137 ; une méthode ancienne reproduit ERROR runtime après contrôle sain, puis onze méthodes candidates PASS, entrées39 stables et preuves archivées. Aucun lifecycle natif lancé. Q1/compilationdefault non répétés. Voir q3a-r1-guards/summary.json et protocoles FR/EN.


### P4 — eleven guards and new public allocation / 2026-11-18T08:16:52.810710+00:00

Onze gardes inertes nommés PASS et contre-exemple ancien helper ERROR runtime vérifiés. Archives et39sources de chaque composition relues ; première comparaison root avait doublé tests/tests via mauvais cwd, corrigée depuis summary.local sans modification/rejeu ; envoi dépendant trop tôt tracé. Nouvelle allocation Q3A-R2 confirmée après readback : une naissance publique automatique fraîche supervisée, image f429/profil inchangés. Aucun corps concurrent ni installation alloué ; preuve primaire nouvelle attendue.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3A-R2-PUBLIC-ALLOCATION.json`.


### P4 — fresh source and single concurrent allocation / 2026-11-18T08:19:59.426693+00:00

Naissance publique fraîche Q3A-R2 vérifiée par Swarm00 : child/supervise exit0,117artefacts/2originaux/marker/inode relus ; même worker exit0 puis remise, watch arrêté children0/bail et Root fermés. Nouvelle fixture isolée, acceptation du produit non déduite. Q3B alloué séparément : un seul corps concurrent ignoré via ELF fixture conservé d4993331…, sans recompilation ni campagnehistorique. Résultat et revue finale encore attendus ; aucune installation.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3B-ALLOCATION.json`.


Q3A-R2 public : exit0 et reçu qualifié, vrai worker automatique/handoff/wait0, arrêt/pause démontrés, trois contrôles et reviewer inert puis publication unique/readback accepted. Root original local et inode conservés ; q3a-r2/qualified-fixture.json fait index. Image/profile/code stables, tous hashes de qualification relus. Ni humain/LLM réel, ni retrait concurrent observé ; Q3B séparé avant clôture. Pas de replay des contrôles antérieurs.


### Q3B — échec conservé, consommateur du schéma corrigé à préparer

L’unique test natif alloué termine exit101 (1 nommé, 0 PASS, 1 FAIL), après
le témoin Commit signé et le probe EX positif. Les contrôles de contention
EAGAIN sur le même inode, Trust inchangé pendant SH, engagement enregistré et
réponse de retrait public revoked ont été franchis. L’assertion de relecture
attendait à tort mandates[id].state : State.public projette les mandats bruts,
dont le retrait est représenté par revoked_at, et un champ séparé mandate_status.
L’état original conservé contient le revoked_at du mandat concurrent.

Cause : contrat du consommateur de test inventé, et non refus du retrait établi.
La suite refus de nouvel usage/Reconcile n’a pas été atteinte. La réparation
portera sur les assertions du test, sans modifier le schéma produit ni l’état
persisté. Nouvelle compilation et exécution exigent une allocation distincte ;
aucun rejeu inchangé. Primaires : article-v6-withdrawal-f8vbbjjd/q3b,
stdout/stderr/result/invocation/diagnosis et source-before.rs. ELF d499 et
entrées Rust compilées stables ; aucune installation, H2 globale non établie.


### P4 — Q3B schema recovery / 2026-11-18T08:26:23.115590+00:00

Q3B unique : exit101, un test nommé FAIL, aucun PASS. Témoin Commit signé/EX valide franchi ; assertion1430 attendait mandates[id].state inexistant. Producteurs relus : show→mandates bruts avec revoked_at et statut effectif séparé mandate_status (« mandat révoqué »). Correction réservée au consommateur de test/docs/recette ; pas produit ni SQL modifié. Refus/Reconcile après l’assertion non exécutés, aucun H2PASS. Original conservé, revue source et nouvel ELF requis avant réallocation.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q3B-SCHEMA-REPLAN.json`.


### P4 — Q3B-R1 qualification preparation / 2026-11-18T08:27:47.276957+00:00

Q3B-R1 APPROVE_SOURCE_ONLY : unique correction du consommateur de projection Trust, sans affaiblir les contrôles. Allocation de préparation : compilation exacte de la seule cible intégration avec --no-run, nouveau ELF retenu ; puis, si qualifiée, nouvelle fixture publique originale via helper inchangé/default f429 réutilisé. Ancien Root muté et FAIL conservés, aucun compteur réinitialisé. Aucun corps concurrent ni installation alloué ; reçus nouveaux requis avant essai.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3B-R1-PREP-ALLOCATION.json`.


### Q3B-R1-PREP — compilation et nouvelle naissance, incident de méthode ouvert

Le seul ELF d’intégration corrigé compile sans exécuter de test : exit0,
build-finished success,118 artefacts, sources et registre stables. Copie conservée
SHA256 08b99e72de0794d2dc22a9a66620775e25c2cbe22df8088fb784aaae3cc5dce1,
feature test-fixtures/profil test, distincte de l’image par défaut f429.
Composition locale : .cache/article-v6-p4-q3b-r1-uud5mnox ; seule entrée Rust
modifiée depuis Q1 : l’assertion de schéma du test.

Le vérificateur de depfiles a d’abord résolu les chemins relatifs depuis la
racine de composition au lieu de son sous-dossier rust. Son assertion a échoué.
Une commande shell mal séquencée a néanmoins lancé la naissance dépendante B :
violation de notre méthode, conservée dans method-incident.json. Le resolver
corrigé normalise depuis rust ;101 depfiles donnent520 sources workspace toutes
déclarées,1070 registre,3 générées ; aucun manquant ni désaccord. Aucun build
rejoué. Les prochaines étapes dépendantes passent par des appels séparés ;
le runner futur exige aussi ce reçu avant toute création. Ses octets exécutés
restent conservés séparément. Cette mesure n’efface pas l’incident antérieur.

B a terminé proprement child/supervise exit0, mais son acceptation reste à
décider par Swarm00 à cause du démarrage prématuré. Reçu brut : q3a-r3. Root
original LOCAL : .cache/article-v6-p4-q3a-r3-dqn_ov3t/.swarm-fixtures/
swarm-model83-l1k8i7mu, .swarm dev64513/inode19057820 ; ne pas nettoyer ni
substituer de copie. Les originaux Q3A-R2/Q3B restent conservés avec leurs
mandats et l’échec initial. Le corps concurrent corrigé n’a pas été exécuté.
Primaires : article-v6-withdrawal-f8vbbjjd/q3b-r1-compile et q3a-r3.


### P4 — PREP sequencing incident / 2026-11-18T08:33:39.453087+00:00

Incident de méthode : B a démarré après erreur d’auditdepfiles. Après correction de base S/rust, Swarm00 a vérifié3499sources,1593dépendances,101depfiles et520workspaceprébornés ; nouveauELF08b99… conforme. B termine0 avec117artefacts/originaux/marker/inode concordants. RésultatsA/B admis individuellement aprèsaudit, séquencement historique resteFAIL. Avantcorpsnatif : préflight exécutable bloquant +recetteétapesarrêtéessuréchec etgardesinertes valides/absent/hashfaux àremettre. Aucunrejeu/recompile/bodyalloué.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/Q3B-R1-PREP-RECOVERY-DECISION.json`.


Préflight séquentiel source et trois gardes inertes PASS (q3b-r1-preflight-inert-r1), recette retained-ELF distincte sans Cargo ; A/B admis après audit, défaut méthode conservé. Revue source et allocation corps encore attendues. Voir protocole FR/EN.


### P4 — fail-closed preflight source review / 2026-11-18T08:40:07.190765+00:00

Préflight/recette2étapes source+trois preuves inertes remis,14pins vérifiés puis revue indépendante demandée. Corrections additionnelles signalées avant verdict : lier ELF au manifestesources et maps/reçus eux-mêmes ; root relèvevalidationminimum>=0affaiblie/test0 etabsence rehashimmutablepostchild. Propriétaire autorisé source/inertseulement, paquetfinalfigé requis etrevue avertie ; aucuneexécutionréelle/body, ancien3PASSnequalifiepasnouveauxoctets.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/PREFLIGHT-SOURCE-REVIEW-CONTEXT.json`.


### P4 — preflight review changes / 2026-11-18T08:41:19.910903+00:00

Revue préflight REQUEST_CHANGES : réserve0/schemaaffaibli, absence rehashimmutablepostchild etmanifestc5e4d614sansreceipts requisparcodecourant. Deuxfichiersmobiles,3PASSanciensoctetsnonapplicables. Rootetrevueaccordentl’arrêtfailclosed de recipe.py, sansqualifiernouveauchecker. Paquetfinal source/test/manifeste/docs figé demandéavantrevuedelta ; aucuneexécutionréelle/corps.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/PREFLIGHT-REVIEW-REQUEST-CHANGES.json`.


Préflight final après REQUEST_CHANGES09 : politique positive via validateur existant, relecture immuable après étape et agrégation des causes ; manifests liés. Deux contre-exemples préimage runtime conservés puis sept gardes inertes PASS et11bundles vérifiés. Manifeste final66d2… et14pins remis, sans préflight réel ni corps natif ; historique séquencement toujours en échec, pas d’installation.


### P4 — qualified two-step execution allocation / 2026-11-18T08:46:36.888168+00:00

Préflight final APPROVE_SOURCE_AND_INERT_EVIDENCE_ONLY :14pins/manifest66d2 relus inchangés,7PASS et11bundlesinertes validésdansportée. Allocation unique de recette bornée : préflight réel puisseulementsisuccès un corps natif08b99…, rechecksavant/après/FD etarrêtfailclosed. Verifyintégritéaprèsrunautorisé, aucunrejeudegardes/build/historique. Résultatencoreinconnu ; pasH2global/install/main accepté.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-Q3B-R1-RUN-ALLOCATION.json`.


### P4 — native concurrent primary result / 2026-11-18T08:49:17.452621+00:00

Q3B-R1 primaires relus : recette réelle2étapes exit0/PASS, un test concurrent nomméPASS0FAIL0ignored ; marqueurs contrôlevalide/contention-retrait-refus-Reconcile etchild0 présents, stderrvide.14pins/ELF/manifeststables, verifyintégritéVERIFIEDsansrejeu. Preuvebornée àceparcoursisolé, pasPV2global/expirationphysique/install/main. Remisefinalefigée etrevueindépendanteP4 restentrequises ; anciensFAIL/incidentsconservés.

Evidence: `/run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q/P4-NATIVE-PRIMARY-RECEPTION.json`.


### Q3B-R1 — observation concurrente native vérifiée

Après APPROVE_SOURCE_AND_INERT_EVIDENCE_ONLY09 et allocation root distincte,
une seule recette retained-ELF a été exécutée. Préflight réel PASS, puis test
concurrent exact PASS : 1 nommé, 0 échec, 0 ignoré, 8 autres filtrés,5,63s.
La recette entière termine exit0 ; recipe verify termine exit0. Les14pins,
ELF08b99… et manifeste66d2… sont stables avant/après, admission FD externe et
interne vérifiée. Aucun changement de source pendant ces étapes.

Le témoin Commit signé et le probe EX libre passent. Pendant la vraie section
SH de Commit, le retrait public entre dans le chemin EX ; EAGAIN est constaté
sur le FD vérifié, même device/inode64513:19057828 que le verrou attendu. Trust
reste inchangé pendant la section tenue ; après libération, engagement enregistré,
ref candidate observée et mandat publiquement révoqué sont vérifiés. Un nouvel
usage signé refuse agreement-stale sans nouvelle ref ; Reconcile retourne
séparément effet historique et ref actuelle. Les fermetures/jointures et causes
sont vérifiées par le test ; diagnostics startup/action/cleanup sans erreur.

Root source original LOCAL : .cache/article-v6-p4-q3a-r3-dqn_ov3t/.swarm-fixtures/
swarm-model83-l1k8i7mu, .swarm dev64513/inode19057820 (différent de l’inode du
fichier verrou). Deux services isolés et quatre fixtures d’autorité signée
supplémentaires sont conservés et indexés par stdout. Leurs noms temporaires
ne sont pas une autorisation de ménage. Aucun modèle LLM, serveur accepté,
présence humaine ou confinement OS n’est démontré.

Primaires : article-v6-withdrawal-f8vbbjjd/q3b-r1-run/{invocation,result,verify}.json
et recipe/step-0002.{stdout,stderr}, archives et empreintes du runner permanent.
Les échecs initiaux et incidents de méthode restent conservés, sans remboursement
de tentatives ni réattribution des anciens PASS. Observation H2 bornée à ce
scénario coopératif ; pas de garantie d’expiration physique au COMMIT, de PV2
globale, de main ou d’installation. Revue combinée finale encore requise.


## Retained earlier isolated-checkpoint history

These are preserved historical checkpoint passages, not current acceptance.

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

P3 lease and P4 concurrent barrier remain unimplemented. P4 preparation is
recorded in ARTICLE-V6-CONCURRENT-WITHDRAWAL.md. No native allocation, build,
installation, main merge or global PV2/H2 acceptance is claimed.

Historique du checkpoint P1, conservé :

Checkpoint Git WIP P1 : branche isolée depuis main230d4b2, sixsources et documents P1 seulement. Les constantes HELP étrangères ne sont pas importées. Résultats conservés sur la composition de qualification2109, pas une exécution du profil de ce commit isolé ; aucun rejeu effectué pour cette sauvegarde. Tamarin/oracle/annexes et P2Reconcile/bail/concurrence restent des lots séparés, pas une présence ou conformité garantie par cette branche. Revue09 P1-TEXT-01 close, sans admission Git/réseau, installation ni PV2globale. Preuves/index/restauration : /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p1-checkpoint-6jdcin6i.

Checkpoint P2 WIP : source Reconcile relue09, recordhistorique distinct refGitactuelle, exports/types/testsignatures adaptés. Dix gardes initiaux puis même gardeGit étendu FAIL/PASS, pas11tests ; --no-run antérieur typecompatibilité seulement. Branches/HELP étrangers exclus du commit ; preuves issues de composition2109, pas qualification du profil de ce commit isolé. Reproduction signée/native finale, bail/P4, runtime/install/main/PV2globale restent ouverts. Index /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p2-checkpoint-qqdlrdiw.

Checkpoint P6 WIP — oracle A0 (source et delta ciblés relus) : six fichiers maintenus, corpus original non importé ni muté. Les cinq défauts et deux variantes résiduelles ont reçu leurs gardes causaux ; la dernière portée est 13 contrôles ciblés PASS, 32 PASS historiques sur anciens octets, manifeste34 non exécuté globalement. R4 lit83rangées existantes,2324empreintes inchangées,0erreur et aucun changement de verdict observé ; ce ne sont pas83nouveauxeffets ou une acceptation scientifique/Rust/PV2globale. Deux phrases FR/EN sont qualifiées historiquement à r1, sans date inventée ni rejeu pourprose. La recette est versionnée, pas déclarée VERIFIED sur la version34. Source de sauvegarde seulement ; P3bail/P4native/profil livré/main/install restent ouverts. Index /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p6-checkpoint-rpcsdb4u.

Checkpoint P3 WIP — bail d’admission/attente contrôlée relu09 : minimum expiration signée/mandat, échantillon monotone avant UTC, deadline fixe non renouvelée et consommée dans les lectures imbriquées, minimum politique opérateur existante, causes/fermetures et P1/P2 conservés. Douze gardes composants actuels PASS, RFC3339 sourceP2 FAIL→candidat PASS, typage et Rustdoc réussis sur composition2109 ;9sortiesfixture ne sont pas des sources supprimées. Recette permanente7étapes déclarée, non exécutée intégralement. Aucune borne physique publicationGit/FS/SQL ni PA/H2/PV2/natif signé/profil de ce commit/install/main acceptés. P4hooks ne sont pas dans ce checkpoint ; productionfixture et nativefreshqualification restent séparées. Index /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p3-checkpoint-jkm1oxa8.


## Portée de la sauvegarde WIP P4 isolée

Cette sauvegarde P4 de vingt chemins conserve seulement les insertions attribuées, outils de fixture/vérification et documents bilingues revus ; les passages des sauvegardes isolées précédentes restent conservés ci-dessus. La revue09 approuve le scénario natif/source borné puis les trois corrections documentaires. Le PASS natif appartient à ses entrées composites conservées ; cette branche assemblée séparément reste non qualifiée. Aucun nouveau test, fusion main ou installation annoncé. Index de preuves et restauration : /run/media/fpizzi/SWARM/swarm-evidence/root00-p4-review-z02o085q. Qualification et livraison P7 restent ouvertes.

## P7 — profil fixture et parcours borné du checkpoint c421

Le producteur de vérification GLM a été intégré après corrections reproduites
et revues indépendantes. Les 37 gardes inertes canoniques passent. La nouvelle
compilation c421 termine Cargo0/118artefacts ; son contrôleur reste1 après un
refus lexical trop large des chemins `..`. L’audit supplémentaire corrigé, sans
recompiler, lie197depfiles,764sources workspace,2474registre et3générés, sans
entrée manquante/non liée ; ELF fixture f837ad04… retenu séparément du CLI
standard364bc… local. Les deux preuves ont leur admission indépendante.

Un cycle public neuf scripté (2planificateurs,1worker,1reviewer,3contrôles) puis
un seul test concurrent exact ont été vérifiés et revus :2étapes recette PASS,
1test PASS/0FAIL/0ignored/8filtered. La tâche de fixture est publiée, la mission
reste non acceptée. L’inode64513:17181246 désigne `trust-actions.lock` du projet ;
l’ancien libellé erroné « service » est conservé dans la preuve d’erratum.
Aucun LLM réel, graphe C/système/proc-macro complet, globalH2/expiration physique,
installation ou fusion main acquis. Les 122fichiers de preuve copiés conservent
bytes/modes/argv et les originaux ; la copie n’est pas une nouvelle exécution.

Preuves : `/run/media/fpizzi/SWARM/swarm-evidence/root00-p7-fixture-controller-ndtbeuz7`. Les erreurs, refus initiaux et compilations sont conservés ;
aucune campagne historique rejouée. Ce lot de cinq outils/tests/docs de
vérification n’altère pas l’application installée ni ses notes de version.
Leur sauvegarde Git ultérieure reste distincte du checkpoint applicatif c421
exercé et des acceptations globales encore ouvertes.
