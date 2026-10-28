# 8. Discussion

## 8.1 Où placer la frontière entre tolérance et fermeture

Cursor rapporte qu'exiger une correction complète avant chaque intégration sérialisait le travail,
et que son système accepte un taux d'erreur faible en comptant sur des corrections ultérieures
[Lin 2026a]. Notre conception fait le choix inverse : en cas de doute, le moteur bloque. Les deux
choix ne s'opposent pas ; ils s'appliquent à des effets de nature différente. Une erreur de code se
corrige au commit suivant ; un virement exécuté ne se corrige que par une nouvelle opération,
souvent hors du système qui l'a émis.

Nous proposons de fixer la frontière selon la réversibilité de l'effet, et non selon le type
d'agent ou de tâche. Une même mission peut combiner une phase exploratoire tolérante (analyse,
préparation du lot) et une phase d'exécution fermée (règlement) ; la séparation des rôles rend
cette combinaison explicite, puisque seule la seconde phase détient la capacité d'effet. Les
mesures donnent le prix de cette fermeture : une durée multipliée par 8 à 30 avec des agents
scriptés, et environ 17 secondes de plus qu'un workflow fixe avec un agent réel. Ce prix est
élevé pour une boucle interactive, négligeable pour un lot de paiements traité quelques fois par
jour. Le coût en blocages à tort, l'autre moitié de la question QR2, n'est pas encore mesuré.

## 8.2 Clé d'idempotence, juge et exécution déterministe

Trois réponses existent au rejeu d'un effet. La clé fournie par l'appelant suppose une requête
identique, hypothèse que les agents violent [Zheng 2026]. Le juge d'équivalence d'ACRFence accepte
cette variabilité et décide après coup si deux requêtes désignent la même intention [Zheng 2026].
L'exécution déterministe étudiée ici supprime la variabilité à la source : l'émetteur de l'effet
n'est pas un modèle, et la clé dérive de l'identité métier.

Les résultats précisent la place de chacun. La clé métier suffit contre la répétition à
l'identique (H1), avec ou sans moteur ; elle ne protège ni contre un candidat modifié ni contre un
faux succès, que seule la médiation arrête (H2, H3). La troisième réponse a un coût : elle exige
que l'effet se décrive par un candidat structuré, un lot de lignes de paiement, avant d'être
exécuté. Elle convient aux processus dont les actions sont énumérables (paiements, écritures,
ordres), mal aux tâches où l'action se construit au fil de l'interaction. Pour celles-ci, une
approche par juge reste nécessaire, et les deux peuvent coexister dans une même architecture.

## 8.3 Ce que l'agent réel a appris

Le lot avec un agent réel a produit trois enseignements qu'aucun agent scripté ne pouvait donner.
D'abord, l'agent seul n'a jamais payé faux : quand il échouait, il s'arrêtait pour demander une
approbation. La médiation ne sert donc pas ici à corriger un agent maladroit, mais à garantir le
comportement face aux fautes de l'infrastructure, que l'agent ne voit pas. Ensuite, la fraude au
changement d'IBAN testée a été repérée par le modèle lui-même : une attaque aussi grossière ne
mesure pas l'apport des contrôles, et des fraudes plausibles restent à construire. Enfin,
l'isolement annoncé par un client d'agents ne vaut que testé : le mode restreint laissait lire un
fichier hors du répertoire par un chemin détourné.

## 8.4 Le moteur comme source de faute

Un moteur déterministe applique une règle erronée avec la même régularité qu'une règle juste. Ray
souligne que le blocage modifie lui-même la suite de l'exécution [Ray 2026], et la taxonomie MAST
inclut les défauts de conception du système [Cemri 2025]. QR3 vise une attribution
plus précise aux mécanismes du moteur et aux règles, sans nier cette catégorie existante. Le chapitre 7 en donne des exemples
concrets : un verrou transitoire transformé en échec durable, une indisponibilité traitée comme
une faute de l'agent, une garantie appliquée à la clôture plutôt qu'avant l'effet. Aucun de ces
défauts n'a produit de paiement faux dans nos campagnes ; tous ont produit des blocages, des
exclusions ou des diagnostics trompeurs. C'est l'argument pour traiter le moniteur comme un objet
d'évaluation, et pour l'extension de taxonomie proposée en QR3.

