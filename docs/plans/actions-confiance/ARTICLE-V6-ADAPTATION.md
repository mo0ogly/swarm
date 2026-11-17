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
| P1 | État d’effet typé indépendant de l’erreur et de l’autorité ; unknown après effet possible ; causes et résultats connus conservés | Candidat intégré ; perte de résultat reproduite puis corrigée sur composition figée ; six régressions moteur couvertes (cinq PASS r2 et une PASS r3 après correction du seul libellé attendu). Vérification de types CLI/web et revue 09 en cours ; parcours Git/HTTP et installation non vérifiés |
| P2 | Reconcile relit la référence liée aux témoins physiques, sans publication ni nouvelle autorité Commit | Non réalisé ; cas journal absent, intention seule, effet durable, référence ancienne/candidate/autre et erreurs de lecture |
| P3 | Expiration par bail dont les hypothèses sont explicites | Non réalisé ; un timeout du client ne borne pas le point d’effet Git. Justifier horloge, borne et récepteur avant toute garantie PA/PR |
| P4 | Retrait concurrent pendant réservation partagée, ordonné par barrière | Non réalisé ; témoin valide, demande, blocage effectif, effet, libération et retrait effectif observés, sans sleeps comme preuve |
| P5 | Deux modèles TAMARIN avec/sans section et contre-exemple figure 3a | Modèles écrits ; six verdicts attendus obtenus avec Tamarin 1.12.0 / Maude 3.5.1. PR falsifiée sans verrou, vérifiée avec verrou ; témoins positifs vérifiés. Voir annexe PR et preuves P1/tamarin ; aucune conformité Rust/Git déduite |
| P6 | Annexe oracle sur les 83 cas A0, verdict par propriété et attribution explicite | Inventaire original 2324 fichiers relu sans écart. Paquet GLM ARTICLE-V6-ORACLE en cours, dossier article-v6-oracle-kjevtgz4 ; aucune annexe 83 verdicts encore produite ni validée |
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

Reprise actuelle : exploiter le résultat terminal du seul appel P1, examiner et
intégrer uniquement les changements attribués, reproduire/vérifier puis poursuivre
P2–P7 sans attendre une nouvelle demande humaine. Toute erreur déclenche replan.


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


Checkpoint Git WIP P1 : branche isolée depuis main230d4b2, sixsources et documents P1 seulement. Les constantes HELP étrangères ne sont pas importées. Résultats conservés sur la composition de qualification2109, pas une exécution du profil de ce commit isolé ; aucun rejeu effectué pour cette sauvegarde. Tamarin/oracle/annexes et P2Reconcile/bail/concurrence restent des lots séparés, pas une présence ou conformité garantie par cette branche. Revue09 P1-TEXT-01 close, sans admission Git/réseau, installation ni PV2globale. Preuves/index/restauration : /run/media/fpizzi/SWARM/swarm-evidence/root00-article-p1-checkpoint-6jdcin6i.
