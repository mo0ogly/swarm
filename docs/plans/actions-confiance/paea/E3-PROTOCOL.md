# PAEA — protocole comparatif E3 en préparation

[English](../../../en/plans/actions-confiance/paea/E3-PROTOCOL.md) ·
[Plan PA-00…13](../PAEA-APEX.md)

**Statut : brouillon, non préenregistré, aucune campagne lancée.** Le 25 octobre
2026, l’utilisateur confirme ne disposer ni d’évaluateur indépendant ni d’équipe
tierce. Codex prépare le protocole ; il ne peut occuper ces rôles pour l’évaluation
de sa propre intégration. Ce document n’autorise aucun appel, participant ou budget.

## Question et périmètre

Mesurer ce que la composition des contrôles apporte à une facturation fictive
et à la promotion Git d’un candidat de fixture : effets interdits, réussite des
tâches, refus injustifiés, reprise, coût et charge. Comparateur principal :
workflow déterministe B2 disposant des mêmes protections métier connues.
Ne pas affaiblir B2 pour fabriquer un avantage de Swarm.

Les actions et contextes utilisent le contrat PA-01. Le calcul canonique Python
local fournit des vecteurs d’interopérabilité, pas une seconde implémentation
indépendante de l’exécution. Le tiers devra construire sa propre frontière
d’autorité/effet sur le sous-ensemble partagé, sans dépendre du moteur Swarm.

## Entrées obligatoires avant préenregistrement

| Entrée | Propriétaire | Condition vérifiable |
|---|---|---|
| Évaluateur | Commanditaire | Identité, acceptation de mission et indépendance vis-à-vis des auteurs/intégrateurs déclarées. |
| Seconde implémentation | Équipe tierce à désigner | Sources, dépendances, protocole et binaire identifiés ; exécution réellement distincte de Swarm. |
| Candidat Swarm | Intégration | PA-10 avec profil livré, configuration, schémas, TCB et inventaire fermé. |
| Corpus réservé | Évaluateur | Attaques non utilisées pour développer le candidat ; provenance et accès conservés. |
| Modèles et moyens | Commanditaire/évaluateur | Identifiants observés, outils/prompts versions, budgets persistés disponibles et droits applicables. |
| Volumes et ordre | Évaluateur | Effectifs par scénario/facteur, seeds, séquence/randomisation et règles d’arrêt fixés avant résultats. |
| Essais humains | Responsable opérationnel | Participants habilités, protocole accepté, opérations sans danger et critères de vigilance annoncés. |
| Critères métier | Responsable du risque | Réussite, faux refus, limites de coût/latence et périmètre de conséquence documentés. Aucun seuil numérique universel ajouté ici. |

Ces entrées sont des dépendances réelles, pas des tâches déjà accomplies. Une
version préenregistrée recevra un identifiant, une date, les empreintes et les
acceptations de ses propriétaires avant la première campagne.

## Conditions expérimentales

| Condition | Usage |
|---|---|
| B0 outils directs | Exploratoire seulement sur fixtures autorisées ; pas le comparateur d’utilité principal. |
| B1 hooks | Politique équivalente, interception locale ; distinguer chemins réellement couverts. |
| B2 workflow durci | Même intention, ressources, protections métier, oracle et reprise établis. |
| B3-a provenance | Ajouter la composition identité/mémoire/IFC propre aux trajectoires agentiques. |
| B3-b médiation | Ajouter l’application effective sandbox/egress/points d’effet. |
| B3-c autorité dynamique | Évaluer capacités déléguées, révocation et UNKNOWN dans les trajectoires dynamiques, sans retirer les protections métier de B2. |
| B3-d assurance | Ajouter preuve tierce et supervision humaine mesurée. |

Faire les ablations une brique à la fois, puis les interactions justifiées.
Déclarer les dépendances : une ablation qui rend une combinaison inexécutable
n’est pas une mesure comparable et ne doit pas recevoir un score artificiel.
Séparer fournisseurs scriptés, modèles réels et étude humaine ; aucun taux
d’autonomie n’est inféré d’une fixture.

## Corpus et observations

Conserver T01…T22, avec déclinaison facturation et logiciel lorsque applicable.
La table du plan est le registre initial, pas un compteur d’essais exécutés.
Avant chaque faute : même environnement et témoin légitime. Couvrir injection,
contamination mémoire, délégation, substitution après accord, révocation/commit,
nouvelle clé d’intention, crash/rejeu, accès direct, secrets/egress, cumul,
approbation, mutation de candidat/outillage, altération et rollback des preuves.

L’observateur conserve état du service avant/après, intention, autorisation,
contenu consommé, ordre, reçus, sortie effective et réconciliation. Pour Git,
ne jamais déduire l’absence d’effet d’un code retour ni l’atomicité Git/SQLite.
Pour le réseau, distinguer tentative observée et octets réellement reçus.

Chaque essai conserve candidat/TCB/configuration, seed, variante, exécuteur,
arguments, heures/durée, sorties originales, coûts connus ou inconnus, verdict
et limites. Les données sensibles sont minimisées ; une expurgation ne détruit
pas l’original autorisé ni sa provenance. Une erreur de collecte rend la preuve
incomplète ; pas de succès par défaut ou de relance inchangée.

## Analyse et verdict

Rapporter par scénario et condition : tentatives réellement distinctes, effets
interdits, réussite, faux refus, UNKNOWN/reprises, latence p50/p95/p99, dépenses
et charge. Conserver les unités et les coûts inconnus. Les attaques adaptatives
ne sont pas supposées indépendantes ; une borne binomiale ne s’applique qu’au
sous-ensemble pour lequel ses hypothèses sont établies. Zéro échec observé ne
prouve ni risque nul ni couverture universelle.

Une réfutation produit un contre-exemple original et un retour au propriétaire,
puis une correction et une nouvelle évaluation du candidat modifié. Aucun échec
requis n’est compensé par une moyenne. Les résultats négatifs face à B2 sont
publiés avec la même précision. Une déclaration E3 reste expérimentale, bornée
aux versions, hypothèses et chemins réellement évalués ; elle n’accorde aucun
droit de déploiement critique. Le RETEX PA-13 reste obligatoire.
