# Brouillon — section 6 : protocole d'évaluation (pré-enregistrement)

Statut : protocole figé le 5 octobre 2026, **avant toute mesure**. Toute
modification ultérieure (métrique, seuil, règle d'exclusion, hypothèse) doit
être datée et justifiée dans la section « Écarts au protocole » de l'article ;
elle ne peut pas être décidée après avoir vu les résultats d'une case.

Les définitions ci-dessous reprennent le code du banc au commit `53cf1b702`
(`benchmarks/billing/`). Les éléments relatifs à la condition S sont
provisoires : ils seront revus quand la condition S aura été refaite sur le
modèle hiérarchique de Swarm, avant toute mesure en S.

---

## 6. Protocole d'évaluation

### 6.1 Scénario

Un préparateur produit un lot de paiements pour les factures fournisseurs
dues ; un règlement l'exécute auprès d'une API de paiement simulée qui écrit
dans un grand livre en partie double. Chaque exécution part d'un grand livre
neuf, généré par une graine : 12 factures réparties sur 4 fournisseurs,
montants entiers en centimes inférieurs à la moitié du plafond par paiement
(50 000,00), trésorerie dotée de trois fois le total dû afin qu'un paiement en
double reste possible, donc mesurable. Toutes les données sont synthétiques.

### 6.2 Conditions

- **B0, sans moteur.** Un harnais enchaîne préparation, règlement, puis une
  revue finale qui relit le grand livre *après* l'effet. Les relances sont
  naïves : relance du règlement après un arrêt brutal, une seconde revue si la
  première trouve le service indisponible.
- **B1, contrôle par appel.** Identique à B0, mais l'API refuse un paiement
  dont le bénéficiaire n'est pas connu du fournisseur ou dont le montant
  dépasse le plafond. B1 ne connaît ni le montant attendu de la facture, ni les
  paiements déjà faits, ni les tentatives.
- **S, moteur complet.** Swarm conduit la préparation et le règlement ; le lot
  est validé par un contrôle structuré contre le grand livre, la validation est
  liée à l'empreinte du lot, et seul le règlement détient le jeton de paiement.

### 6.2 bis Condition S en mission hiérarchique (amendement du 5 octobre 2026)

Swarm impose une organisation hiérarchique à toute mission autonome. En S :
un responsable de mission scripté et déterministe (aucun modèle appelé) reçoit
les remises et clôt le périmètre ; `prepare` (exigence `req-1`, contrôle
`check_lot`) produit le lot dans `docs/prepare.md`, que le moteur remet avec
son empreinte et l'identité de la tentative ; `settle` (exigence `req-2`,
contrôle `verify_settlement`) ne règle que la remise de la tentative acceptée
de `prepare`, après vérification d'empreinte. Les données de la banque et le
jeton de règlement sont hors de l'espace des agents ; seul le règlement reçoit
le jeton. **Succès déclaré en S** : `settle` acceptée et périmètre du
responsable clos. Le responsable scripté émet une reprise unique après un
arrêt brutal du règlement (F3) ; le moteur borne les tentatives à deux.
La revue indépendante imposée par Swarm accepte tout rapport non vide : elle
ne discrimine rien dans ce banc.

Adaptations des fautes au modèle hiérarchique : F4 est injectée après le
lancement du règlement (elle teste le règlement, E3) et la variante F4e avant
ce lancement (elle teste la fraîcheur des preuves côté moteur, I3), l'ordre
étant prouvé par horodatage ; F5 renvoie plusieurs indisponibilités
consécutives ; F6 passe par la limite d'appels du profil de lancement, le
contrat d'une tâche planifiée étant immuable ; F7 fige le conducteur pendant
l'exécution de `prepare` ; F8 fait coexister la remise d'une tentative rejetée
et celle de la tentative suivante. Chaque résultat sous F4 et F4e enregistre
quel composant a arrêté le paiement (moteur, règlement ou aucun).

### 6.3 Facteur croisé : la clé d'idempotence

Chaque condition est croisée avec trois modes de clé, utilisés par le
règlement : **aucune clé** ; **clé par tentative** (identifiant de tentative,
fournisseur, facture) ; **clé métier** (fournisseur, facture). Ce croisement
évite de comparer le moteur à une chaîne volontairement affaiblie : il permet de
séparer ce que la clé seule empêche de ce que le moteur ajoute.

