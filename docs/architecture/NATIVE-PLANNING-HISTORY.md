# Historique natif des décisions de planification

[English](../en/NATIVE-PLANNING-HISTORY.md). Lecture native installée et vérifiée en215 ; correction de deux traductions
en cours de qualification. Recette et installation restent dans le [plan de migration](../plans/migration-rust/PLAN.md).

GET/HEAD /api/v1/planning-history?work=WORK et CLI planning history WORK
renvoient le tableau Go des décisions retenues, triées par révision de l’événement.
Chaque élément contient id, at, scope, reason et operations. Le corps et les
paramètres supplémentaires sont refusés. Le lecteur natif exige un travail
existant ; un travail inconnu est404, alors que le Go renvoie un tableau vide.

Le lecteur prend la mission et ses événements dans un instantané SQL, sous
réservation partagée des réglages et contrôle du descripteur de la racine.
Les événements autres que planning.decide restent hors de cette projection.
Une charge utile invalide refuse le tableau entier ; aucun succès partiel vide.
Les erreurs SQL, décodage, finalisation, rollback, fermeture et revalidation
physique conservent leur cause sûre.

Le décodeur lit le contrat Go complet de PlanningRequest, en autorisant les
champs inconnus comme json.Unmarshal. Il conserve les types connus, doublons,
casse, null, tableaux vides et encodage spécial des dépendances. Il ne valide
pas cette ancienne donnée comme une nouvelle commande : aucun schéma courant,
bail, budget ou droit n’est adopté. Seuls les cinq champs publics sont exposés ;
la requête complète, ses gardes et ses champs privés ne sont pas renvoyés.

La vue du cockpit lit sur action explicite, rend le texte sans HTML et conserve
les lignes précédentes après un refus. Les compteurs entiers historiques restent
exacts, même au-delà de la précision JavaScript. Les opérations ne sont jamais
exécutées ; leurs plafonds déclarés ne deviennent pas la configuration actuelle.

Aucun plafond de lignes, délai, rétention, relance ou nouvelle politique opérateur
n’est introduit. Ce parcours lit tout l’historique retenu, avec mémoire
proportionnelle à son contenu ; les limites réseau existantes restent applicables.
Un tableau vide signifie qu’aucune décision ne figure dans cette observation,
sans prouver la complétude de l’histoire, l’identité d’un planificateur ou
l’application des opérations. Les fixtures injectées sont explicitement des
données synthétiques non appliquées, jamais une acceptation fabriquée.

~~~mermaid
sequenceDiagram
    actor U as Utilisateur
    participant UI as Historique à déplier
    participant H as API protégée
    participant R as Root lié
    participant SQL as Instantané SQL
    U->>UI: Lire les décisions
    UI->>H: GET planning-history avec Work
    H->>R: Contrôles méthode, paramètres et liaison
    R->>SQL: Lire Work et événements
    SQL-->>R: Données retenues
    R->>R: Décoder les décisions, sans les exécuter
    alt Donnée illisible
        R-->>H: Cause conservée et rollback contrôlé
        H-->>UI: Refus, lignes précédentes conservées
    else Lecture valide
        R-->>H: Tableau public, fermeture contrôlée
        H-->>UI: Décisions enregistrées et entiers exacts
    end
~~~
