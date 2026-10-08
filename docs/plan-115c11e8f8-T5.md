# T5 — Dossier de clôture à examiner (version condensée)

Mission `w-115c11e8f802a4f98c3def32`, tâche `plan-115c11e8f8-T5`. Correction documentaire par Codex après le worker `auto-2a6914c3316e81db08fe`, tentative `a-1637354a070e743f69f201a7`. Aucun nouvel exécutant, aucun changement du code candidat, aucune livraison proposée à ce stade. Les originaux du worker et le dossier antérieur sont conservés.

## 1. Recette des quatre parcours — critère 1

FR/EN × clair/sombre rendus et inspectés ; Entrée ouvre, Échap ferme, focus revient au déclencheur. Recettes navigateur effectivement réalisées par le superviseur avec CUA ; tests moteur et frontend exécutés séparément, sans assimiler npm test à une recette visuelle. La revue indépendante 42, r306, a déjà rendu PASS sur ce critère avec les mêmes vingt captures. Cette preuve historique n’est pas un PASS global de T5. Les vingt images sont empreintées par les contrôles courants ; les quatre images ledger initiales ont été remplacées par quatre nouvelles captures QW3 avec le bouton de fermeture effectivement au focus clavier. Les anciennes captures sont conservées.

| Parcours | Résultat observé | Preuves détaillées conservées |
| --- | --- | --- |
| QW1 | Résumé objectif/rôle/fournisseur/modèle/profil/skills avant départ ; modèle réel inconnu avant exécution ; divergence demandée/rapportée affichée sans invention | T1-handoff.md ; launch-v4-browser-replay.json ; launch-final-en-light.json ; 4 captures launch |
| QW2 | Organisation absente : Vous/You et Préparer l’organisation ; plafond réel de tentatives : Responsable et Examiner les tentatives ; Entrée/Échap/restauration du focus ; aucune reprise contournant le plafond | T2-supervisor-recipe.md ; 8 captures organization/limit ; contrôles CLI réels |
| QW3 | État du processus distinct de validation ; lectures/écritures/autres/répétitions séparés ; absence d’usage ou coût explicitement inconnue ; tentatives terminée et interrompue réellement examinées | T3-supervisor-recipe.md ; 4 captures ledger ; contrôle CLI de données persistées |
| QW4 | Refus → correction bornée → reprise réelle, sans nouveau producteur ni hausse du plafond des tentatives ; preuves conservées et périmées distinguées | T4-supervisor-recipe.md ; T4-full-fresh-recipe.json ; 4 captures recovery |

Limites : la divergence de modèle QW1 est un fournisseur de protocole isolé, pas une divergence physiquement provoquée chez Claude. Les bannières QW2 ont des captures FR/EN claires ; les détails de modale ont les quatre variantes de langue/thème. Le contrôle courant QW4 vérifie les traces du parcours attribué, il ne prétend pas rejouer le navigateur. T1–T4 sont actuellement acceptées ; cette mission relève d’une supervision explicite, pas d’une autonomie sans intervention.

## 2. RETEX quantifié daté — critère 2

Instantané public complet capturé `2026-10-03T19:39:54.280769+00:00`, révision 315, après la revue 43 terminée sans verdict. Source : `T5-spending-snapshot-before-review44.json`. Les valeurs suivantes sont figées à cet instant, pas des totaux futurs ni un compteur en temps réel.

