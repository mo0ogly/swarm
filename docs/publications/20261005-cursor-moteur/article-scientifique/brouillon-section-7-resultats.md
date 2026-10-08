# Brouillon — section 7 : résultats

Statut : brouillon du 7 octobre 2026. Chaque chiffre vient des fichiers de résultats du banc,
par les modules d'analyse figés avant les données (`tables.py`, `hypotheses.py`,
`tables_reel.py`) ; le rapport Word `Resultats_banc_facturation_20261007.docx` les reproduit.

Numérotation : la section 8 du plan (« transposition ») est absorbée par les sections 4 et 6 ;
les résultats deviennent la section 7. Le protocole d'attribution (QR3), pré-enregistré sous le
titre « section 7 », remplace le § 6.9 (amendement du 6 octobre) et prend ce numéro.

Sources :

| Série | Fichier | Exécutions | Provenance |
|---|---|---|---|
| Campagne scriptée | `resultats/campagne-20261005.jsonl` | 9 000 | commit `de5cc67`, dépôt propre, empreintes du banc et du binaire inchangées du début à la fin (19 h 49 min) |
| Lot réel | `resultats/lot-reel-20261006.jsonl` | 80 | branche `feat/billing-bench-reel` |
| Rejeu F8 | `resultats/lot-reel-F8-20261007.jsonl` | 10 | après correction du banc (amendement du 7 octobre) |

---

## 7. Résultats

### 7.1 Campagne scriptée

**Validité.** B0 et B1 : 3 000 exécutions valides sur 3 000 chacune. S : 2 968 sur 3 000 ;
les 32 exclues (27 ERREUR, 5 DÉLAI) viennent toutes d'un verrou SQLite transitoire non
réessayé par le moteur (défaut D5) : 31 sous F4e, 1 sous F7. Une case dépasse le seuil de 10 %
(S, clé métier, F4e : 13 exclues sur 100) ; elle est signalée, non retirée (§ 6.7). Tous les
contrôles positifs de B0 ont produit le défaut attendu, et le contrôle négatif aucun défaut,
dans les trois conditions.

**Tableau 3.** Exécutions valides présentant au moins un doublon / un paiement inexact / un
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
[0 ; 3,7 %].

**Hypothèses** (§ 6.10, verdicts calculés par des règles fixées avant la fin de la campagne) :

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
  parallèles, dérivent avec la charge de l'hôte (§ 10.1) ; le facteur n'en dépend pas.
- **H6** relève de l'étiquetage des blocages (§ 6.9) ; non établie à ce jour.

**Qui arrête quoi.** Sous F4 (lot modifié après le départ du règlement), l'arrêt revient au
règlement dans 100 % des exécutions valides de S : son contrôle d'empreinte refuse le lot
modifié. Sous F4e (lot modifié après acceptation, avant le départ du règlement), il revient au
moteur dans toutes les exécutions valides : la preuve de `prepare` n'est plus fraîche et le
règlement ne part pas. Les deux mécanismes se complètent ; aucun ne couvre seul les deux
moments.

**Ce que le moteur ne fait pas.** Sous F5 (service indisponible), aucune exécution n'absorbe
la faute, dans aucune condition ; S laisse les factures impayées, sans faux succès, mais ne
distingue pas l'indisponibilité d'un échec du contrôle (défaut D3). Sous F1 et F3, la
protection contre le doublon vient de la clé métier, non du moteur : avec une clé par
tentative, F3 produit encore un doublon dans S.

### 7.2 Agents réels

Lot exploratoire : cinq essais par scénario, un modèle (`claude-sonnet-5`), un client (Claude
Code 2.1.280). Coût déclaré : 3,12 $ pour le lot, 0,36 $ pour le rejeu F8 ; quatre appels sans
coût déclaré (agents tués sous F3).

**Tableau 4.** Exécutions correctes sur valides, par faute et condition.

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
alors que l'agent n'annonçait aucune réussite (lecture a posteriori, § 6.8 bis).

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

### 7.3 Attribution des blocages (QR3)

Non réalisée à ce jour. Le corpus (lot S conservé, § 6.9) et l'étiquetage restent à produire.

---

## Points à vérifier

Vérifiés sur les fichiers : coût moyen B0-réel sans faute 0,369 $ / 5 = 0,07 $ (le texte dit
« coût de l'agent », c'est une moyenne, pas une médiane) ; sous F1, un rejeu absorbé dans trois
exécutions valides sur quatre, aucune demande sans clé (journal des demandes).

1. Section 5.3 : ajouter D6 (couverture des exigences à la clôture, pas avant l'effet) et la
   seconde voie de D5.
