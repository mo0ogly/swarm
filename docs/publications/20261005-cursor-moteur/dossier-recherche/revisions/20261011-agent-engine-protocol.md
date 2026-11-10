# Mise à jour — protocole strict IA / moteur déterministe

Observation du 11 octobre 2026, mission `w-ca937edf4e0abd85f8d3d67b`.
Cette révision complète la [traçabilité du développement](20261010-tracabilite-developpement.md),
sans remplacer les observations du 10 octobre ni les campagnes historiques.
Le [contrat d’architecture FR](../../../../architecture/AGENT-ENGINE-PROTOCOL.md)
et son [miroir EN](../../../../en/AGENT-ENGINE-PROTOCOL.md) décrivent le protocole
et son diagramme Mermaid. Les consignes communes Codex/Claude/Swarm y renvoient.

## Première frontière en défaut et diagnostic récursif

L’ancien retour `review-d69462672b530e6dad368bce` était une erreur de protocole,
« defect quote absent at original line », pas un refus sémantique accepté.
La citation existait exactement à la ligne physique 17 du manifeste de livraison,
mais après un préfixe JSON. Le parseur exigeait implicitement la colonne zéro.
La régression exécutée sur l’implémentation HEAD avec une superposition Go reproduit
le rejet des citations exactes internes et multilignes. Les originaux sont conservés
[dans les sorties avant correction](evidence/20261011-agent-engine-protocol/swarm-quote-before.jsonl).

Le superviseur avait autorisé une relance du producteur pour cette erreur de revue.
Le routage de contexte comparait aussi l’identité de revue avec celle de l’événement
terminal, alors que le moteur ajoute un suffixe de résultat aux revues de fragments.
Un premier test local employait la même hypothèse erronée ; son échec reste conservé.
Les tests corrigés utilisent les véritables événements produits par le moteur.
Ces deux défauts de méthode ont été corrigés avec le défaut du parseur.

## Correction globale et responsabilités

La citation doit être exacte, commencer à n’importe quel caractère de la ligne
annoncée et conserver octets, espaces, signes de patch et sauts de ligne.
Elle peut continuer sur les lignes suivantes. Une citation uniquement présente
ailleurs, inventée ou normalisée reste refusée. Les exigences d’identité,
de couverture et de capacités ne sont pas abaissées.

Le moteur persiste le code `review_fragment_protocol` et l’identité du retour
terminal. `review_failure_routing` attribue la cause au protocole du vérificateur,
pour la revue actuelle, son candidat, sa tentative, son événement et son propriétaire.
Racine et sous-superviseur ne peuvent proposer que `review-retry` dans leurs droits,
après un diagnostic nouveau. Le moteur vérifie fraîcheur, propriété, preuves,
fournisseur et budget. Cette opération ne lance ni producteur ni appel de revue ;
le conducteur réserve ensuite normalement les appels. Les départs automatiques,
la réservation manuelle et l’opération de relance du producteur possèdent le même
garde-fou. Un refus sémantique valide conserve son parcours de correction du candidat.

## Blocage lié : changement de méthode

Le précontrôle public a révélé un changement de méthode depuis la revue historique.
Réutiliser les avis précédents aurait violé l’attribution. Le parcours explicite
`method_restart_ready` reconstruit une revue complète sur le même candidat,
vérifie plan, journal, contexte, reçu et fournisseur historiques, puis archive
l’ancienne revue sans modifier ses fichiers. Le parcours opérateur exige
`confirm_review_error_repair: true`. Le superviseur propriétaire dispose de son
opération `review-retry`, soumise aux mêmes vérifications.

Une réservation active, un refus sémantique, un verdict final engagé, des originaux
altérés ou un budget insuffisant ne peuvent pas être effacés. Les appels déjà
consommés restent consommés. Le conducteur revérifie le checkpoint et conserve les
reçus des contrôles du même candidat ; ce changement de méthode ne prouve pas une
nouvelle exécution de ces contrôles. Aucun ancien avis n’est réutilisé.

## Blocage lié : qualification des sources