| Rôle/tâche | Appels enregistrés | Outils observés | Jetons sortie rapportés | Coût partiel rapporté USD | Appels sans usage |
| --- | ---: | ---: | ---: | ---: | ---: |
| Responsable : root | 5 | 0 | 18077 | 0.451902 | 2 |
| Vérificateur indépendant | 43 | 0 | 543343 | 11.9872264 | 3 |
| Inspection ciblée lecture seule (RETEX + points d'entrée) | 2 | 27 | 5404 | 0.28512860000000007 | 1 |
| REQ-QW1 — Résumé avant lancement (web + CLI) | 3 | 51 | 30589 | 1.0946986 | 1 |
| REQ-QW2 — Blocage expliqué (web + CLI) | 2 | 37 | 16528 | 0.6520226 | 1 |
| REQ-QW3 — Bilan par tentative | 2 | 37 | 13333 | 0.4424447999999999 | 1 |
| REQ-QW4 — Reprise ciblée après refus ou blocage | 2 | 37 | 12131 | 0.44120180000000003 | 1 |
| Recette globale et RETEX (4 parcours, web + CLI) | 1 | 12 | 17226 | 0.7231206 | 0 |

Les coûts et jetons additionnent uniquement les usages rapportés ; les appels sans usage restent inconnus. Les appels fournisseur, événements du flux, outils et jetons sont des unités distinctes. Les contrôles et la revue suivante feront évoluer les compteurs vivants ; cela ne contredit pas cet instantané daté. Le contrôle `real-final-ledger` expose les lignes courantes séparément de cet instantané et vérifie la partition lectures+écritures+autres pour chaque tentative mesurée.

Interventions humaines/superviseur vérifiables : acceptations T1–T4 distinctes, recettes navigateur, corrections bornées des preuves T2/T4/T5, prévisualisations et autorisations de plafonds. Les plafonds 42, 43, puis une seule revue 44 et dix minutes ont été autorisés par l’utilisateur ; aucun appel ni tentative n’a été remboursé. Les refus 41/42 et l’expiration 43 sont conservés dans `T5-reviews-before-review44.json`. L’expiration 43 a produit 234 événements, dont 228 signaux thinking_tokens, sans événement final : cela ne mesure ni 234 appels d’outils ni le nombre de jetons consommés. La cause interne du fournisseur reste inconnue.

Le RETEX complet et ses améliorations proposées restent dans RETEX-supervision.md et RETEX-review43-timeout.md : limiter le volume documentaire d’une clôture, réutiliser les preuves fraîches déjà examinées, séparer les unités de budget, relier le blocage à sa configuration. Ces améliorations non réalisées ne sont pas déclarées livrées. Configuration actuelle : web Budgets et coûts IA → Plafonds de planification et de vérification ; CLI `swarm quotas show|preview|apply`. Plafond d’outils distinct dans Administration et `swarm run-limits`.

## 3. Candidat exact et ordre de clôture — critère 3

Base Git `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, arbre modifié. Candidat exact `a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77` : 48 fichiers SHA-256 de `T5-candidate.json`. Le contrôle `exact-dirty-candidate` revérifie ces empreintes, l’identifiant canonique et la base courante.

Suite complète du même candidat : `go test ./... -timeout 40m` PASS, 473,034 secondes, code 0 ; journal T5-go-full.log et T5-global-checks.md. go vet, race ciblée 12,799 s, npm test et configuration neuf méthodes : PASS attribués au superviseur. Les quatre contrôles moteur de T5 sont réexécutés sur les preuves actuelles avant la revue ; leurs résultats sont donnés par engine_controls, pas inventés dans ce rapport.

Les revues indépendantes 41 (r301) et 42 (r306) ont été réalisées sur ce même candidat exact avant toute proposition de livraison. Elles ont demandé des corrections du dossier ; la revue 42 a confirmé le lien au candidat et le critère visuel mais refusé la quantification non datée et une formulation de livraison prématurée. La revue 43 a expiré sans verdict : aucune de ces revues ne vaut autorisation globale de T5.

Ce dossier corrigé est soumis à une nouvelle revue indépendante, pas à une livraison. Aucune proposition de livraison, fusion ou publication n’est faite ici. Un avis favorable courant sur ce dossier, les contrôles frais puis une acceptation moteur distincte doivent précéder toute proposition de livraison. Le contrat hiérarchique reste inchangé. Aucun PASS futur ni aucune acceptation ne sont affirmés dans le dossier examiné.

## Complément clavier après la revue 44

La revue 44 a rendu PASS sur les critères 2 et 3, et unknown sur le critère 1 : les séquences clavier n’étaient fournies qu’en références empreintées. Ce défaut concerne les pièces transmises, sans nouveau défaut de code démontré. La décision complète est conservée dans T5-review44-decision.json.

Entrée ouvre, Échap ferme, focus revient au déclencheur : ce parcours QW3 a été rejoué réellement par le superviseur CUA le 3 octobre à 19:44–19:45 UTC en FR/EN × clair/sombre. Les quatre captures t3-keyboard-{fr,en}-{light,dark}.png montrent le focus visible sur Fermer/Close ; t3-keyboard-live.json enregistre les états DOM après les vraies touches et leur horodatage. Le contrôle dédié fournit au vérificateur le contenu synthétique de ces enregistrements, pas seulement leurs chemins.

Le même parcours clavier de l’explication d’attente QW2 a été rejoué sur la mission courante en quatre variantes à 19:46 UTC ; t2-keyboard-live.json enregistre les états. Il s’agit ici de la modale indiquant l’absence d’une dépendance en attente, pas d’une nouvelle recette des cas d’organisation et de plafond ; les preuves antérieures de ces deux cas restent conservées. QW1 possède quatre séquences datées dans launch-v4-browser-replay.json ; QW4 possède la séquence réelle dans t4-browser-journey.json, et le parcours complet dans T4-full-fresh-recipe.json. Le contrôle expose les états enregistrés et confirme la fermeture avec focus visible. Il ne prétend pas conduire personnellement le navigateur, et ces données restent attribuées au superviseur.
