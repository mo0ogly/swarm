# 7. Le moniteur mis à l'épreuve

Un moniteur de référence n'est digne de confiance que s'il est lui-même vérifié. La même méthode
d'injection de fautes qui évalue les agents a donc été retournée contre le moteur. Ce chapitre
décrit ce qu'elle a révélé, les correctifs apportés, et la campagne qui vérifie ces correctifs.

## 7.1 Six constats

La construction du banc et les campagnes ont mis au jour six comportements du moteur que la
conception n'anticipait pas : cinq défauts et un choix de sûreté. Le relevé complet, avec les
références dans le code au commit `53f2564`, est dans `docs/benchmarks/billing/defauts-moteur.md`.

**D1 — Validation jamais reprise en espace de travail propre.** Lorsqu'une tâche s'exécute dans
son propre répertoire, la remise lit le livrable dans cet espace, mais la reprise de la validation
le cherchait à la racine du projet : la tâche n'était jamais acceptée. Le harnais de tests du
moteur masquait ce défaut en écrivant le livrable aux deux endroits.

**D2 — Refus silencieux.** Une tâche dont la dépendance n'était plus fraîche n'était pas lancée,
ce qui est correct, mais sans motif journalisé ni événement adressé au responsable, et la
dépendance restait affichée comme acceptée. L'arrêt était sûr ; le diagnostic, absent.

**D3 — Indisponibilité traitée comme un échec métier.** Un contrôle qui échouait parce qu'un
service était indisponible déclenchait la correction automatique de la tâche, donc la régénération
du livrable, au lieu d'une attente de rétablissement. Sur un processus de paiement, cela produit un
nouveau candidat à valider au lieu de revalider le même : c'est un écart avec I6.

**D4 — Pas de reprise automatique après un arrêt brutal.** Un agent tué sans diagnostic bloque sa
tâche ; la reprise revient au responsable. C'est un choix de sûreté défendable, conservé et
documenté, non un défaut.

**D5 — Verrou SQLite non réessayé.** Un verrou transitoire pendant l'application d'une décision du
responsable, ou pendant la revue indépendante, devenait un échec durable de la planification.
C’est la cause explicite de 27 erreurs sur 3 000 essais S. Les cinq délais supplémentaires
présentent un claim sans décision suivante ; leur cause initiale reste inconnue (section 6.4).

**D6 — Exigences vérifiées à la clôture, pas avant l'effet.** La clôture d'un périmètre refusait
une exigence sans tâche acceptée, mais rien n'imposait qu'une tâche à effet ne parte qu'après
l'acceptation des tâches qui contrôlent son candidat : c'était au responsable de déclarer la
dépendance. Avec un responsable réel, une erreur de planification aurait pu laisser partir le
règlement sur un lot non contrôlé ; seul l'exécutant l'aurait arrêté (E3). Ce constat a été fait
à la lecture du code, non en exécution, le responsable du banc étant scripté.

## 7.2 Correctifs

Les correctifs sont réunis dans le commit `53752da` et documentés dans
`docs/en/ENGINE-BUSINESS-CONTROLS.md`.

- **D1** : la protection existait déjà sur la branche principale ; elle est désormais testée sans
  copie du livrable à la racine.
- **D2** : le refus porte un motif visible et adresse au responsable un événement
  `dependency_stale`, sans effacer l'historique de la preuve.
- **D3** : un contrôle peut déclarer ses codes d'échec d'environnement. Un tel échec produit un
  reçu marqué comme tel et bloque la validation sans relancer le producteur ; la revalidation du
  même résultat, après réparation, est une opération explicite de l'opérateur.
- **D5** : la reprise bornée et idempotente des transactions couvre désormais le verrou SQLite,
  avec le même événement, la même requête et la même révision attendue.
- **D6** : l'opérateur peut déclarer à la création du travail des prérequis entre exigences, par
  exemple que l'exigence de règlement attend celle du contrôle du lot. Une tâche portant
  l'exigence dépendante ne peut alors partir que si une **autre** tâche acceptée, à preuve fraîche,
  couvre l'exigence préalable, même si le responsable a omis la dépendance. Ces règles ne peuvent
  plus être modifiées ensuite, et une tâche sans exigence déclarée ne peut pas s'exécuter. La
  lecture du code (`requirement_prerequisites.go`) confirme ce comportement.

La documentation des correctifs en fixe elle-même les limites : le moteur vérifie des
classifications déclarées et ne peut pas deviner qu'une commande quelconque effectue un paiement ;
la règle de lancement n'empêche pas une modification concurrente pendant l'exécution ; ces
correctifs ne font pas de Swarm un système financier de production.

## 7.3 Campagne de vérification

La vérification suit la même méthode que l'évaluation principale. Son protocole
(`docs/benchmarks/billing/verification-correctifs.md`) a été inscrit dans l'historique du dépôt
avant la campagne ; il l'a été en revanche après une mise au point exploratoire, pendant laquelle
deux choix ont changé. Le principal : sans espace de travail propre, le contrôle positif de D6 ne
prouvait rien, car l'espace partagé sérialise déjà les tâches ; les variantes D6 utilisent donc des
espaces propres. Le binaire mesuré est construit depuis le commit des correctifs, dans un arbre de
travail sans modification ; les résultats historiques ne sont ni recalculés ni remplacés.

La campagne compte 1 700 exécutions scriptées en condition S : F4e et F5 sous les trois clés,
trois variantes sans faute (règlement sans dépendance déclarée avec la règle de prérequis, même
chose sans la règle comme contrôle positif, espaces de travail propres), et une non-régression avec
la clé métier sur les autres cas.

**Tableau 7 — Vérification des correctifs.**

