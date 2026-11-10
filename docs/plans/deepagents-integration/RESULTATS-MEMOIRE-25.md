# M29 — lectures explicites, refus et propagation

17 octobre 2026. Demande « go » après le plan en cinq étapes. Codex seul dans
le checkout existant ; baseline `f32b9f8fa6e99f75845f57872a7f8a56a0ef9dc6`,
branche `codex/clear-product-method-names`, candidat non commité. Les sources
et documents du chantier identité sont préservés. Aucune délégation, requête
de modèle, mission native, activation de service ou mise à jour du serveur actif.

**Résultat : incrément local vérifié ; MEM-P1 reste en cours.** Les quatorze
sites de lecture de corps hors identité recensés sont raccordés au validateur
commun M28. La suite n'est pas une déclaration de conformité des 167 appelants
actuels, du SQL dynamique ou des dix droits publics M20.

## Ce qui change

Le contrôle commun exige clé SQL sûre, corps UTF-8/JSON de schéma 1, identité
concordante et révision positive égale à la colonne SQL. Il est réutilisé dans
les sources extractives, export, lancement transactionnel, politique/espace/reprise
causale d'automation, aperçu/application de graphe, inventaire du cycle de vie,
organisation, révocation des baux, conversion vers un travail existant et focus
de reprise. Le décodage est séparé du scan pour la jointure d'inventaire, sans
dupliquer les règles ni introduire une seconde lecture SQL.

Transactions, réservations, conflits de révision et contrôles métier sont
conservés. Créer un nouveau Work en mémoire à révision zéro reste valide ; cette
exception ne s'applique pas à une ligne persistée. Le rejeu exact du graphe garde
son reçu historique, distinct du rejeu de mutation qui retourne l'état courant.

Le sélecteur terminal n'affiche plus « aucune tâche » lorsque la lecture échoue.
Rendu et dialogue trient chacun les tâches du Work déjà lu ; le dialogue ne
recharge plus un autre snapshot pour sa portée et ses actions. Les opérations
automation `target` et `createAutomationSchedule` réservent `unknown_target` à
`sql.ErrNoRows`, sans masquer JSON invalide, incohérence ou panne SQL.

## Preuves avant et après

Répertoire : [`evidence/m29-20261017/`](evidence/m29-20261017/).

| Série | Observation | Limite |
| --- | --- | --- |
| `red` | 29 cas précis refusés par les assertions car l'ancien lecteur accepte l'incohérence ; 31 noms FAIL en comptant deux parents agrégateurs. Deux cas identité/révision de la source passent déjà. | Sources initiales et entrées stables ; aucune suppression de trigger. |
| `green` | Échec de compilation : import JSON devenu inutilisé ; zéro test collecté. | Preuve conservée, n'est pas un contre-exemple applicatif. |
| `green-current` | 34 noms PASS après suppression de l'import ; les refus, snapshots durables identiques et retours au cas valide passent. | Avant ajout des trois cas de propagation ; périmètre historique conservé. |
| `red-propagation` | Deux défauts reproduits : tâche vide dans le terminal, JSON invalide transformé en cible absente. Le parcours public déjà corrigé passe. | Deux FAIL, un PASS ; entrées stables. |
| `red-schedule` | Même traduction incorrecte reproduite pour la création d'un programme. | Un FAIL ; entrées stables. |
| `targeted` | 29 noms PASS : 18 parents, 11 sous-cas ; vet et build réussissent. | Les alternatives de regex sans suffixe ne sélectionnaient pas les familles de tests ; aucune couverture de ces familles déduite de cette série. |
| `boundaries` | Sélection corrigée explicite `TestWorkRow.*`, `TestMemoryWork.*`, `TestMemorySummarySource.*`, `TestGraphDraftB01.*`, `TestAutomationC01.*` : 107 noms PASS, 42 parents et 65 sous-cas. | Aucun FAIL/SKIP ; tous les huit nouveaux parents effectivement présents. |
| `race` | 61 noms PASS, 25 parents et 36 sous-cas : nouveaux refus, SQL/CAS/rejeu, concurrence du graphe/automation/lancement et quatre recettes terminal. | Aucun FAIL/SKIP ; double local de fournisseur lorsqu'un test le nécessite. |
| `verify_cli.py` | Binaire réellement compilé : export incohérent code 2 sans stdout/fichier, destination existante conservée, stockage inchangé, export/import valide. Quatre assertions, sept commandes. | Deux racines temporaires ; corruption de la seule colonne révision dans la fixture. Aucune réparation publique prétendue. |

Les séries finales normales totalisent **136 noms distincts : 60 parents et
76 sous-cas**. Avec race : **140 noms distincts, 64 parents et 76 sous-cas** ;
les séries se recouvrent et ne s'additionnent pas. Leurs reçus fixent commandes,
horaires, code de sortie et empreintes avant/après. Pas de watchdog global :
Go `-timeout=0`, subprocess sans timeout. Le contrôle race a progressé et s'est
terminé normalement ; aucun délai produit n'a changé.

Les nouveaux tests vérifient les données de toutes les tables des fixtures après
refus, pas seulement une révision ou un code HTTP. La recette publique Go traverse
la commande CLI en processus et les vrais handlers HTTP authentifiés : source
refusée sans paquet, inventaire sans liste partielle, export sans écrasement et
retour au cas valide. La recette Python vérifie en plus l'exécutable autonome.
L'import valide contrôle identité, révision et contenu attendu. Les bases réelles
et la version déjà servie par l'application ne sont jamais utilisées comme fixtures.

## Matrice de critères

