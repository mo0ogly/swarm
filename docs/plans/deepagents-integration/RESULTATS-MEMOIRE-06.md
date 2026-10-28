# Dialogue : une fin confirmée avant le statut répondu

16 octobre 2026, reprise après « next », Codex seul. Les quatre tâches de
[l’incrément 6](INCREMENT-MEMOIRE-06.md) sont réalisées et vérifiées localement.
Le défaut de frontière de tour est corrigé dans le candidat ; aucune conversation
réelle, mission vivante ou installation existante n’a été modifiée. Aucun appel
de modèle ni déploiement n’a été effectué.

## Défaut reproduit et cause

Trois sous-tests échouaient avec le vrai wrapper en sous-processus et un
fournisseur local terminant avec un code nul : marqueur de fin absent,
`turn.failed`, et appel d’outil sans résultat. Dans les trois cas, le wrapper
retournait sans erreur et la table contenait **un tour `answered`**. Le moniteur
signalait pourtant déjà le résultat d’outil manquant dans le troisième cas.
Les originaux sont dans
`evidence/m10-20261016/dialogue-receipt-red-short-root.json/.log`.

`runAgentDialogue` écrivait `answered` avant d’envoyer la fin au moniteur.
La réussite du processus était confondue avec une fin de réponse qualifiée.
Le canal fd4 autorisait le début d’un tour, sans confirmer sa fin.

## Correction et parcours vérifié

Le même canal confirme désormais les deux frontières. Le moniteur exige une
fin fournisseur positive unique, une identité stable, l’absence de refus,
d’activité non prise en charge, de résultat manquant et de limite déclenchée.
La voie Codex attend `turn.completed` ; Claude attend un `result` de sous-type
`success`, sans erreur. Les événements après fin positive sont refusés.
Les erreurs fournisseur réutilisent le classificateur existant, qui ne recopie
pas leur texte privé dans le motif public.

Les journaux sont enregistrés avant l’accord de fin. Ensuite seulement, le
wrapper peut mettre à jour son propre tour encore `running`, avec contrôle du
nombre de lignes modifiées. Une panne de journal ou de statut bloque l’échange
suivant. Un canal fermé avant confirmation conserve l’interruption et l’usage
déjà reçu. Les identifiants de tour rejoués sont refusés.

Les tests de wrapper utilisent les vrais canaux fd3/fd4 et des exécutables locaux
nommés Codex ou Claude. Sur deux tours confirmés, ils vérifient la même session,
les arguments de reprise, les permissions configurées, un prompt initial reçu
une seule fois et un second message distinct. Un identifiant d’outil réutilisé
reste distingué par tour. Les compteurs conservés après réouverture sont de
**deux appels, deux résultats et 20/4 tokens d’entrée/sortie**. Ces tokens sont
des valeurs de fixture, sans consommation de modèle.

Le test `TestDialogueActualSupervisorConfirmsTurnReceipts` construit le vrai
`cmd/swarm`, démarre son superviseur, utilise le terminal avec bail et numéros de
séquence, puis relit l’agent par `swarm --json agent show`. Ses trois reçus sont
dans `evidence/m10-20261016/actual-supervisor/` :

| Scénario | État de l’agent | Tours répondu/interrompu | Invocations locales | Effet observé |
| --- | --- | --- | --- | --- |
| Deux tours confirmés, puis `/fin` | `completed` | 2 / 0 | 2 | Deux appels et résultats, usage 20/4, tâche non acceptée. |
| Refus fournisseur avec sortie 0 | `interrupted` | 0 / 1 | 1 | Pas de faux `answered`, pas d’invocation suivante. |
| Résultat d’outil manquant | `interrupted` | 0 / 1 | 1 | Un appel, zéro résultat, un outil en attente, usage 10/2 conservé. |

Les pannes d’écriture sont injectées uniquement dans les bases de fixture.
Après échec, aucun tour n’est répondu ; la finalisation conserve un tour
interrompu et l’usage observé. Le redémarrage implicite du wrapper reste refusé,
sans nouvelle invocation ni modification de l’enregistrement précédent.

## Vérifications et liaison aux sources

