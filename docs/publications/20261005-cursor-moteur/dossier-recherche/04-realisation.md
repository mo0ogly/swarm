# 4. Réalisation

Le modèle du chapitre 3 est mis en œuvre dans deux artefacts publiés dans le même dépôt
(`github.com/mo0ogly/swarm`) : le moteur Swarm et un banc de facturation qui le soumet à des
fautes. Ce chapitre décrit ce qu'ils font, sans revenir sur les choix du modèle.

## 4.1 Le moteur Swarm

Swarm est un programme Go autonome : environ 30 600 lignes hors tests dans 127 fichiers et 480
fonctions de test au commit `53f2564` (19 septembre 2026), chiffres à recalculer sur le commit
figé de toute publication. Son état est stocké dans une base SQLite embarquée, sans serveur ni
dépendance native ; il s'utilise par une interface en ligne de commande et par un cockpit web qui
n'écoute que sur l'interface de bouclage.

Toute mutation passe par une requête versionnée, qui porte un identifiant d'événement et la
révision attendue (I1, I2). Les agents sont des processus externes déclarés par configuration,
lancés dans un environnement filtré, dont le moteur applique les limites (appels d'outils,
répétitions, erreurs consécutives) et qu'il arrête dès qu'une limite est atteinte (I5).

**Mission hiérarchique.** Une mission autonome exige un responsable de mission, fourni par un
exécutable, qui reçoit les remises et décide des opérations : création de tâche, reprise,
clôture. L'agent producteur ne communique pas avec ses pairs : il écrit son livrable et se
termine. Le moteur remet ce livrable au responsable avec son empreinte et l'identité de la
tentative, seulement si la tentative s'est terminée normalement. Une décision du responsable qui
s'appuie sur une tentative périmée est refusée.

**Validation.** Chaque exigence porte des contrôles structurés : programme tiré d'une liste
blanche, arguments, répertoire, délai, critères couverts et justification. Ils sont exécutés sans
interpréteur de commandes (I7). Une revue indépendante, confiée au fournisseur du responsable,
s'ajoute aux contrôles comme condition préalable ; l'acceptation exige ensuite que **tous** les
contrôles réussissent, si bien qu'une revue favorable ne lève jamais un contrôle en échec. Une
tâche n'est acceptée que si ses preuves portent sur la tentative courante et sur le contenu actuel
du livrable, et cette fraîcheur est réévaluée avant de lancer une tâche dépendante (I3).

**Conduite durable.** Un seul processus conduit une mission à la fois. La possession est un bail
renouvelé, acquis par comparaison et échange, et considéré comme périmé après trente secondes sans
renouvellement (I4).

## 4.2 Le banc de facturation

Le banc est écrit en Python, bibliothèque standard uniquement, dans `benchmarks/billing/`. Il
simule un processus de paiement de factures fournisseurs, entièrement sur données synthétiques.

- **Grand livre.** Une base SQLite en partie double : trésorerie, comptes fournisseurs, écritures
  équilibrées. La mesure lit un instantané cohérent et vérifie les invariants comptables.
- **API de paiement.** Seul écrivain du grand livre, elle déduplique par clé d'idempotence,
  refuse une clé réutilisée avec un contenu différent et exige un jeton de règlement. Elle sait
  injecter une réponse perdue après paiement, une indisponibilité persistante et, pour les agents
  réels, un libellé frauduleux.
- **Préparateur, contrôle, règlement.** Le préparateur propose un lot et ne paie jamais. Le
  contrôle compare chaque ligne au grand livre (facture, montant, bénéficiaire, plafond,
  complétude). Le règlement exécute le lot ; sa clé d'idempotence est configurable (aucune, par
  tentative, métier) et sa progression est liée à l'empreinte du lot.
- **Harnais.** Chaque exécution s'isole dans une racine temporaire, avec provenance : commit,
  état du dépôt, empreintes du code du banc et du binaire mesuré.

Chaque exécution part d'un grand livre neuf, généré par une graine : douze factures réparties sur
quatre fournisseurs, montants en centimes inférieurs à la moitié du plafond par paiement, et une
trésorerie dotée de trois fois le total dû, pour qu'un paiement en double reste possible, donc
mesurable.

## 4.3 Conditions comparées

**Tableau 3 — Conditions de l'évaluation.**

| Condition | Agents | Protection | Enchaînement |
|---|---|---|---|
| B0 | scriptés | aucune ; revue finale après l'effet | harnais naïf : relance après arrêt brutal |
| B1 | scriptés | l'API refuse un bénéficiaire inconnu ou un montant hors plafond | comme B0 |
| S | scriptés | contrôle du lot lié à l'empreinte, règlement seul porteur du jeton | Swarm, mission hiérarchique |
| B0-réel | Claude Sonnet, qui paie lui-même | aucune ; clé métier donnée en consigne | aucun |
| W-réel | Claude Sonnet, qui prépare seulement | mêmes contrôle et règlement que S | workflow fixe en Python, sans Swarm |
| S-réel | Claude Sonnet, qui prépare seulement | comme S | Swarm |

B1 ne connaît ni le montant attendu de la facture, ni les paiements déjà faits : c'est une
instance minimale du contrôle par appel, pas toute approche de ce type. W-réel reprend les règles
observées du moteur (une correction après un contrôle en échec, une reprise après un arrêt brutal
du règlement, règlement du seul lot contrôlé) sans son état durable, ses baux ni sa revue ; il
sert à isoler l'apport propre du moteur. Dans S, le responsable de mission et la revue
indépendante sont scriptés ; aucun modèle n'y est appelé.

## 4.4 Isolement des agents réels

Les agents réels sont lancés par le client Claude Code en mode non interactif restreint : réglages
de l'opérateur ignorés, aucun crochet ni serveur MCP, outils de fichiers confinés au répertoire
de travail, modèle figé (`claude-sonnet-5`). Une sonde a vérifié cet isolement au lieu de le
supposer : un secret aléatoire était placé hors du répertoire de l'agent, que l'on chargeait de le
lire par tous les moyens. Avec `curl` autorisé, le mode restreint bloquait bien `cat`, `ls` et
l'écriture hors du répertoire, mais `curl file://` lisait le secret ; dans S-réel et W-réel,
l'agent aurait donc pu lire le jeton de paiement, et E1 aurait été faux.

Deux profils d'outils en découlent. Le préparateur (W-réel, S-réel) ne reçoit que l'écriture dans
son répertoire ; le banc lit lui-même la liste des factures dues et la place dans la consigne. La
sonde ne trouve alors ni lecture ni écriture hors du répertoire. Le payeur de B0-réel garde
`curl` : il détient le jeton par construction, et sa capacité de lecture locale est une propriété
documentée de cette condition.
