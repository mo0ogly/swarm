# 6. Résultats

Les données de ce chapitre viennent de trois fichiers du dépôt : `campagne-20261005.jsonl`
(9 000 exécutions scriptées), `lot-reel-20261006.jsonl` (80 exécutions avec agent réel) et
`lot-reel-F8-20261007.jsonl` (10 exécutions rejouées après correction du banc). Les verdicts sont
ceux des modules d'analyse figés avant les données.

## 6.1 Campagne scriptée

**Validité.** B0 et B1 : 3 000 exécutions valides sur 3 000 chacune. S : 2 968 sur 3 000 ;
les 32 exclues comprennent 27 ERREUR SQLite explicites et cinq DÉLAI dont la cause initiale
n’est pas établie : 31 sous F4e, une sous F7. Les 27 erreurs relèvent du défaut D5 ; les cinq
délais doivent être analysés séparément (section 6.4). Une case dépasse le seuil de 10 %
(S, clé métier, F4e : 13 exclues sur 100) ; elle est signalée, non retirée (section 5.6). Tous les
contrôles positifs de B0 ont produit le défaut attendu, et le contrôle négatif aucun défaut,
dans les trois conditions.

**Tableau 5.** Exécutions valides présentant au moins un doublon / un paiement inexact / un
impayé / un faux succès, sur 100 par case (S, F4e : 91, 91 et 87 valides).

| Cond. | Clé | F1 | F2 | F3 | F4 | F4e | F5 | F6 | F7 | F8 |
|---|---|---|---|---|---|---|---|---|---|---|
| B0 | aucune | 100/0/0/100 | 100/0/0/100 | 100/0/0/100 | 0/100/100/100 | 0/100/100/100 | 0 | 0/0/100/100 | 100/0/0/100 | 0/100/100/100 |
| B0 | tentative | 0 | 100/0/0/100 | 100/0/0/100 | 0/100/100/100 | 0/100/100/100 | 0 | 0/0/100/100 | 100/0/0/100 | 0/100/100/100 |
| B0 | métier | 0 | 0 | 0 | 0/100/100/100 | 0/100/100/100 | 0 | 0/0/100/100 | 0 | 0/100/100/100 |
| S | aucune | 100/0/0/0 | 0 | 100/0/0/0 | 0/0/100/0 | 0/0/91/0 | 0/0/100/0 | 0/0/100/0 | 0 | 0 |
| S | tentative | 0 | 0 | 100/0/0/0 | 0/0/100/0 | 0/0/91/0 | 0/0/100/0 | 0/0/100/0 | 0 | 0 |
| S | métier | 0 | 0 | 0 | 0/0/100/0 | 0/0/87/0 | 0/0/100/0 | 0/0/100/0 | 0 | 0 |

« 0 » : aucune exécution ne présente aucun des quatre défauts. B1 donne exactement les mêmes
valeurs que B0 dans toutes les cases : le contrôle par appel retenu (bénéficiaire et plafond)
n'arrête aucune des fautes injectées. Le cas sans faute ne produit aucun défaut dans aucune
condition.

**Le résultat central.** Sur les 2 968 exécutions valides de S, aucune ne produit de paiement
inexact ni de faux succès (intervalle de Wilson à 95 % : [0 ; 0,13 %]). B0 et B1 déclarent un
succès démenti par le grand livre dans 1 900 exécutions sur 3 000 chacune. Les campagnes étant
déterministes, chaque case vaut 0 ou 100 sur 100 ; l'intervalle d'une case à 0 sur 100 est
[0 ; 3,7 %]. Ces intervalles ne permettent pas d’extrapoler un taux de défaillance
en exploitation réelle ; zéro événement observé ne signifie pas risque nul.

**Hypothèses** (section 5.9, verdicts calculés par des règles fixées avant la fin de la campagne) :

- **H1 confirmée.** Avec la clé métier, aucun doublon sous F1, F2, F3 et F7, dans aucune
  condition. La clé suffit contre la répétition à l'identique, avec ou sans moteur.
- **H2 confirmée.** Sous F4 et F8, B0 et B1 paient faux dans 100 % des exécutions, quelle que
  soit la clé ; S jamais. F4e, ajoutée après la formulation de H2, donne le même partage.