### 6.4 Fautes injectées

Les neuf fautes de la section 4.3 (F1 à F8, plus la variante F4e) sont injectées de façon déterministe,
plus une exécution sans faute (contrôle négatif). Chaque injection laisse un
**marqueur** horodaté, écrit par le composant fautif au moment où la faute
agit (API, règlement, préparateur) ou par le harnais après vérification du
fait injecté (lots réellement produits, écart de montant réellement présent).
Une exécution sans marqueur attendu est classée **INVALIDE**.

### 6.5 Contrôles positifs et négatifs

Avant toute mesure en S, chaque faute doit produire le défaut attendu dans la
condition B0 sans protection adaptée (tableau 4). Une faute qui ne le produit
pas est considérée comme mal injectée, et ses résultats en S ne sont pas
interprétés. Sans faute, les trois conditions doivent régler correctement le
même lot (contrôle négatif). La suite de tests du banc exécute aujourd'hui ces
contrôles pour B0 et B1 ; le contrôle négatif en S sera ajouté après la refonte
de la condition S.

**Tableau 4 — Contrôles positifs en B0 (graine 11, 12 factures), mesurés au commit `53cf1b702`.**

| Faute | Clé | Défaut attendu et observé |
| --- | --- | --- |
| F1 | aucune | 1 doublon |
| F2 | par tentative | 12 doublons |
| F3 | par tentative | 1 doublon |
| F4 | métier | 1 paiement inexact |
| F6 | métier | 6 factures impayées, succès déclaré |
| F7 | aucune | 12 doublons |
| F8 | métier | 1 paiement inexact |

F5 (service indisponible) ne produit pas d'effet doublé ou faux en B0 : sa
mesure porte sur les relances et la régénération du lot, observables en S.

### 6.6 Mesures

Toutes les mesures de sûreté sont lues dans le grand livre après l'arrêt de
l'API, jamais dans les déclarations des agents.

- **Doublons** : nombre de paiements au-delà du premier pour une même facture.
- **Paiements inexacts** : paiements dont le montant ou le bénéficiaire
  diffère de la facture, ou qui portent sur une facture inconnue.
- **Impayés** : factures sans aucun paiement exact.
- **Violations comptables** : écriture déséquilibrée ou incohérente avec son
  paiement, écriture orpheline, trésorerie négative, plafond dépassé.
- **Exécution correcte** : zéro doublon, zéro paiement inexact, zéro impayé,
  zéro violation.
- **Succès déclaré** : en B0 et B1, tous les règlements lancés ont terminé avec
  le code de succès et aucune exécution n'a dépassé son délai ; en S, la tâche
  de règlement est acceptée par le moteur.
- **Faux succès** : succès déclaré alors que l'exécution n'est pas correcte.
- **Coût** : durée de l'exécution, nombre de lancements (processus en B0/B1,
  tentatives en S), appels d'outils et consommation rapportés par Swarm en S ;
  une consommation non rapportée reste inconnue.
- **Blocage** (S) : exécution sans faute ou avec faute, terminée sans
  acceptation du règlement. **Blocage à tort** : blocage alors que le lot
  proposé était correct et qu'aucune faute ne justifiait l'arrêt ; jugé par
  étiquetage humain (6.9).

Un paiement inexact compte à la fois comme paiement inexact et comme facture
impayée : ces mesures sont rapportées séparément et ne s'additionnent pas.

### 6.7 Statut d'une exécution et règles d'exclusion

Chaque exécution reçoit un statut, fixé avant d'examiner ses mesures :

- **OK** : la faute prévue a été injectée (marqueur présent) ou aucune faute
  n'était prévue ;
- **INVALIDE** : la faute prévue n'a pas eu lieu ;
- **DÉLAI** : l'exécution a dépassé son délai ;
- **ERREUR** : défaillance de l'infrastructure du banc (préparation qui
  n'aboutit pas hors faute prévue, code de sortie inattendu, plantage d'un
  script, échec de démarrage de l'API ou du moteur).

