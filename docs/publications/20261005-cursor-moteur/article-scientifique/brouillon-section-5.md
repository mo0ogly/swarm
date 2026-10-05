# Brouillon — section 5 : implémentation

Statut : brouillon du 5 octobre 2026. Les chiffres de taille portent sur la
branche `main` du dépôt swarm au commit `53f2564` (19 septembre 2026) et doivent
être recalculés sur le commit figé de la campagne. Les références `fichier:ligne`
proviennent de `docs/benchmarks/billing/observations.md` et ont été relues par
une revue indépendante ; elles servent à la vérification et seront retirées ou
déplacées en annexe dans la version soumise.

---

## 5. Implémentation

### 5.1 Le moteur Swarm

Swarm est un programme Go autonome : environ 30 600 lignes hors tests réparties
dans 127 fichiers, et 480 fonctions de test (commit `53f2564`). Son état est
stocké dans une base SQLite embarquée (`modernc.org/sqlite`), sans serveur ni
dépendance native ; le binaire s'exécute hors ligne. Il s'utilise par une
interface en ligne de commande et par un cockpit web local qui écoute
uniquement sur l'interface de bouclage, sur un port libre choisi par le
système.

Toute mutation passe par une requête versionnée qui porte un identifiant
d'événement et la révision attendue (invariants I1 et I2). Les agents sont des
processus externes déclarés par configuration : Claude Code, Codex, ou tout
exécutable respectant le protocole de sortie ; leur environnement est filtré
et leurs limites (appels d'outils, répétitions, erreurs consécutives) sont
appliquées par le moteur, qui arrête une tentative dès qu'une limite est
atteinte (invariant I5).

**Mission hiérarchique.** Une mission autonome exige un responsable de
mission, lui-même fourni par un exécutable, qui reçoit les remises et décide
des opérations (création de tâche, reprise, clôture). L'agent producteur ne
communique pas avec ses pairs : il écrit son livrable et se termine ; le
moteur remet ce livrable au responsable avec son empreinte SHA-256 et
l'identité de la tentative, et seulement si la tentative s'est terminée
normalement. Une décision du responsable qui s'appuie sur une tentative
périmée est refusée, et l'empreinte des pièces remises est revérifiée au
moment de la décision.

**Validation.** Chaque exigence porte des contrôles structurés : programme
pris dans une liste blanche, arguments, répertoire, délai, critères couverts
et justification. Ils sont exécutés sans interpréteur de commandes, depuis la
racine du projet (invariant I7). Une revue indépendante par le fournisseur du
responsable s'ajoute aux contrôles. Une tâche n'est acceptée que si ses
preuves portent sur la tentative courante et sur le contenu actuel du
livrable ; cette fraîcheur est réévaluée avant de lancer une tâche
dépendante (invariant I3).

**Conduite durable.** Un seul processus conduit une mission à la fois : la
possession est un bail renouvelé, acquis par comparaison et échange, et
considéré comme périmé après 30 secondes sans renouvellement (invariant I4).

### 5.2 Le banc de facturation

Le banc est écrit en Python, bibliothèque standard uniquement, dans
`benchmarks/billing/` du même dépôt. Il comprend :

- un grand livre SQLite en partie double, dont la mesure lit un instantané
  cohérent et vérifie les invariants comptables ;
- une API de paiement simulée, seul écrivain du grand livre, qui déduplique
  par clé d'idempotence, refuse une clé réutilisée avec un contenu différent,
  et sait injecter une réponse perdue après paiement ou une indisponibilité
  persistante ;
- un préparateur scripté, un contrôle de lot et un règlement déterministe
  dont la clé d'idempotence est configurable (aucune, par tentative, métier)
  et dont la progression est liée à l'empreinte du lot ;
- les trois conditions B0, B1 et S, un harnais qui isole chaque exécution
  dans une racine temporaire, et un enregistrement de provenance (commit,
  empreintes du code du banc et du binaire Swarm).

En S, la mission comprend un responsable scripté et déterministe, une tâche
`prepare` dont le livrable est le lot, et une tâche `settle` qui ne règle que
la remise de la tentative acceptée de `prepare`, après avoir comparé son
empreinte à celle enregistrée par le moteur (propriété E3). Les données de la
banque et le jeton de paiement sont placés hors de l'espace de travail des
agents, et seul le fournisseur du règlement reçoit le chemin du jeton
(propriété E1). Cette séparation repose sur l'emplacement des fichiers et les
arguments transmis, non sur un cloisonnement du système de fichiers : un agent
réel pourrait lire le répertoire de la banque, et l'isoler exigerait un bac à
sable.

### 5.3 Ce que l'implémentation a révélé

La construction du banc a mis au jour quatre comportements du moteur que la
conception n'anticipait pas. Nous les rapportons parce qu'ils conditionnent
l'interprétation des mesures.

1. **Espace de travail propre.** Lorsqu'une tâche s'exécute dans son propre
   sous-répertoire, la remise lit le livrable dans cet espace mais la reprise
   de la validation le cherche à la racine du projet ; la tâche n'est jamais
   acceptée. Le harnais de référence du moteur n'y échappe qu'en écrivant le
   livrable aux deux endroits. Le banc utilise un espace partagé.
2. **Refus silencieux.** Une tâche dont la dépendance n'est plus fraîche n'est
   pas lancée, sans motif journalisé ni événement adressé au responsable ; la
   dépendance reste affichée comme acceptée.
3. **Indisponibilité traitée comme un échec métier.** Un contrôle qui échoue
   parce qu'un service est indisponible déclenche la correction automatique
   de la tâche, donc la régénération du livrable, au lieu d'une attente de
   rétablissement.
4. **Absence de reprise après arrêt brutal.** Un agent tué sans diagnostic
   bloque la tâche ; aucune reprise automatique n'a lieu, la décision revient
   au responsable.

Les points 1 à 3 sont des écarts entre le comportement observé et le modèle
de la section 4 ; le point 3 concerne directement l'invariant I6. [À AJUSTER :
statut de ces écarts auprès des mainteneurs du moteur au moment de la
soumission.]

---

## Points à vérifier

1. Recalculer les tailles (lignes, fichiers, tests) au commit figé.
2. Vérifier la liste des fournisseurs pris en charge dans la configuration du
   moteur avant de les nommer.
3. Point 3 de 5.3 : formuler l'écart avec I6 après la campagne, sur plus d'une
   exécution.