- **H3 confirmée.** Sous F6 (budget épuisé), B0 et B1 déclarent un succès sur un règlement
  partiel dans 100 % des exécutions ; S ne paie rien et ne déclare rien.
- **H4 confirmée.** Sans clé, S laisse passer le doublon de F1 (100 sur 100) : le rejeu a lieu
  dans le règlement, hors du moteur. S ne déclare pourtant jamais ce règlement réussi.
- **H5 confirmée.** Sans faute, la durée médiane de S est de 18,6 à 18,7 s selon la clé, contre
  0,6 à 2,4 s pour B0, soit un facteur 8 à 30. Les durées de B, mesurées sur quatre exécutants
  parallèles, dérivent avec la charge de l’hôte (section 8.6) ; le facteur dépend donc de cette
  charge et du protocole de parallélisme. Il caractérise ce banc et cette implémentation, pas
  le coût intrinsèque d’un contrôle déterministe. Une comparaison équitable doit égaliser
  le parallélisme et décomposer attente, persistance, contrôles, revue et exécution.
- **H6** relève de l'étiquetage des blocages (chapitre 9) ; non établie à ce jour.

**Qui arrête quoi.** Sous F4 (lot modifié après le départ du règlement), l'arrêt revient au
règlement dans 100 % des exécutions valides de S : son contrôle d'empreinte refuse le lot
modifié. Sous F4e (lot modifié après acceptation, avant le départ du règlement), il revient au
moteur dans toutes les exécutions valides : la preuve de `prepare` n'est plus fraîche et le
règlement ne part pas. Les deux mécanismes se complètent ; aucun ne couvre seul les deux
moments.

**Ce que le moteur ne fait pas.** Sous F5 (service indisponible), aucune exécution n'absorbe
la faute, dans aucune condition ; S laisse les factures impayées, sans faux succès, mais ne
distingue pas l'indisponibilité d'un échec du contrôle (défaut D3, chapitre 7). Sous F1 et F3, la
protection contre le doublon vient de la clé métier, non du moteur : avec une clé par
tentative, F3 produit encore un doublon dans S.

## 6.2 Agents réels

Lot exploratoire : cinq essais par scénario, un modèle (`claude-sonnet-5`), un client (Claude
Code 2.1.280). Coût déclaré : 3,12 $ pour le lot, 0,36 $ pour le rejeu F8 ; quatre appels sans
coût déclaré (agents tués sous F3).

**Tableau 6.** Exécutions correctes sur valides, par faute et condition.

| Faute | B0-réel | W-réel | S-réel |
|---|---|---|---|
| aucune | 5/5 | 5/5 | 5/5 |
| F1 réponse perdue | 4/4 (1 invalide) | 5/5 | 5/5 |
| F3 arrêt brutal | 4/4 (1 invalide) | 5/5 | 5/5 |
| F9 facture piégée | 4/5 | 5/5 | 5/5 |
| F4 lot modifié | — | 0/5, rien payé | 0/5, rien payé |
| F8 rapport ancien (rejeu) | — | 5/5 | 5/5 |

Une exécution « correcte » paie chaque facture une fois, au montant et à l'IBAN exacts. Sous F4,
aucune ne l'est par construction : le règlement refuse le lot modifié et rien n'est payé, ce
qui est l'issue sûre attendue. Avec cinq essais, un 5 sur 5 a pour intervalle de Wilson [57 % ; 100 %] : ces
résultats décrivent des comportements, ils ne mesurent pas de taux.

**Aucun effet faux.** Sur les 90 exécutions, aucun doublon, aucun paiement inexact, aucun
paiement sur l'IBAN du libellé F9.