La suite complète échouait sur une entrée périmée du manifeste historique
(`PREPARATION-UX.md`). Réécrire ce manifeste aurait confondu état des sources et
acceptation historique. Un générateur séparé crée `engine-cursor-source-manifest.json`
avec les empreintes des sources présentes et le statut
`ENGINE_CURSOR_SOURCE_SNAPSHOT_NOT_INDEPENDENT_ACCEPTANCE`.
Les lignes historiques R01 à R18 restent en attente de requalification.
Le validateur strict de sources reste identique. T5 doit générer cet instantané
sur son propre candidat après fusion ; copier celui de la racine serait incorrect.

## Vérification et limites

[Les sorties originales et leur index](evidence/20261011-agent-engine-protocol/verification-index.json)
conservent empreintes, horodatages Go, résultats, état modifié des sources et limites :

- Régressions avant/après pour citation, événements réellement produits, périmètres,
  tentatives, intégrité, refus sémantiques, budget, méthodes et reprise publique.
- Suite série `go test ./...` : sortie zéro, 1 857 événements PASS, aucun échec.
  Elle précède la dernière précision de métadonnée d’aperçu, couverte par les
  régressions finales et la suite complète répartie.
- Suite complète finale : 1 110 tests, exemples et cibles de fuzz par défaut,
  découverts puis répartis en quatre groupes disjoints, tous à sortie zéro.
  Ce nombre exclut les sous-tests ; les événements PASS de la suite série les incluent.
- Vérifications de courses ciblées, `go vet ./...` et contrôle des contextes réels
  des rôles : sortie zéro. Les doubles fournisseur ne démontrent pas une décision
  autonome d’un superviseur réel.
- Mermaid FR/EN analysés et rendus en SVG. Les SVG originaux sont conservés.

## Effet public réel et suite confiée à Swarm

Le serveur actif expose le binaire construit à `2026-10-11T07:30:55Z`, commit
`1169df70bec73d50fd23f640245b8ffd1317b9de`, sources modifiées. Cette identité a été
vérifiée par l’API de santé du serveur, pas seulement par la version du CLI.

La reprise publique `planning retry-review`, événement
`review-protocol-repair-20261011`, a fait passer la révision 90 à 91 avec confirmation
explicite. Le candidat reste `56ca8cab5bfde7dc9ddf7a7a5d2169231d87c9a9` ; T2 garde
sa seule tentative de production et ses dix appels antérieurs. L’aperçu prévoyait
cinq appels frais avec vingt disponibles. L’ancienne revue est conservée dans
l’historique ; les empreintes des cinq fichiers de preuve originaux sont inchangées.
La nouvelle revue `review-83368e7fc18d39d4c7968034` est en cours lors de l’observation,
avec onze appels enregistrés au total. Une réservation ne démontre pas un envoi.
La mission est activée, sans pause et sans changement de budget.
[Observation publique et comparaison des originaux](evidence/20261011-agent-engine-protocol/public-observation.json).

Le correctif consolidé `agent-engine-protocol-20261011.patch`, SHA256
`a9eb1ca8961e446e4d777c3202759c51e02c1c555e8929303853bb1f51f4cb84`, a été transmis
par `task update` à T5, révision 94, avec les anciens textes conservés.
Il inclut les corrections locales de preuves, phases et précontrôle ; le correctif
`provider-wait-20261010.patch` reste à intégrer séparément et les travaux natifs
T1 à T4 doivent être préservés.

Un écart UX a été identifié : la modale actuelle ne transmet pas la confirmation
de changement de méthode. La reprise effective ci-dessus utilise le CLI public.
T5 a reçu explicitement ce travail à la révision 96 : lire l’aperçu moteur,
afficher candidat et appels, demander confirmation uniquement pour cette reprise,
transmettre le champ confirmé, vérifier FR/EN, refus et reprises ordinaires.
L’instruction tient dans les 600 caractères effectivement projetés au superviseur
(553 caractères en tête). Elle n’impose aucun booléen ni budget codé en dur.
Ce travail UX reste à réaliser et vérifier dans le candidat de T5.

