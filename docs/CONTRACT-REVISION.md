# Réviser un contrat hiérarchique bloqué

Une erreur de séquencement peut demander une clôture avant l’acceptation qui la rend possible. La correction ne consiste pas à accepter artificiellement le résultat : l’opérateur révise explicitement le texte des critères, puis les contrôles et la revue indépendante examinent le nouveau contrat.

Commande publique : `swarm task update WORK --input revision.json`.

La requête contient `schema_version`, `event_id`, `expected_revision`, `id`, `criteria`, `confirm_contract_revision: true`, `expected_contract` (empreinte `contract` de la dernière revue) et `contract_revision_reason` (motif explicite de 16 à 2000 caractères). Chaque critère conserve sa place ; aucune suppression de critère n’est autorisée.

Préconditions : mission en pause publique, tâche bloquée, aucun agent ni revue actif, périmètre ouvert, empreinte et révision courantes. Les dépôts en intégration gérée ne sont pas encore couverts par cette opération.

La révision ne change ni identité, livrable, dépendances, propriétaire, exigences globales, historique, tentatives, budget ni délais fournisseur. Elle archive l’avis précédent et invalide les anciennes gates et preuves automatiques. La tâche reste bloquée jusqu’à une reprise publique du résultat existant, avec contrôles et revue frais. Le journal conserve la confirmation, l’empreinte antérieure et le motif ; rejouer le même événement ne modifie pas une seconde fois le travail.

L’ordre de finalisation reste : revue indépendante → acceptations fraîches → clôture publique enfant/parent → installation supervisée sous ses préconditions. L’opération ne crée aucune acceptation, aucune clôture ni autorisation de publication.