| Critère | Résultat | Preuve / portée |
| --- | --- | --- |
| M29-01 | PASS pour l'inventaire défini ; PARTIAL pour MEM-P1 | 373 fichiers de production, 27 SELECT exacts avant, 14 après ; 168 puis 167 sélecteurs. [Matrice détaillée](evidence/m29-20261017/AUDIT-LECTURES.md), JSON et limites conservés. |
| M29-02 | PASS local | 29 cas initiaux + 3 cas de propagation = 32 contre-exemples précis ; sources et reçus rouges stables conservés. |
| M29-03 | PASS sur les refus exercés ; limite explicite lancement | Les treize sites autres que le lancement ont un cas invalide direct. Le lancement recontrôle désormais l'enveloppe sous sa réservation ; flux valides, idempotence et attente de rédacteur vérifiés, mais injection d'incohérence entre son get initial et sa transaction non testée directement. |
| M29-04 | PASS dans les fixtures concernées | Snapshots sans mutation, SQL/JSON conservés, destination préservée, cas valide après refus, migration/inspection, export/import et création/conversion existants. |
| M29-05 | PASS local ; aucune acceptation globale | Noms réellement exécutés, entrées stables, normal/race/vet/build, recette CLI réelle, auto-revue du diff et documentation FR/EN. |

L'inventaire est un scanner AST de sélecteurs et d'arguments SQL littéraux,
pas un parseur SQL ni un graphe dynamique typé. Une recherche complémentaire
repère le comptage `lifecycleMissionImpact`, la migration de projection identité
et les captures construites par `snapshotTable`. Ils ne sont pas ajoutés
silencieusement aux dénominateurs initiaux. La matrice conserve leur suite.

## Reprise de MEM-P1 et étapes suivantes

| Suite / origine | Valeur attendue et propriétaire | Critère et condition de reprise |
| --- | --- | --- |
| Boucles `missionOwnedCycle` / `missionCycleObserved` : sortie sur erreur list | Ne pas laisser une supervision silencieuse ; Codex mémoire/Store | Reproduire lecture invalide pendant le cycle, suivre diagnostic reçu, réconciliation sans effet puis reprise ; prochain lot P1. Aucun défaut runtime supplémentaire prétendu par la seule inspection. |
| Callbacks booléens de fraîcheur et `reviewFailure` | Conserver la cause du refus jusqu'au résultat ; Codex mémoire/Store | Injecter l'échec au callback, suivre annulation/publication/diagnostic ; matrice des chemins dans AUDIT-LECTURES. |
| Fallbacks et vues complémentaires | Distinguer une aide facultative d'un succès incomplet ; Codex mémoire/Store | Tracer chaque consommateur et son second contrôle, reproduire avant correction ; ne pas assimiler tout `err == nil` à une faille. |
| `snapshotTable`, `trashRestorePreview`, `applyRestore` | Couvrir les sources indirectes hors scanner ; Codex mémoire/Store | Lire producteur, schéma et validations de capture, puis variantes incompatibles et rollback. |
| Lancement entre sonde et transaction | Fermer la preuve spécifique de fraîcheur de l'enveloppe ; Codex mémoire/Store | Barrière déterministe dans une sonde locale, incohérence après get, refus avant agent/événement et reprise valide ; pas de délai arbitraire. |
| Lecteurs/protection identité | Préserver leur autorité propre ; chantier identité | Réutiliser leurs preuves actuelles sans modifier leurs sources ou inventer bootstrap/compte/confinement. |

Ces lignes font partie de la suite autorisée MEM-P1 ; elles ne sont ni effacées ni
clôturées par ce rapport. P2/P3 peuvent préparer leurs contrats indépendants,
mais aucune attribution réelle ou activation publique d'usage n'est réalisée ici.
P4/P5 gardent leurs dépendances d'identité et d'acceptation. Les dix critères M20
restent ouverts.

## RETEX de l'incrément

- **Cause commune vérifiée :** chaque lecteur direct avait reconstruit une partie
  du contrat de stockage. Certains acceptaient un schéma ou une ligne SQL/JSON
  discordante. Le noyau M28 évite de recopier ces validations ; la jointure garde
  un snapshot unique. La concordance ne prouve aucune autorité humaine.
- **Propagation :** refuser une ligne dans le noyau ne suffit pas si un appelant
  remplace sa cause par une absence. Les deux voies automation sont corrigées
  ensemble ; le dialogue terminal reçoit directement l'erreur. Les autres
  catégories restent une suite concrète avec propriétaire et preuve attendue.
- **Méthode :** une compilation échouée ne collecte aucun test. Une regex valide
  peut omettre une famille. Les recettes finales vérifient les noms attendus,
  pas seulement le statut du paquet ; les anciens reçus gardent leur périmètre.
  Deux patches atomiques ont été refusés pour un contexte documentaire inexact ;
  aucun changement partiel n'a été appliqué, le texte réel a été relu avant reprise.
  Les recherches de chemins absents imposent l'inventaire préalable : les noms
  de fonctions et chemins de la matrice sont recontrôlés dans la source.
- **Transmission :** contrat Work, mémoire et automation FR/EN actualisés dans le
  même incrément, plan et reçus persistants disponibles au prochain contexte.
  Pas de changement des contextes embarqués, de l'UI web ou des diagrammes README :
  leurs affirmations de mémoire/identité restent valables, aucune génération
  d'asset n'est requise. Le Mermaid du contrat Work est aligné dans les deux langues.
- **Limites :** auto-revue par l'auteur, pas de revue indépendante. HTTP en fixtures,
  terminal exercé par tests, aucun navigateur FR/EN/thèmes pour ce lot sans modification
  web. Aucune preuve de déploiement, de fournisseur réel ou de conformité générale
  de MEM-P1. Le besoin initial de mémoire avec propriété et droits reste au plan.
