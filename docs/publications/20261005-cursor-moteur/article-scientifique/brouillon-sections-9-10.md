# Brouillon — sections 9 et 10

Statut : brouillon du 5 octobre 2026. La section 10 décrit le protocole tel
qu'il est conçu ; elle ne dépend pas des résultats. La section 9 pose les
questions de discussion ; les passages `[À AJUSTER]` attendent la campagne.

---

## 9. Discussion

### 9.1 Où placer la frontière entre tolérance et fermeture

Cursor rapporte qu'exiger une correction complète avant chaque intégration
sérialisait le travail, et que son système accepte un taux d'erreur faible en
comptant sur des corrections ultérieures [Lin 2026a]. Notre conception fait le
choix inverse : en cas de doute, le moteur bloque. Les deux choix ne
s'opposent pas ; ils s'appliquent à des effets de nature différente. Une erreur
de code se corrige au commit suivant ; un virement exécuté ne se corrige que
par une nouvelle opération, souvent hors du système qui l'a émis.

Nous proposons donc de fixer la frontière selon la réversibilité de l'effet,
et non selon le type d'agent ou de tâche. Une même mission peut combiner une
phase exploratoire tolérante (analyse, préparation du lot) et une phase
d'exécution fermée (règlement). La séparation des rôles de la section 4 rend
cette combinaison explicite : seule la seconde phase détient la capacité
d'effet.

[À AJUSTER : confronter au surcoût mesuré (QR2). Si les blocages à tort sont
fréquents, la frontière doit être déplacée vers des contrôles plus étroits,
pas abandonnée.]

### 9.2 Clé d'idempotence, juge LLM et exécution déterministe

Trois réponses existent au rejeu d'un effet. La clé fournie par l'appelant
suppose une requête identique, hypothèse que les agents LLM violent
[Zheng 2026]. Le juge LLM d'équivalence d'ACRFence accepte cette variabilité et
décide après coup si deux requêtes désignent la même intention [Zheng 2026].
L'exécution déterministe que nous étudions supprime la variabilité à la
source : l'émetteur de l'effet n'est pas un LLM, et la clé dérive de l'identité
métier.

Cette troisième réponse a un coût. Elle exige que l'effet puisse être décrit par
un candidat structuré (ici, un lot de lignes de paiement) avant d'être exécuté.
Elle convient aux processus métier dont les actions sont énumérables :
paiements, écritures comptables, ordres. Elle convient mal aux tâches où
l'action elle-même est construite au fil de l'interaction, comme la navigation
ou l'administration interactive d'un système. Pour celles-ci, une approche par
juge reste nécessaire, et les deux approches peuvent coexister dans une même
architecture.

### 9.3 Le moteur comme source de faute

Un moteur déterministe applique une règle erronée avec la même régularité
qu'une règle juste. Une règle de fraîcheur trop large invalide une preuve après
un changement sans portée ; un plafond trop bas interrompt un travail utile.
Ray souligne que le blocage modifie lui-même la suite de l'exécution
[Ray 2026]. La taxonomie MAST ne prévoit pas de catégorie pour ces défauts
[Cemri 2025]. Notre extension (faute du moteur, règle mal calibrée) vise à
rendre ces cas visibles plutôt qu'à les confondre avec des erreurs d'agent.