| Contrôle | Résultat | Preuve sous `evidence/m10-20261016/` |
| --- | --- | --- |
| Trois faux `answered` avant correction | 3 FAIL effectivement exécutés | `dialogue-receipt-red-short-root.json/.log` |
| Wrapper, cumul, permissions, erreurs privées, terminal contradictoire, persistance et refus de redémarrage | PASS, 8 tests nommés et 16 sous-tests | `dialogue-receipt-green.json/.log` |
| Protocole : rejeu, EOF non confirmé, changement de session ; vrai superviseur/terminal/CLI | PASS, 2 tests nommés et 6 sous-tests | `dialogue-public.json/.log`, `actual-supervisor/` |
| Concurrence sur le moniteur et les wrappers locaux | PASS, 6 tests nommés et 15 sous-tests avec `-race` | `dialogue-race.json/.log` |
| Analyse statique du moteur | PASS | `engine-vet.json/.log` |
| Compilation du vrai binaire | PASS dans le test du superviseur | `dialogue-public.json/.log` |
| Redémarrage d’une conversation interrompue, raisonnement d’un modèle réel, serveur installé | NOT TESTED | Aucune revendication de restauration ou de déploiement. |

Il y a **10 noms de test distincts**. Les six tests `-race` recoupent les deux
lots verts ; ils ne sont pas additionnés comme des scénarios indépendants.
Le seul ajout entre ces lots est le fichier du test public ; aucune source
préexistante du premier lot n’a changé. Les 1 004 entrées Go et ressources
embarquées du dernier manifeste restent identiques pendant les contrôles et
après la documentation. Les reçus conservent commandes, dates, noms exécutés,
empreintes des journaux et éventuelle dérive. `final-audit.json` relie les
contrôles finaux aux fichiers du candidat et aux preuves.

La revue locale compare aux préimages de cette séance, préservant le travail
antérieur. Elle suit wrapper → moniteur → persistance → statut → consommateur
CLI, ainsi que la protection du modèle observé par le verrou du puits de sortie.
Elle reste une revue dans le contexte d’implémentation, sans approbation
indépendante. Aucun rendu web n’a changé ; aucune recette navigateur n’est
revendiquée. Les documents [FR](../../PROJECT-MEMORY.md) et
[EN](../../en/PROJECT-MEMORY.md) décrivent le même contrat.

## RETEX et incidents de méthode

Le premier essai de fixture échouait sur « Chemin du dialogue trop long » :
`t.TempDir` ajoutait les noms longs des sous-tests au chemin du socket. Ce n’était
pas la reproduction recherchée. Le journal `dialogue-receipt-red.json/.log` est
conservé. Une racine temporaire courte, possédée et nettoyée par le test, a levé
cette précondition ; la limite de transport du produit n’a pas changé. Les trois
échecs de comportement ont ensuite été obtenus avant la correction du moteur.
La source exacte du reproducer est conservée dans `preimages/` et reliée à son
empreinte originale.

Deux lectures reposaient encore sur un chemin supposé : la recherche de
`providerUsage` dans un fichier absent, puis la lecture d’une préimage sous un
nom non utilisé par son producteur. Les symboles et fichiers réels ont ensuite
été inventoriés. Ces erreurs d’exploration n’ont produit ni correction du produit
ni preuve de test ; leur prévention demande d’appliquer la règle existante de
recherche des chemins réels, sans ajouter une consigne doublon.

Leçon réutilisable : une sortie de processus réussie ne qualifie pas une frontière
de mémoire. Le composant qui observe les résultats et les gardes confirme la fin
avant son inscription ; une erreur de stockage interdit la continuation. Cette
règle est appliquée par le protocole commun, les producteurs et consommateurs,
les tests de panne et la documentation bilingue. Elle ne dépend d’aucune mission
ou formulation particulière d’erreur et ne crée aucun droit supplémentaire.

## Limites et suite du besoin mémoire

`answered` décrit une fin technique confirmée. Ce statut ne garantit ni la
justesse de la réponse ni l’acceptation métier. Il ne constitue pas un checkpoint
complet des gardes. Les usages non fournis restent inconnus ; l’agrégat durable
de l’agent est conservé par la supervision, sans nouvelle promesse de transaction
atomique avec chaque tour.

Une restauration après arrêt doit encore reconstruire les gardes cumulées,
les outils en attente et l’identité autorisée, puis faire vérifier les effets
incertains avant tout rejeu. Le refus existant reste donc nécessaire. Ce lot
ferme le défaut confirmé qui précédait cette restauration ; il ne l’implémente
pas implicitement. Le responsable de sa prochaine définition est Codex, avec
choix produit utilisateur si les droits ou le comportement proposé changent.
Résumé progressif, suppression et qualité LLM gardent leurs critères ouverts.
