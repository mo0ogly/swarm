# Cohérence de la frontière durable avant restauration

16 octobre 2026, reprise du projet après la parenthèse candidature. Codex seul,
méthodes APEX et `replan`. MEM-U08, MEM-U12, MEM-U13 ; MEM-C15/MEM-C16.

## Observation et décision

M12 fournit un diagnostic, sans droit de restauration. En examinant les
préconditions d'une décision fraîche, le lecteur commun ne compare pas la
session du checkpoint avec la session du tour répondu. Le wrapper réécrit
également la session sans vérifier que la valeur de départ est inchangée.
Ces constats de code doivent être reproduits avant correction. Un chemin de
lecture supposait encore un nom de fichier absent (`prepared_launch.go`) ;
la recherche a été corrigée vers les producteurs existants, sans relance ni
mutation de produit.

Deux voies : ajouter des exceptions dans le diagnostic M12, ou renforcer le
contrat commun de lecture et de confirmation du tour. La seconde est retenue :
elle couvre aussi `agent show`, `agent-detail` et les futurs restaurateurs.
Un état de fin écrit avant l'accord du wrapper reste un état transitoire valide,
mais jamais une frontière confirmée tant que le statut n'est pas `answered`.

| Lot | Propriétaire et périmètre | Critère observable |
| --- | --- | --- |
| M13.1 | Codex, contre-exemples et préimages | Session de ligne divergente, statut inconnu, session non reprise en charge et usages invalides refusés ; échecs initiaux conservés. |
| M13.2 | Codex, lecteur/confirmation communs | Une ligne répondue porte la même session que son checkpoint ; la confirmation refuse un remplacement concurrent ; état transitoire et historique absent restent qualifiés. |
| M13.3 | Codex, vérification publique | CLI/API refusent sans mutation ; test wrapper/superviseur avec scripts locaux ; contrôles ciblés, concurrence et parcours navigateur existant. |
| M13.4 | Codex, documentation et RETEX | Contrat FR/EN, preuves liées aux sources, plan et limites mis à jour. |

Pas de nouveau plafond, de migration, de délégation ni d'appel de modèle.
Le serveur utilisateur reste en place. La décision d'exécution et la restauration
fournisseur demeurent ouvertes ; cette correction ne les simule pas.

## Résultat local

M13.1 à M13.4 sont réalisés et vérifiés : contre-exemples conservés, quinze
tests Go distincts dont six avec `-race`, compilation, analyse statique, contrôles
de configuration et de traduction, vraie recette navigateur FR/EN avec seize
captures. Documentation et RETEX :
[RESULTATS-MEMOIRE-09.md](RESULTATS-MEMOIRE-09.md). Aucun résultat n'est attribué
au serveur utilisateur conservé ; la restauration après arrêt reste ouverte.
