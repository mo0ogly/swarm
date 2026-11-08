# N2 — Cadrage indépendant X-IDEE-12 / Kaelan

Remise du 8 novembre 2026. État : cadrage documentaire terminé ; scénario non exécuté, défaut Swarm non démontré, développement non attribué.

## Autorité, état et méthode

Attribution reçue du coordinateur : lecture et cette seule sortie, écrite atomiquement. Aucune source, recette, configuration, index Git, file nocturne ou documentation commune modifiée. Aucun test, build, fournisseur, service, commit, push, nettoyage, agent ou autre chat lancé. La fenêtre nocturne et les droits généraux de NUIT.md n'élargissent pas cette attribution particulière.

Checkout résolu : /home/fpizzi/workspace/swarm → /home/fpizzi/Documents/ChatGPT/swarm. Branche observée main, HEAD d084c1487726f0d240de84d521d19d14d7938250. État initial : nombreux fichiers modifiés et non suivis, notamment Rust, UX et qualification ; aucun de ces travaux n'est attribué à N2. Le binaire en cours, sa version et les services n'ont pas été interrogés : non nécessaires au cadrage, aucune conclusion d'installation.

AGENTS.md (66–83 et 438–454), CONTRACT.md, skills personnelles swarm-research et replan, méthode replan du projet, STRATEGIE.md, CONCURRENCE.md, DISCUSSIONS.json, NUIT.md, NUIT-QUEUE.json et CONTEXTE.md de l'axe consultés. La file lue marque encore N2 pending, worker null ; l'attribution conversationnelle ultérieure autorise ce cadrage seul. Gi.eth est signalé actif par l'attribution et la file, sans nouvelle inspection de son chat. Les états du registre sont historiques, pas preuve d'activité actuelle.

Replan appliqué à la reprise : besoin → code/documentation/proofs existants → inconnues → plus petit prochain contrôle. Les recherches larges ont produit des sorties tronquées ; elles ont été remplacées par lectures de fichiers et passages ciblés. Aucun raisonnement d'absence ne repose sur la partie tronquée. Pas de preuve d'exécution créée : la sortie locale unique est imposée par l'attribution ; aucun transfert USB n'est entrepris.

## Source et portée du besoin