| Défaut | Critère pré-enregistré | Avant correctif | Après correctif |
|---|---|---|---|
| D5 | aucune exclusion due au verrou SQLite sous F4e | 26 ERREUR SQLite + 5 DÉLAI non attribués sur 300 | 0 sur 300 |
| D3 | sous F5 : un seul lancement de `prepare`, reçu d'échec d'environnement, ni faux succès ni paiement inexact | lot régénéré | tenu sur 300 exécutions valides sur 300 |
| D2 | sous F4e : événement `dependency_stale` et arrêt attribué au moteur | arrêt muet | tenu sur 300 sur 300 |
| D6 | sans dépendance déclarée et avec la règle : aucun départ du règlement avant l'acceptation de `prepare`, règlement exact | non exercé | tenu sur 100 sur 100 |
| D6, contrôle positif | sans la règle : le règlement part trop tôt | — | 100 sur 100 |
| D1 | espaces de travail propres : règlement exact | tâche jamais acceptée | tenu sur 100 sur 100 |

Le contrôle positif de D6 a été examiné exécution par exécution, et pas seulement compté : dans
les 100 cas, le journal du moteur contient le refus du règlement, « prepare non accepté », avec
`prepare` encore en cours (99 fois) ou bloquée (1 fois) au moment du départ du règlement. Sans la
règle, le moteur laissait donc partir le règlement trop tôt dans tous les cas, et seul
l'exécutant l'arrêtait ; avec la règle, jamais. La garantie vient bien du moteur.

La non-régression est conforme : avec la clé métier, sur les huit autres cas (sans faute, F1, F2,
F3, F4, F6, F7, F8), chaque défaut est présent ou absent comme dans la campagne historique. La
campagne a duré 7 h 11 ; sur 1 700 exécutions, 1 600 sont valides et les 100 autres sont les
ERREUR attendues du contrôle positif. Le code du banc et le binaire mesuré sont restés inchangés du
début à la fin (fichier `verification-20261007.jsonl`).

La suite de tests du moteur a été rejouée après la campagne sur le commit des correctifs. Un seul
test échoue sur la branche de vérification : il compare un manifeste d'empreintes aux fichiers du
banc, que la vérification a modifiés (`run_s.py`). Rejoué sur le commit des correctifs intact, il
passe ; l'échec vient donc de l'instrumentation de la vérification, non des correctifs.

## 7.4 Ce que la vérification ne montre pas

Ces résultats disent que les critères pré-enregistrés sont tenus sur les cas testés ; ils ne
disent pas que les défauts sont corrigés en général.

- D3 : seule l'absence de régénération est vérifiée ; la revalidation explicite après
  rétablissement n'est pas exercée.
- D6 : la règle est déclarée par l'opérateur et le responsable est scripté. Le cas qu'elle vise,
  un responsable réel qui se trompe de dépendance, reste à mesurer (chapitre 9).
- D5 : vérifié sous la charge de la faute F4e, avec un seul exécutant à la fois ; un accès plus
  concurrent n'est pas testé.
- Les critères et le code d'analyse sont de l'auteur, sans relecture indépendante à ce jour.

La boucle « mesure, défaut, correctif, re-mesure » constitue néanmoins une contribution de méthode :
elle traite le moniteur comme un objet d'évaluation au même titre que les agents qu'il contrôle.

## 7.5 Relecture des entrées archivées

Le 8 octobre 2026, après collecte des données, le lecteur CLI `tables_verif.py` a été durci
pour refuser une grille incomplète, les graines dupliquées ou inattendues, une fin de campagne
absente, les empreintes incohérentes entre les en-têtes et les lignes, et les preuves d’ordre
manquantes. Ce changement de validation des entrées n’est pas un critère pré-enregistré : il
ne modifie ni les critères métier, ni les JSONL historiques. La non-régression compare seulement
les huit cas annoncés et refuse une comparaison sans mesures valides.

Les 1 700 lignes de vérification et les 9 000 lignes historiques satisfont ces contrôles.
Les cinq verdicts « critère tenu » et la non-régression restent inchangés. Cette relecture
est une vérification technique distincte de l’analyse initiale ; elle ne vaut pas revue par
les pairs ni inspection de chaque trace d’exécution. Les empreintes sont comparées dans
l’archive ; ce contrôle ne reconstruit pas le binaire historique à partir de son commit.

## 7.6 Vivacité des superviseurs — observation du 10 octobre 2026

Une interruption de la mission d'évolution Cursor révèle une limite distincte
des défauts D1 à D6. Le vérificateur puis le responsable racine ont été arrêtés
par un plafond total de 90 secondes malgré une activité de raisonnement observée.
Les événements avaient été adressés au responsable : le défaut concernait sa
capacité à terminer le diagnostic, pas l'absence de routage.

La correction sépare silence, durée totale facultative et bail de propriété
renouvelable. Ces politiques sont persistées et réglables dans Admin, sans valeurs
de politique en constantes Go. Les superviseurs analysent les retours et proposent
les corrections ; le moteur contrôle propriété, budgets et acceptation. Seul le
responsable racine était actif dans ce cas ; aucun sous-planificateur n'avait été
créé. Un événement de raisonnement prolonge l'attente autorisée, sans prouver que
le travail avance utilement.

Après correction sur le serveur réel, la revue a terminé en 166,40 secondes avec
une demande de preuve complémentaire. Le responsable a ensuite enregistré une
correction ciblée et le moteur a lancé la deuxième tentative de T1. La
[note datée](revisions/20261010-vivacite-supervision.md) conserve la chronologie,
les réglages, la vérification, les limites et l'essai comparatif proposé.
Cette observation après incident ne constitue ni une nouvelle campagne
pré-enregistrée ni une preuve d'absence générale de blocages.
