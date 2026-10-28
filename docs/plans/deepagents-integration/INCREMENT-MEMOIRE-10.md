# M14 — reprise explicite d'une conversation arrêtée

16 octobre 2026, autorisation « go ». Codex seul, APEX et `replan`.
MEM-U08/MEM-U12/MEM-U13, cas MEM-C15/MEM-C16. Serveur utilisateur conservé.

## Contrat retenu

Réouvrir la même ligne d'agent demanderait de remplacer son état terminal et
sa réservation déjà réglée. Cela compliquerait l'histoire et le coût. Le choix
retenu est une **nouvelle tentative d'exécution**, avec identité, réservation
et événements propres, qui reprend explicitement la **même conversation
fournisseur** à sa dernière frontière confirmée. Les plafonds de tentatives
existants s'appliquent ; aucune tentative consommée n'est remboursée.

L'opération reçoit révision, événement idempotent, checkpoint et faits examinés,
confirmation, motif et nouvelle consigne. Le lancement normal contrôle à nouveau
autorisations, dépendances, espace, plafonds, précontrôle et réservation. Une
preuve de capacités fournisseur vérifiée est nécessaire ; une ancienne sonde
compatible non vérifiée ne suffit pas. Le premier résultat doit confirmer la
même session ; un refus ou une session différente ne devient jamais une reprise
réussie et ne déclenche pas de relance automatique.

Les gardes d'outils et le nombre d'échanges restent cumulatifs dans le checkpoint.
Les métriques et usages publics de l'agent comptent seulement sa nouvelle
exécution, afin de ne pas additionner deux fois l'histoire dans les totaux.
Le seuil de durée reste celui du lancement, selon son contrat d'observation
existant ; les durées des tentatives précédentes restent conservées.

La première version conserve exactement le fournisseur, le modèle, l'espace et
les limites examinés. Une copie Git gérée qui imposerait un nouvel espace doit
passer par la reprise de production existante, et non réutiliser cette session.
Le refus reste explicite. Aucun nouvel appel de modèle réel n'est autorisé ici.

Le stockage passe à 35, avec sauvegarde privée, pour empêcher un ancien programme
de lire une intention de restauration comme un lancement neuf. Aucun stockage
de mission utilisateur n'est migré pendant ce développement.

Replan de vérification : le binaire provisoire reproduit une migration implicite
sur quatre lectures d'un stockage 34 (`mission recovery`, `changes`, `spending`
et `agent resume-dialogue-preview`). Le classement commun d'inspection doit les
refuser avant toute migration. Les quatre originaux restent dans
`evidence/m14-20261016/preview-storage-red-results.json`. Le correctif appartient
au contrat partagé d'ouverture du stockage, avec garde de régression CLI native.

| Lot | Responsable et dépendance | Critère observable |
| --- | --- | --- |
| M14.1 | Codex, contrats et contre-exemple | Opération absente reproduite ; plan, préimages et sources conservés. |
| M14.2 | Codex, lancement et runtime | Décision liée aux faits, refus atomiques, garde restaurée, même session, usages séparés, garde de version et concurrence. |
| M14.3 | Codex, CLI/API/UI | Même opération authentifiée, confirmation explicite, erreurs conservant la saisie, suivi du nouvel agent ; FR/EN, thèmes, clavier et mobile. |
| M14.4 | Codex, vérification et RETEX | Tours avant/après arrêt, deux adaptateurs scriptés, limites et coûts, parcours public compilé, documentation FR/EN et preuves attribuées. |

Une vérification locale n'est ni une qualification fournisseur réel, ni une
revue indépendante, ni un déploiement. Les autres usages mémoire restent suivis.

## Résultat local

M14.1 à M14.4 sont réalisés et vérifiés dans leur périmètre local : 42 tests Go
distincts, dont huit avec détection de concurrence, quatre refus CLI natifs de
migration implicite et neuf contrôles du navigateur. Les reprises demandées en
FR/EN confirment la même session ; la garde cumulée arrête la quatrième invocation
scriptée. Aucun modèle réel n'a été appelé. Les défauts de recette et le défaut
de classement des inspections gardent leurs échecs originaux.

Lire [RESULTATS-MEMOIRE-10.md](RESULTATS-MEMOIRE-10.md) pour la matrice,
l'attribution des sources, la documentation FR/EN, le RETEX et les limites.
MEM-C15/MEM-C16 restent PARTIAL ; aucune acceptation globale ou migration du
serveur utilisateur n'est déclarée.
