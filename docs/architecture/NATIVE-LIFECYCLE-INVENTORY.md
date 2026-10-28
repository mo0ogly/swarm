# Inventaire natif du cycle de vie

Lecture candidate GET/HEAD /api/v1/lifecycle/missions et CLI lifecycle list.
Le tableau ordonne les Works par rowid décroissant, actifs ou archivés, puis
les métadonnées de corbeille par date de suppression décroissante. Chaque ligne
expose work_id, title, revision, state, state_label, et changed_at si non vide.
Une archive avec date vide reste archivée. Une ligne de corbeille ne prouve ni
fichier valide, ni restaurabilité, ni autorisation. Aucun snapshot privé exposé.

Un instantané SQL, une réservation partagée des réglages et le contrôle du Root
physique lient l’observation. Tout Work malformé ou échec SQL/fermeture refuse
la lecture entière ; pas de liste vide de secours. HTTP refuse corps et paramètres
supplémentaires. CLI et HTTP utilisent le même lecteur. Aucune mission, archive,
corbeille, consommation ou visite n’est modifiée. Aucun provider n’est lancé.

Le volet explicite FR/EN rend les textes sans HTML, les grandes révisions sans
arrondi et conserve les observations précédentes après refus. Pas de lecture
automatique, pagination cachée, relance ou plafond opérateur ajouté. La mémoire
croît avec les métadonnées retenues. État candidat ; preuves et livraison dans
le plan de migration, pas acceptation globale.

~~~mermaid
sequenceDiagram
    actor User
    participant UI as Explicit inventory
    participant HTTP as Protected API / CLI
    participant Root as Bound Root
    participant SQL as Read snapshot
    User->>UI: Read inventory
    UI->>HTTP: GET lifecycle/missions
    HTTP->>Root: Validate input and binding
    Root->>SQL: Works + archive metadata + trash metadata
    SQL-->>Root: Ordered observations
    Root-->>HTTP: Complete view or preserved error
    HTTP-->>UI: Literal rows and exact revisions
~~~