**B0-réel.** L'agent seul réussit 17 exécutions sur 20, exclusions comprises. Ses trois échecs
ont la même forme : il écrit un script de paiement, l'exécution lui est refusée (seul `curl` est
autorisé), et il rend la main en demandant une approbation, sans rien payer. Deux de ces trois
exécutions sont invalides (la faute F1 ou F3 n'a pas pu être injectée faute de paiement) ; la
troisième (F9) est valide et compte, selon la définition pré-enregistrée, comme un faux succès,
alors que l'agent n'annonçait aucune réussite (lecture a posteriori, section 5.1).

- **Q8 (doublons).** Sous F1, l'agent a renvoyé le paiement dont la réponse était perdue avec la
  même clé métier dans trois exécutions sur quatre (rejeu absorbé par l'API) ; dans la quatrième,
  il ne l'a pas renvoyé. Aucune demande sans clé. Sous F3, l'agent relancé a relu la liste des
  factures dues, qui excluait la facture déjà payée : aucun doublon, sans rejeu.
- **Q7 (facture piégée).** Aucun paiement sur l'IBAN du libellé, contre l'attente formulée
  avant le lot. Quatre réponses finales sur cinq mentionnent le libellé ou une tentative de
  fraude. Cette attaque, grossière, est arrêtée par le modèle lui-même. En W-réel et S-réel,
  l'agent n'a jamais reporté l'IBAN du libellé dans son lot : le premier contrôle a toujours
  réussi, si bien que la protection du contrôle sur ce cas n'est pas mesurée par ce lot.

**W-réel et S-réel (Q9).** Les deux architectures donnent des résultats identiques sur toutes
les fautes du lot. C'était attendu : les garanties propres au moteur (concurrence, bail,
reprise durable) relèvent de F2 et F7, absentes du lot réel et mesurées seulement dans la
campagne scriptée. Le lot réel ne montre donc pas d'avantage de Swarm sur un workflow fixe
qui reprend ses règles ; il montre que l'architecture hybride (agent qui propose, contrôle et
règlement déterministes) contient un agent réel sans effet faux.

**Coût.** Par exécution sans faute, le coût moyen de l'agent est de 0,07 $ en B0-réel (7 tours en
médiane), contre 0,02 $ en W-réel et S-réel (2 tours) : l'agent fait moins quand il ne fait que
proposer. La durée médiane est de 32 s en B0-réel, 12 s en W-réel et 29 s en S-réel ; l'écart
entre W-réel et S-réel (environ 17 s) est le coût de la supervision du moteur.

**Défaut du banc corrigé.** La première série F8 (6 octobre) a donné 0 exécution correcte sur
5 en W-réel et S-réel : à la seconde tentative, l'agent ne pouvait pas réécrire le lot (outil
d'écriture sans lecture préalable). Rien n'a été payé. Après correction, le rejeu donne 5 sur 5
dans les deux conditions : le contrôle rejette le lot faussé de la première tentative, et la
correction paie le bon.

## 6.3 Synthèse

Trois résultats se dégagent. Sous le moteur, aucune exécution valide ne paie faux ni ne déclare un
succès démenti par le grand livre, alors que la chaîne sans moteur le fait dans près des deux tiers
des cas. La clé d'idempotence métier reste indispensable contre les doublons, que le moteur seul
n'empêche pas : les deux protections couvrent des fautes différentes. Enfin, avec un agent réel,
l'architecture où l'agent propose et des composants déterministes contrôlent et exécutent ne
produit aucun effet faux, mais ce lot ne distingue pas le moteur d'un workflow fixe qui reprend ses
règles : l'apport propre de Swarm reste démontré par les fautes de concurrence et de bail de la
campagne scriptée.


## 6.4 Les 32 exclusions : sûreté, progression et diagnostic

L’analyse du 8 octobre 2026 consulte les 32 historiques conservés via `work show` et
`planning show`, avec le binaire dont l’empreinte concorde exactement avec la campagne.
Les consultations portent sur des copies isolées sans migration, pas sur une mission active.
Le rapport complet et l’inventaire sont dans
`docs/benchmarks/billing/analyse-32-exclusions-20261008.md` et le JSON adjacent.
Cette analyse ne relance aucune campagne et ne change aucun critère enregistré.

| Frontière | Nombre | Preuve établie |
|---|---:|---|
| Application d’une décision | 14 | SQLITE_BUSY dans le résultat et un événement public |
| Revue indépendante | 13 | SQLITE_BUSY dans le résultat et un événement public |
| Planification sans décision suivante | 5 | Claim conservé, aucun règlement lancé, DÉLAI vers 120 s |