[À AJUSTER : résultats QR3 ; exemples de cas où le moteur était fautif dans la
campagne. Les présenter même s'ils sont défavorables.]

### 9.4 Supervision humaine et preuves conservées

L'article 14 de l'AI Act exige une supervision humaine des systèmes à haut
risque, et Han propose une délégation conditionnée à la conservation des
preuves dans la finance [Han 2026]. Dans notre conception, l'humain intervient
à deux moments : il fixe la politique de validation avant l'exécution, et il
arbitre les blocages que le moteur lui renvoie. Le moteur lui fournit, pour
chaque blocage, la cause, l'acteur qui peut agir et la condition de reprise.
La conservation des preuves liées à l'empreinte du candidat répond à l'écart de
vérifiabilité décrit par Han : une décision passée reste vérifiable même si le
modèle qui a produit le candidat a changé.

[À vérifier avant soumission : citation exacte de l'article 14 du règlement
(UE) 2024/1689.]

---

## 10. Menaces sur la validité

### 10.1 Validité interne

- **Agents factices.** La campagne principale utilise des fournisseurs
  scriptés et déterministes. Ils garantissent la reproductibilité et
  l'injection exacte des fautes, mais ils ne reproduisent pas les erreurs
  propres aux LLM : réécriture de la requête, lot plausible mais faux, oubli
  d'une facture. La campagne réduite avec agents réels (k essais par scénario)
  ne couvre qu'un sous-ensemble des fautes.
- **Injection validée par contrôle positif.** Chaque faute doit produire le
  défaut attendu dans la condition sans protection ; une exécution sans preuve
  d'injection est classée invalide et exclue des taux. Ce contrôle prouve que
  l'injection agit, pas qu'elle est représentative des incidents réels.
- **Auteur du moteur et des règles.** Le moteur, ses règles de validation et le
  banc sont écrits par le même auteur. Les contrôles du lot (`check_lot`) et du
  règlement ont été relus par des revues indépendantes en deux étapes
  (conformité, puis qualité), mais un biais de conception commun reste
  possible.
- **Comparaison B1.** La condition « contrôle par appel » vérifie le
  bénéficiaire et le plafond, pas le montant de facture ni le doublon. C'est un
  choix de conception ; un contrôle par appel plus riche réduirait l'écart avec
  la condition S. Les résultats doivent être lus comme une comparaison avec ce
  B1 précis, non avec toute approche par appel.

### 10.2 Validité externe

- **Un scénario, un domaine.** Le banc simule un seul processus (paiement de
  factures fournisseurs) avec des données synthétiques ; il ne valide aucun
  système bancaire réel, ni ses délais, ni ses rejets, ni ses procédures de
  rapprochement.
- **API de paiement idéalisée.** L'API simulée déduplique parfaitement par
  clé. Une API réelle peut limiter la durée de conservation des clés ou rejeter
  un rejeu dont les paramètres diffèrent ; ces comportements ne sont pas
  modélisés.
- **Un seul moteur.** Les résultats portent sur Swarm. Ils ne s'étendent pas
  sans vérification à d'autres orchestrateurs, ni à Swarm dans une version
  différente de celle mesurée.

### 10.3 Validité de construction

- **Doublons, paiements faux, impayés.** Ces mesures sont relevées dans le
  grand livre et non dans le discours des agents. Un paiement inexact compte à
  la fois comme paiement faux et comme facture impayée ; les mesures ne
  s'additionnent pas et sont rapportées séparément.
- **Blocage à tort.** Juger qu'un blocage était injustifié suppose une
  référence ; elle est établie par étiquetage humain, avec deux annotateurs et
  un accord mesuré.
- **Succès déclaré.** Un « faux succès » est une tâche déclarée réussie alors
  que le grand livre n'est pas conforme. Sa définition dépend de ce que chaque
  condition appelle « réussite » ; elle est fixée avant la campagne.

### 10.4 Validité des conclusions

- **Non-déterminisme des modèles.** Les campagnes avec agents réels sont
  répétées, et la régularité est rapportée par pass^k [Yao 2024] avec des
  intervalles de confiance.
- **Intervalles.** Les taux sont rapportés avec des intervalles de Wilson à
  95 %. Les cases où la faute n'a pas été injectée, où l'exécution a dépassé
  son délai ou a échoué pour une raison d'infrastructure sont comptées et
  publiées séparément, jamais intégrées aux taux.
- **Versions figées.** Chaque résultat porte l'empreinte du code du banc, celle
  du binaire Swarm, le commit du dépôt et, pour les agents réels, le
  fournisseur, le modèle et la version de l'outil ; Han rappelle que les mises
  à jour de modèles modifient des décisions passées [Han 2026].
- **Reproductibilité du moteur.** À la date de rédaction, une partie du code de
  Swarm exercé par le banc n'est pas commitée. La campagne publiée doit être
  rejouée sur un commit figé ; tout résultat obtenu sur un arbre de travail
  modifié est exclu.

---

## Points à vérifier

1. Article 14 du règlement (UE) 2024/1689 : intitulé exact et portée (systèmes
   à haut risque).
2. Accord inter-annotateurs : protocole d'étiquetage à écrire avant la campagne.
3. Définition du « succès déclaré » par condition : à figer dans le protocole.