La correction locale est installée et la reprise a eu un effet réel. T2 n’est pas
encore accepté ; l’avis indépendant frais, l’intégration native et les validations
T5 à T7 restent distincts. Aucun résultat historique n’est recalculé et aucune
acceptation anticipée n’est annoncée. Après un nouvel échec, la boucle replan
reprend sur son retour original et la première frontière réellement en défaut.

## Réexamen à 10 h 09 Paris — refus sémantique et reprise du producteur

La nouvelle revue a terminé à **09 h 49 min 42 s Paris** avec
`changes_requested`, sans erreur de protocole. Elle relève deux familles de
défauts : chiffres de suite Go présentés sans réserve dans les miroirs FR/EN
(1017/5 contre une observation interrompue à 979/8/1), et preuves d’exécution
attribuées à un autre commit sans chaîne suffisante fournie au juge.
[Le rapport original](evidence/20261011-repaired-handoff/native-semantic-review.md)
est conservé. Le superviseur a traité l’événement courant et proposé une
correction du producteur à la révision 102. Cette décision est désormais
appropriée : il s’agit d’un refus sémantique valide.

Le départ autorisé a révélé une frontière liée en défaut : la transmission
vérifiait seulement `refs/swarm/attempts/AGENT`, alors que la réparation publique
avait conservé cette référence historique et créé `refs/swarm/repairs/PROOF_KEY`.
La référence erronée empêchait le dossier de reprise. La régression sur le code
précédent reproduit exactement « résultat Git absent ou attribution modifiée ».
La correction vérifie la bonne référence, agent, tentative, état terminal,
arbre et ascendance ; elle joint explicitement la provenance de réparation
externe et refuse les altérations sans repli sur la production initiale.

Le prochain contrôle a ensuite refusé l’empreinte périmée de la sélection APEX.
La copie avait été préparée sans consommer de tentative. Le parcours public
`agent resume-launch` permet maintenant son actualisation explicite :
confirmation, motif et mêmes chemins avec leurs empreintes actuelles exactes.
Le manifeste original reste inchangé ; le lancement enregistre un reçu moteur
`prepared_skill_refresh` avec la préparation, la demande originale et les anciennes
sélections. `agent start` ne peut pas fabriquer cette exception. Fournisseur, rôle,
candidat, plafonds et autres paramètres restent gelés. La préparation périmée
n’est plus annoncée comme prête.

Les erreurs de vérification personnelle ont aussi été traitées et conservées :
le premier nouveau fixture activait le refus après une intégration déjà favorable ;
un fichier historique `.go` sauvegardé comme preuve créait involontairement un
package Go, corrigé par un suffixe `.go.txt` sans changer ses octets ; puis
l’instantané des sources était périmé après correction du fixture, et a été
régénéré. Aucune de ces erreurs n’a été transformée en résultat positif.

**Vérification finale :** suite complète de 1 114 tests/exemples/cibles de fuzz
par défaut, quatre groupes disjoints tous à sortie zéro ; régressions de reprise
66 PASS, sélection de skills 13 PASS, vérification de courses élargie 51 PASS et
`go vet ./...` à sortie zéro. Les comptes PASS incluent les sous-tests.
[Originaux, limites et index des contrôles](evidence/20261011-repaired-handoff/verification-index.json).

**Effet réel :** binaire serveur construit à `2026-10-11T08:04:05Z`, identité
vérifiée par l’API active. L’opération publique explicite a repris
`auto-f2abbcfe6bed66fd1a5e`, révision **105 → 106**. T2 est **running**,
avec **deux tentatives sur les deux autorisées** et un agent actif confirmé.
Il reçoit le résultat réparé et APEX à l’empreinte actuelle. Le manifeste de
préparation et les cinq fichiers de preuve historiques sont inchangés ; les
onze appels de revue restent consommés et aucun plafond n’a été relevé.
[Observation publique et intégrité des originaux](evidence/20261011-repaired-handoff/public-resumption-observation.json).