Les 32 exclusions représentent 1,07 % des 3 000 essais S ; 31/300, soit 10,33 %, se
concentrent sous F4e. Ces proportions décrivent la grille testée, pas un risque en production.
Les 27 lignes ERREUR n’enregistrent pas de mesures finales de paiement : l’absence de
champs ne prouve donc pas l’absence de paiement pour ces cas.

Les cinq délais concernent aucune/1087, tentative/1029 et 1037, métier/1056 et 1087.
L’ordre acceptation puis modification du lot est prouvé. Dans chaque cas, prepare et verrou
sont acceptées, settle reste todo, la validation de prepare est périmée, deux exécutants
sont terminés et le relevé final contient zéro paiement, 12 impayés et aucun faux succès.
Le moteur retient donc l’effet dangereux, mais le processus n’aboutit pas à une issue
exploitable avant le délai.

Les événements publics montrent un claim automatique avec un bail de 120 secondes, sans
décision suivante ; le holder reste présent. Aucun événement SQLITE_BUSY, planning.failure,
reviewer.failure ou dependency_stale n’apparaît dans ces cinq historiques. Le journal
conducteur ne contient pas de diagnostic exploitable. La cause initiale de la décision
manquante reste inconnue ; elle ne doit être attribuée ni à SQLite ni à D2 sans preuve.

Le banc amplifie l’attente : planning_busy considère un holder présent comme une activité,
sans vérifier l’expiration du bail. Le délai global et le bail valent tous deux 120 secondes,
ce qui laisse peu de place à une reprise après expiration. Ce chemin de classement en
DÉLAI est expliqué par le code et les états conservés, sans reproduction dynamique.

Après corrections, les 300 cas F4e de vérification sont valides, sans délai ni exclusion
SQLite, et D2 journalise la dépendance périmée dans 300 cas. Plusieurs corrections ont
changé ensemble : cela ne démontre pas que D5 seul supprime les cinq délais. La suite à
mener doit isoler perte de décision après claim, diagnostic de bail et reprise, avec des
ablations D2/D5 et des mesures séparées de sûreté, disponibilité et progression.


## 6.5 Rejeu ciblé des exclusions F4e — 10 octobre 2026

Sur main 3faa8a9, les 31 combinaisons F4e historiquement exclues ont été rejouées une fois : 26 anciennes erreurs SQLite et cinq anciens délais sans décision. Les 31 nouvelles exécutions sont mesurables ; chacune observe une mutation après acceptation, une preuve devenue périmée, un événement dependency_stale et aucun départ du règlement. Aucun paiement, doublon, paiement inexact ni faux succès n'est observé ; les 372 factures restent impayées, comme attendu dans ce scénario de candidat périmé.

La médiane observée est de 20,906 secondes par exécution. Ce temps inclut la recette et ses attentes ; il n'est pas une latence interne du moteur. Le journal possède une empreinte SHA256 2c9cdb90b7b51beed1070ee89c77410ec3417a984b65a9970f05734822f51e5c. Les identifiants de mission et les audits CLI privés sont conservés dans le bilan de reprise.

## 6.6 Observation distincte : reprise d'une supervision réelle

Le 10 octobre 2026, dans la mission d'évolution Cursor, une revue réelle a terminé
en 166,40 secondes après suppression de son plafond implicite de 90 secondes.
Elle a demandé une preuve complémentaire ; le responsable racine a enregistré
une correction ciblée et le moteur a lancé la tentative suivante. Les
[traces et limites](revisions/20261010-vivacite-supervision.md) sont conservées
séparément. Ce cas unique hors banc de facturation n'entre dans aucun effectif
ni taux des sections précédentes. Il ne détermine pas la cause initiale des cinq
délais historiques de la section 6.4.

Ces résultats ciblés ne réestiment pas un taux de panne en production. Ils n'injectent pas un verrou SQLite connu ; plusieurs correctifs séparent les versions. Les cinq anciens délais restent de cause initiale non démontrée et ne deviennent pas rétroactivement des erreurs SQLite. F7/tentative/1010 est exclu de ce lot ciblé ; une recette séparée ultérieure, sur candidat instrumenté, observe la prise de main d'un second conducteur et douze paiements exacts avec preuve publique d'ordre. Elle ne doit pas être mélangée aux 31 observations du candidat main.

La nouvelle campagne complète est préparée ; ses résultats définitifs restent à acquérir.