Source : [Kaelan Yiu](https://x.com/kaelanyiu/status/2119504309708882341), original conservé dans docs/communication/x-swarm-20261107/lot16-x-sources.json, champ snapshot, ligne interne 117. Le passage conservé décrit : arrêt silencieux, remise annoncée comme démarrage, ordre ciblé transformé au relais en interdiction de relancer la discussion, diagnostic de blocage imprécis. Original relu localement ; aucune nouvelle consultation X. Il s'agit de son témoignage sur son environnement, pas d'une reproduction ni d'un défaut Swarm.

docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md:1099–1107 transforme ce témoignage en X-IDEE-12 et propose déjà A/B, deux relais, arrêt ciblé, altération, maintien de B et reprise A distinctement autorisée. docs/plans/AGENT-CONTROL-POCS.md:68–70 et 2687 rapproche POC-04/06. Deux relais constituent ici une variante de recherche proposée, pas un nombre de relais établi par le témoignage.

But retenu : vérifier que la cible, l'autorité et le sens d'un ordre restent attribuables jusqu'au consommateur ; distinguer remis, reçu, décision appliquée, fin effective, effets conservés et reprise. Ne pas assimiler fin de processus et absence d'effet métier.

## Inventaire borné : spécification, code lu, preuves

| Contrat / affirmation | Passage réellement lu | Portée et limite |
| --- | --- | --- |
| Décision d'arrêt attribuée, événement courant reçu, tâche/tentative/propriétaire, fin confirmée | docs/architecture/RUN-SUPERVISION.md:97–129 | Spécification du parcours Go planning decide, pas preuve de deux relais |
| Réception, décision et effet doivent être distingués ; fixture ne prouve pas autonomie | CONTRACT.md, sections 3–4 | Règle de travail et d'acceptation ; aucune garantie de transmission par sa seule présence |
| Correction de revue conservée dans retry_directive, séparée de Task.Next | AUTORECURSIVE-RECOVERY.md:204–216 | Documentation d'un autre contrat de reprise ; implémentation de retry_directive non inspectée ici, ne pas la réattribuer à stop-run |
| Stop lié à une observation de la tâche en cours et à son scope | internal/engine/run_supervision.go:125–195 | Code Go lu : runStopDecision, runStopRequest, applyRunDecision, validateRunDecisionTx ; aucune invocation |
| Validation SQL du processus dans la transaction de décision | internal/engine/planning.go:420–447 | Appel de validateRunDecisionTx pour continue-run/stop-run ; pipeline intégral claim/bail/budget non réaudité |
| Opération structurée et instruction au planificateur | internal/engine/planning_schema.go:46 ; planning_prompt.go:14 | kind stop-run existe ; consigne id/next, autres champs vides ; aucune fidélité sémantique prouvée |
| Consommateurs du stop enregistré, gestion de course avec fin du processus | internal/engine/agents_process.go:440–482 ; agent_terminal_process.go:340–377 | Lecture du stop lors de la fin ; pas audit complet du signalement, descendants ou tous chemins d'erreur |
| Tests Go pour arrêt effectif root/child, réouverture/rejeu, continuation et refus | internal/engine/run_supervision_test.go:66–207 ; noms de test 209–239 | Code de tests lu ; non exécuté cette nuit, root/child sont variantes, pas preuve de deux relais ni A/B simultanés |
| Signaux Rust de fin/remise attribués au scope | rust/crates/swarm-engine/src/agent_scope_signals.rs:1–108 | ended/handoff/append lient tentative/scope et refusent conflits ; ce n'est pas un canal d'ordre stop |
| Contrôles planning Rust | rust/crates/swarm-engine/src/planning_control.rs:1–165 | Codec, actions de contrôle et limites documentées ; lecture insuffisante pour conclure à la parité stop-run avec Go |
| Stop global de mission Rust | rust/crates/swarm-engine/src/mission_control_write.rs:1–105 | stage suspend les départs, révoque les claims et écrit consent=false pour Stop ; la fonction elle-même ne signale pas le processus ; ne convient pas telle quelle pour arrêter A en préservant B dans la même mission |
| Révocation effective et incertitude durable prévues par recette | tests/rust_mission_controls_revocation.py:scenario/main ; tools/verification/recipes/mission-controls-revocation.json | Code lu : pair réellement entré avant révocation, arrêt public de mission, attente TERM, reçu durable, coûts/producteur d'origine préservés, aucune acceptation fabriquée. Ni deux relais ni arrêt ciblé A/B établis |

Lecture accessoire : début de agent_supervision.rs (observation native et distinction règlement/acceptation). Aucun audit complet du runner Rust effectué. Les empreintes d'un fichier ne signifient jamais lecture intégrale ni qualification de toutes ses fonctions.

### Preuves historiques réellement retrouvées

Dossier : docs/publications/20261005-cursor-moteur/dossier-recherche/revisions/evidence/20261011-run-supervision/.
Note explicative : fichier frère 20261011-run-supervision.md, notamment section Vérification et portée ; index verification-index.json, observed_at_utc 2026-10-11T09:17:57.142567+00:00, base_commit 1169df70bec73d50fd23f640245b8ffd1317b9de, dirty=true.

L'index déclare les commandes ciblées/race et leurs résultats historiques. targeted.txt, race.txt, verified-race.txt et baseline-red.txt existent et leurs SHA-256 concordent avec les entrées correspondantes de l'index. Les trois sorties positives lues sont agrégées : respectivement paquet moteur ok en 14.080 s, 19.848 s, 78.101 s. Elles n'énumèrent pas les tests ; elles ne suffisent pas à établir les cas nommés effectivement exécutés ni le scénario N2. baseline-red.txt conserve FAIL du paquet (2.191 s) ; aucune réécriture.

Incident de méthode corrigé dans ce tour : concaténer systématiquement le dossier de preuve au champ path de l'index produisait des chemins doublés pour ses entrées déjà relatives au dépôt. Résolution discriminante depuis racine OU dossier, avec comparaison SHA-256, pour les quatre noms ci-dessus. Les faux « exists False » initiaux ne sont pas des absences de preuve. Les autres 105 entrées ne sont pas intégralement requalifiées. Cet apprentissage est conservé ici ; pas de modification du lecteur d'index partagé autorisée.

La recette Rust est un support permanent réutilisable. Aucun résultat d'exécution de cette recette n'est ici retrouvé et vérifié ; ne pas convertir ce constat borné en absence globale d'archives.

## Écart établi et décision proposée

Établi : les pièces lues prouvent l'existence d'un arrêt structuré Go attribué à tâche/scope/tentative et d'un support Rust pour révocation globale. Elles ne fournissent pas une preuve complète de conservation d'un ordre original sur deux relais avec A et B valides.

Non établi : défaut actuel de fidélité Swarm, absence générale d'enveloppe d'instruction, absence générale de tests, parité Go/Rust, canal public adapté à deux relais, arrêt de nouvelles actions métier après application. applyRunDecision vérifie la cible qu'il reçoit ; ces seules fonctions ne démontrent pas la liaison à un ordre humain antérieur ni la capacité de détecter qu'un relais a remplacé A par B, en particulier dans un même scope.

Décision proposée au coordinateur : compléter d'abord la qualification, en réutilisant les fixtures et contrats, sans changement moteur présumé. Ne pas créer un nouveau protocole à partir d'une couverture inconnue. Si le parcours public ne permet pas de représenter les deux relais et leur original, conserver ce critère ouvert et cadrer explicitement ce contrat avant toute correction.

Deux options comparées : (1) prolonger le banc Go d'arrêt ciblé, une fois libéré et autorisé, permet d'examiner le contrat existant sans refonte ; (2) adapter immédiatement la révocation Rust globale impose un autre périmètre et pourrait arrêter B : écarté à ce stade. Le choix effectif de moteur/profile revient au coordinateur après inspection actuelle du parcours public retenu.

## Scénario futur proposé — non exécuté

1. Préconditions : autorisation explicite de qualification, propriétaire et réservations confirmés, checkout/entrées stables, fixture isolée sans mission réelle, binaire/profil exacts identifiés, aucun fournisseur réel. Identifier les opérations publiques existantes pour remise, réception, décision, observation, reprise ; un ACK de transport n'est pas une compréhension. Pas d'ajout arbitraire de délai global : observer progression, watchdog global désactivé sans borne justifiée ; conserver les délais contractuels.
2. Témoin : A et B disposent chacun de droits applicables, effectuent chacun une action fictive vérifiable. Deux relais R1/R2 transmettent un ordre témoin inchangé ; capturer original, identités/droits effectivement établis, destinataire, tâche/scope/tentative, contenu/version/empreinte à chaque frontière, réception et état consommateur. Si ce témoin échoue, diagnostiquer avant injection.
3. Arrêt ciblé : émettre stop A par autorité valide, R1 → R2 → décideur/consommateur. Conserver séparément remise à R1, réception R1/R2, réception du décideur, décision autorisée, application et fin réelle. Les relais transportent l'ordre ; aucune autorité supplémentaire n'est inférée. L'arrêt de B ne constitue pas un succès.
4. Variante, après témoin frais : R1 ou R2 remplace A par global/B, ou transforme stop A en interdiction de discussion. Attendu cible : détecter/refuser la portée altérée avant effet non autorisé ; conserver original et diagnostic. Si le protocole courant n'a pas de liaison vérifiable à l'original, cette attente est une exigence proposée, pas une garantie existante.
5. Vérifier B : nouvelle action valide et effet observable après arrêt A ; ses droits/compteurs/historique restent attribuables. Aucune nouvelle action A après application effective ; ne pas prendre la demande ou la seule réception comme instant d'effet.
6. Variante course : action A déjà engagée avant l'application, réponse retenue ou résultat inconnu. Conserver émission, observation, effet connu/inconnu et consommation ; pas rollback, réussite vide ou absence d'effet inférés d'un refus. Lecture/réconciliation et compensation éventuelle exigent leurs droits propres. Réutiliser POC-04/Mikhail pour les effets, ne pas recommencer leur campagne.
7. Refus pertinents : mauvais scope/propriétaire, observation non reçue, tentative remplacée, événement périmé, doublon, relais sans autorité décisionnelle ; témoin valide avant chaque injection. Vérifier état avant/après et absence d'effet non autorisé, jamais simple exit/HTTP 200.
8. Reprise : décision A distinctement autorisée, identité/tentative/fraîcheur actuelles, parcours public existant, nouvelle action A et B témoins réussies, historique et dépenses conservés. Aucun timeout ou relais ne vaut reprise implicite.

## Propriétaires, réservations et clôture

- Coordinateur nocturne : arbitrage d'autorité, choix Go/Rust, disponibilité des propriétaires, attribution et registre ; aucun message sortant de N2.
- N2 / réserve A4 : ce cadrage uniquement. Fabrice reste décideur de l'évolution. Gi.eth garde sa qualification Go, Peter les mandats/faux accords, Mikhail les effets/réconciliation ; ne pas reprendre ces lots.
- Fichiers candidats à réserver AVANT une future écriture : internal/engine/run_supervision_test.go et, seulement si défaut reproduit, run_supervision.go/planning.go et consommateurs affectés ; recette permanente existante à sélectionner après inventaire du parcours Go, pas une recette inventée ici. Toute recette Rust réutilisée garde son contrat actuel jusqu'à décision explicite.
- Contrats candidats : original reçu → relais → décision planning ; événements de scope et tentatives ; arrêt effectif vs effets ; reprise autorisée. Docs concernées en cas de modification : RUN-SUPERVISION FR/EN, protocole applicable, recette et version_history.json selon livraison. Ces réservations proposées ne sont pas des verrous.
- Précondition de mise en œuvre : remise/libération des travaux concurrents et nouvelle attribution bornée ; refus push main pour protection PR/checks/install conservé selon NUIT.md et le message reçu. Aucun développement N2 attribué tant que ce contexte de livraison reste bloquant ; aucune protection ou branche contournée.
- Critères de clôture technique futurs : témoin A/B effectif, deux relais traçables sans changement de portée, refus d'altération sans effet indu, stop A confirmé et B fonctionnel, conservation des effets engagés/inconnus, reprise A autorisée, preuves liées aux entrées/profile/binaire, revue applicable et livraison vérifiée si demandée. Tous NOT TESTED pour ce scénario.
- Clôture présente : remise du cadrage seulement. Ni conformité globale, ni fonctionnement autonome IA, ni installation, ni acceptation produit. Prochaine action du coordinateur : relire et décider d'une qualification bornée après préconditions ; pas de relance automatique depuis cette réserve.

## Revue du seul incrément documentaire

Politiques : aucune politique d'exécution ajoutée ; « deux relais » est une entrée de scénario demandée, pas un plafond produit. Fonctions : aucune fonction publique/non triviale créée/modifiée ; pas de docstring ni compilation à actualiser. Erreurs : chemins de refus proposés, incertitudes et incident de résolution d'index conservés, pas de correction moteur alléguée. Ce fichier est la documentation du cadrage ; aucune page opérationnelle FR/EN ni note de version livrée ne change, car aucun comportement produit n'a été modifié.

## Empreintes et fraîcheur des lectures

Inventaire borné, SHA-256 des fichiers sous /home/fpizzi/workspace/swarm. Comparaison de fin : aucune dérive détectée parmi ces 22 fichiers entre captures ; ce contrôle n'est pas une exclusion de mutation concurrente ni un inventaire complet du build. Le premier lot couvre avant lecture ciblée Go/Rust ; certains documents avaient déjà été consultés avant capture et ont ensuite été relus par passages. Les archives de logs ont été rapprochées de l'index séparément, sans prétendre une capture initiale avant leur toute première lecture.

| Fichier | SHA-256 avant | SHA-256 après |
| --- | --- | --- |
| AGENTS.md | 9f1e485774acb94d212354c1aca4ccffbf5ec797cc0b64ba4ec27509cc745810 | 9f1e485774acb94d212354c1aca4ccffbf5ec797cc0b64ba4ec27509cc745810 |
| tools/agent-workflows/CONTRACT.md | 877b89a230d830024e068d131d2d2eb2eb27f19137f3dc1bc27b98ba9bc70557 | 877b89a230d830024e068d131d2d2eb2eb27f19137f3dc1bc27b98ba9bc70557 |
| docs/architecture/RUN-SUPERVISION.md | 7e683f910245320bd7c21061a69e0b45421ba8222ae3cbf94c7ad9b2f99f5a1e | 7e683f910245320bd7c21061a69e0b45421ba8222ae3cbf94c7ad9b2f99f5a1e |
| docs/architecture/AUTORECURSIVE-RECOVERY.md | df49a8f0dd2bd4d78d0e9049ab01364601b1bfb67121296c2e3415d309102400 | df49a8f0dd2bd4d78d0e9049ab01364601b1bfb67121296c2e3415d309102400 |
| docs/communication/x-swarm-20261107/IDEES-DISCUSSIONS.md | 6bbbfe9ae1d96f76c04cf2ecd83addde550f415af7c85d32d3777045656b2c2a | 6bbbfe9ae1d96f76c04cf2ecd83addde550f415af7c85d32d3777045656b2c2a |
| rust/crates/swarm-engine/src/planning_control.rs | 6f13a0d5cb789925da35fd847f756de44382fa29018e3cbce53707ece17670a0 | 6f13a0d5cb789925da35fd847f756de44382fa29018e3cbce53707ece17670a0 |
| rust/crates/swarm-engine/src/agent_scope_signals.rs | a302789edd9cd2492e942d0266e7efc68c59d0de709c379abb00d0b2f7fb9a11 | a302789edd9cd2492e942d0266e7efc68c59d0de709c379abb00d0b2f7fb9a11 |
| rust/crates/swarm-engine/src/agent_supervision.rs | 962023bb0570e077c1053ef09a8910d0f0839c6456969fb29e8868b0512585cc | 962023bb0570e077c1053ef09a8910d0f0839c6456969fb29e8868b0512585cc |
| rust/crates/swarm-engine/src/mission_control_write.rs | 213bc48ed997dbbf17be957848f484dd855da4d2f4fb3c8157decc9d59464d08 | 213bc48ed997dbbf17be957848f484dd855da4d2f4fb3c8157decc9d59464d08 |
| tests/rust_mission_controls_revocation.py | dc90c4f2a8eab81ba5696121543aa81995170613ee7a25b8420763a906029683 | dc90c4f2a8eab81ba5696121543aa81995170613ee7a25b8420763a906029683 |
| tools/verification/recipes/mission-controls-revocation.json | e4e9ad5dbe16d703dd482bd63e88bcc53139b07f1566f16dc74aeb7befa95c04 | e4e9ad5dbe16d703dd482bd63e88bcc53139b07f1566f16dc74aeb7befa95c04 |
| docs/plans/AGENT-CONTROL-POCS.md | 4b2484434c03b4f45ed425e8b7440159a151c6038b90ca381d0731b09b06286e | 4b2484434c03b4f45ed425e8b7440159a151c6038b90ca381d0731b09b06286e |
| internal/engine/run_supervision.go | c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f | c4b53f49951102f8fe9778ac3b8b8a8f1868ef63ba38b601bd8e07343b84c01f |
| internal/engine/run_supervision_test.go | 93d492c29d71953acbf611a2c3582789170dfe3793e93ab51587fc5220832450 | 93d492c29d71953acbf611a2c3582789170dfe3793e93ab51587fc5220832450 |
| internal/engine/planning.go | 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb | 66ee75df2ce0490e2d362b23030ed033e4e8cf1db402c5448b4464e1af5f62eb |
| internal/engine/planning_schema.go | e743a39d1ac6ccb016528f09cb59ed8d4084980d11d54d2d3d0161a9f2541080 | e743a39d1ac6ccb016528f09cb59ed8d4084980d11d54d2d3d0161a9f2541080 |
| internal/engine/planning_prompt.go | c32dbbff1ec7279eabe91437257212b4c2238e0cbdb3fc27c6e1647e94ca1725 | c32dbbff1ec7279eabe91437257212b4c2238e0cbdb3fc27c6e1647e94ca1725 |
| docs/publications/20261005-cursor-moteur/dossier-recherche/revisions/20261011-run-supervision.md | fc5c8177be5c772bf1e69f8bae4aa19a4945c4e45e24cd91c56c92e8fb610900 | fc5c8177be5c772bf1e69f8bae4aa19a4945c4e45e24cd91c56c92e8fb610900 |
| internal/engine/agents_process.go | 00d74a35555c8c4fb2f2ac56d5482bc3b8a8ca0bbb97755a14f5cb02dfec256f | 00d74a35555c8c4fb2f2ac56d5482bc3b8a8ca0bbb97755a14f5cb02dfec256f |
| internal/engine/agent_terminal_process.go | c7fc0874c0ce9b2da5b2f6e43663dd14fe883f8193926050754995646a19144d | c7fc0874c0ce9b2da5b2f6e43663dd14fe883f8193926050754995646a19144d |
| docs/communication/x-swarm-20261107/lot16-x-sources.json | 2d5a13dbfbbe3ae1ac3bfb179abfd68bf59cbe7f1a3a8235a0020d976dd6a4af | 2d5a13dbfbbe3ae1ac3bfb179abfd68bf59cbe7f1a3a8235a0020d976dd6a4af |
| docs/publications/20261005-cursor-moteur/dossier-recherche/revisions/evidence/20261011-run-supervision/verification-index.json | 168a0db7beaf3cc81d6409616a715904d4735754c815e27c88fe6b173f96bca2 | 168a0db7beaf3cc81d6409616a715904d4735754c815e27c88fe6b173f96bca2 |