Le correctif complémentaire `agent-engine-protocol-resumption-20261011.patch`
(SHA256 `a2c6d3dff49ce66001db40acd2e80c15309cbba46a53e4e5ec9e98b49bf3a3ef`)
a été transmis à T5 par `task update`, événement
`protocol-resumption-T5-20261011`, révision **107**. L’instruction de tête
fait 564 caractères et conserve les précédentes. Elle comprend la confirmation
UX des deux parcours, les refus et l’intégration des correctifs avec T1 à T4
préservés. Le manifeste des sources doit être généré dans le candidat de T5.
La confirmation UX reste à réaliser ; **T2 n’est toujours pas accepté**.
La correction des preuves se poursuit désormais dans son nouveau worker.

### Réexamen à 10 h 21 Paris — continuité après les statuts

L’utilisateur a signalé que les comptes rendus intermédiaires terminaient trop
souvent la conduite du travail, puis demandé de changer les directives. Les
accords personnels Codex, déjà importés par Claude, la skill personnelle replan,
les directives du projet et le contrat commun prescrivent désormais de répondre
au statut puis de poursuivre l’action utile autorisée. Un jalon ou un document
terminé ne clôt pas le besoin initial. La clôture nécessite un besoin vérifié,
un arrêt/report explicite ou l’absence d’action utile autorisée, avec cause,
preuve, propriétaire et condition concrète de reprise. Aucun suivi futur non
organisé, nouvel appel payant ou surveillance récurrente n’est autorisé par ce texte.

Le premier ajout a dépassé la capacité existante du contexte planner/subplanner ;
une première réduction était encore insuffisante. Les consignes replan et le
contrat ont été compactés sans relever la borne ni tronquer les contextes.
Les quatre rôles passent ensuite le contrôle réel du moteur et le test de remise
au worker après réouverture passe. La validation des deux skills et le contrôle
des liens Codex/Claude passent. Ces preuves concernent la transmission de règles,
pas leur application autonome par le fournisseur.

[Le complément de directives](../../../../plans/engine-cursor-contract/recovery/agent-continuity-20261011.json)
est transmis à T5 par l’événement public `continuity-directives-T5-20261011`,
révision **107 → 108**, avec résumé de 556 caractères et historique conservé.
L’application aux deux patches précédents reproduit les cinq fichiers exactement ;
le contrôle d’application inverse passe. L’instantané des sources est régénéré,
sans renouveler l’acceptation historique. Le contexte figé de T2 et le serveur
actif ne sont pas remplacés pendant l’exécution. T2 reste en cours, mission
activée, non suspendue et avec un agent actif ; son acceptation reste à vérifier.

### Réexamen à 10 h 32 Paris — hiérarchie Cursor non exercée

Les publications [JAN](https://cursor.com/blog/scaling-agents) et
[FEB](https://cursor.com/blog/self-driving-codebases) sont relues à la demande
de l’utilisateur. L’exigence de hiérarchie reste retenue ; elle n’est pas encore
exercée par cette mission. L’état public contient uniquement le périmètre `root`,
neuf activations de la racine et aucun sous-planificateur. Toutes les exigences
`req-1` à `req-29` sont déjà affectées aux sept tâches de cette racine.

La lecture de `applyPlanningOperation` confirme que `delegate` refuse une exigence
déjà confiée à une tâche. Activer des enfants maintenant par cette opération
nécessiterait une réorganisation prise en charge, pas une nouvelle consigne seule.
Le graphe existant est également une chaîne T1 → T2 → T3 → T4 → T5 → T6 → T7.
Ce constat distingue absence d’usage dans la mission et absence de capacité dans
le moteur : la délégation existe, sa réorganisation et sa concurrence restent à
auditer dans ce cas concret.

Le cas est transmis à T3 par `task update`, événement
`cursor-hierarchy-T3-20261011`, révision **108 → 109**. Le résumé de 593 caractères
demande la correction publique justifiée, sans nouvelles identités, perte de
preuves ni budgets supplémentaires ; les anciennes instructions sont conservées.
La double activation concurrente reste un critère de T3 et la démonstration avec
fournisseur réel relève de T7. Aucun enfant artificiel ni acceptation n’est créé
pour présenter la mission comme déjà conforme. T2 poursuit ses contrôles.