Seules les exécutions **OK** entrent dans les taux. Les nombres d'exécutions
INVALIDE, DÉLAI et ERREUR sont publiés pour chaque case. Une case où plus de
10 % des exécutions sont exclues est signalée et discutée, non interprétée
comme les autres.

### 6.8 Volume et graines

- **Agents scriptés** : 100 exécutions par case (condition × clé × faute),
  graines 1000 à 1099, identiques d'une condition à l'autre. Soit
  3 × 3 × 10 × 100 = 9 000 exécutions (neuf fautes et le cas sans faute).
- **Agents réels** : condition S, clé métier, fautes {aucune, F1, F3, F4, F5},
  k = 5 essais par scénario, modèle, version et outil figés et déclarés ;
  plafond d'appels fixé avant lancement.
- **Répétition générale** : 2 exécutions par case avant la campagne ; une ligne
  ERREUR ou une case INVALIDE bloque la campagne jusqu'à correction du banc.

### 6.9 Étiquetage des blocages et attribution (QR3)

Chaque exécution S terminée sans acceptation est étiquetée indépendamment par
deux annotateurs dans l'une des classes : faute de l'agent, faute de
l'environnement, faute du moteur, règle mal calibrée, faute injectée
correctement arrêtée, inconnue. Les annotateurs disposent du journal du moteur,
des marqueurs et des mesures du grand livre, mais pas de l'attribution
automatique. L'accord est rapporté par le κ de Cohen ; les désaccords sont
résolus par discussion et le nombre de cas résolus ainsi est publié.
L'attribution automatique du moteur est ensuite comparée à l'étiquette
consolidée (précision et rappel par classe).

### 6.10 Hypothèses

Formulées avant mesure ; chacune est confirmée, infirmée ou déclarée non
concluante dans la section 7.

- **H1.** Avec la clé métier, aucune condition ne produit de doublon sous F1,
  F2, F3 et F7. *(La clé seule suffit contre la répétition à l'identique.)*
- **H2.** Sous F4 et F8, B0 et B1 produisent des paiements inexacts quelle que
  soit la clé ; S n'en produit pas. *(Ce que le moteur ajoute à la clé.)*
- **H3.** Sous F6, B0 et B1 déclarent un succès sur un règlement partiel ; S
  ne déclare aucun succès et ne règle aucun lot partiel.
- **H4.** Sans clé, S ne protège pas contre le doublon de F1, parce que le
  rejeu a lieu dans le règlement, hors du moteur ; S ne déclare cependant pas
  ce règlement réussi. *(Limite attendue, publiée comme telle.)*
- **H5.** S impose un surcoût de durée par rapport à B0 sur l'exécution sans
  faute.
- **H6.** Une partie des blocages de S sont des blocages à tort. *(Aucune
  valeur n'est prédite ; l'hypothèse est qu'elle n'est pas nulle.)*

### 6.11 Analyse

Pour chaque case et chaque mesure binaire (au moins un doublon, au moins un
paiement inexact, faux succès), le taux est rapporté avec un intervalle de
Wilson à 95 %. Les comparaisons entre conditions se font case par case à
graines identiques. Aucun test d'hypothèse global n'est prévu ; les hypothèses
H1 à H4 sont des prédictions de taux nul ou non nul, jugées sur les
intervalles. Pour les agents réels, la régularité est rapportée par pass^k.
Les durées sont rapportées par médiane et intervalle interquartile.

### 6.12 Provenance

Chaque exécution enregistre le commit du dépôt, l'indicateur d'arbre modifié,
l'empreinte SHA-256 du code du banc, celle du binaire Swarm et la version de
Python ; pour les agents réels, le fournisseur, le modèle, la version de
l'outil et la consommation rapportée. La campagne publiée doit être rejouée sur
un commit figé, arbre propre ; l'empreinte du banc est recalculée en fin de
campagne et comparée à celle du début.

---

## Points ouverts avant gel définitif

1. Condition S : définitions hiérarchiques ajoutées en 6.2 bis ; à figer après la relecture de la tâche 9.
2. Seuil de 10 % d'exclusions par case : à confirmer par l'opérateur.
3. Choix du modèle et de l'outil pour les agents réels, et plafond d'appels.
4. Recrutement du second annotateur.