## 8.5 Supervision humaine et preuves conservées

Le règlement européen sur l'intelligence artificielle impose une supervision humaine des systèmes
à haut risque (article 14 du règlement (UE) 2024/1689, référence à vérifier avant toute
publication), et Han propose une délégation conditionnée à la conservation des preuves dans la
finance [Han 2026]. Dans cette conception, l'humain intervient à deux moments : il fixe la
politique de validation et les prérequis entre exigences avant l'exécution, et il arbitre les
blocages que le moteur lui renvoie. La conservation des preuves liées à l'empreinte du candidat
répond à l'écart de vérifiabilité décrit par Han : une décision passée reste vérifiable même si le
modèle qui a produit le candidat a changé.

## 8.6 Menaces sur la validité

**Validité interne.**

- *Agents scriptés.* La campagne principale garantit la reproductibilité et l'injection exacte des
  fautes, mais ne reproduit pas les erreurs propres aux modèles. Le lot avec agent réel ne couvre
  qu'un sous-ensemble des fautes, avec cinq essais.
- *Injection validée par contrôle positif.* Ce contrôle prouve que l'injection agit, pas qu'elle
  est représentative des incidents réels.
- *Auteur unique.* Le moteur, ses règles, le banc et les critères sont écrits par le même auteur.
  Les composants du banc ont été relus par des revues indépendantes en deux étapes, conformité
  puis qualité, mais un biais de conception commun reste possible.
- *Condition B1.* Le contrôle par appel retenu vérifie le bénéficiaire et le plafond, pas le
  montant attendu ni le doublon ; la comparaison vaut pour ce B1 précis.
- *Durées de B.* Les conditions B0 et B1 tournent sur quatre exécutants parallèles et leurs
  durées dérivent avec la charge de la machine ; elles ne se comparent pas entre elles à mieux
  qu’un facteur 2 à 3. Le ratio de H5 dépend lui aussi de cette charge ; seule la
  direction du surcoût est observée dans ce protocole.

**Validité externe.** Un seul scénario métier, sur données synthétiques ; une API de paiement
idéalisée, qui déduplique parfaitement par clé, sans limite de conservation des clés ; un seul
moteur, dans une version donnée ; un seul modèle pour les agents réels.

**Validité de construction.** Les mesures sont relevées dans le grand livre. Le blocage à tort
suppose une référence humaine, non encore établie. La définition du succès déclaré varie selon la
condition ; elle a été fixée avant les campagnes, et la lecture a posteriori des réponses des
agents réels en montre la limite en B0-réel.

**Validité des conclusions.** Les taux sont donnés avec leur intervalle de Wilson ; les exclusions
sont publiées case par case. Le lot réel, avec cinq essais, décrit des comportements et ne mesure
pas de taux. Chaque résultat porte le commit, l'état de l'arbre et les empreintes du banc et du
binaire ; la campagne scriptée a été exécutée sur un arbre propre, empreintes inchangées du début à
la fin.


## 8.7 Ce que peut renforcer une réplication

La distinction centrale est entre sûreté et progression. Retenir un paiement sur preuve périmée peut être correct tout en laissant le processus métier inachevé. Un moteur utile doit expliquer cet arrêt et permettre une reprise causale autorisée ; il ne doit ni payer malgré l'incertitude ni régénérer aveuglément les agents. Les indicateurs doivent distinguer effet indu, impayé attendu, blocage injustifié, délai, reprise et consommation.

Une réplication de milliers de cases peut renforcer la reproductibilité de ces mécanismes sur la grille choisie et révéler des régressions. Elle ne transforme pas des acteurs scriptés en agents LLM autonomes, ni une API synthétique en infrastructure bancaire. Pour évaluer l'apport propre de l'orchestration, il reste nécessaire d'aligner les contrôles d'un workflow fixe, de tester des ablations et d'exercer de vrais rôles de coordination. La revue indépendante du protocole et des analyses reste distincte des tests et de la supervision qui les a produits.
