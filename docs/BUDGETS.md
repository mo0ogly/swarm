# Budgets et coûts IA

## Régler l’enveloppe d’une mission

Dans **Budgets et coûts IA**, ouvrir **Régler le budget**, saisir le plafond USD,
la réservation estimée par départ, la source et la date de référence.
**Prévisualiser le budget** montre ancien et nouveau plafond et engagements
conservés. **Enregistrer ce budget** applique la modification. Modifier un champ
invalide la prévisualisation. Une page périmée est refusée : fermer le formulaire,
actualiser, puis examiner à nouveau les valeurs.

Un plafond de zéro désactive le contrôle financier estimatif. Une baisse ne tue
pas les agents actifs et n’efface pas les dépenses estimées ni les réservations.
Les futurs départs restent soumis aux autres contrôles du moteur.

## Même contrôle depuis le CLI

```sh
swarm budget show WORK --json
swarm budget preview WORK --input budget.json --json
swarm budget apply WORK --input budget.json --json
```

Exemple de `budget.json` (montants fictifs, à remplacer) :

```json
{
  "schema_version": 1,
  "event_id": "budget-authorization-001",
  "expected_revision": 12,
  "budget": {
    "limit_usd": 10,
    "reserve_per_launch_usd": 2,
    "estimate_source": "Estimation locale documentée",
    "reference_date": "2026-09-23"
  }
}
```

Utiliser la révision renvoyée par `show`. `preview` est une lecture indicative ;
les réservations peuvent évoluer ensuite. `apply` vérifie la révision dans la
transaction qui enregistre le réglage et son événement. Deux autorisations
concurrentes sur la même révision ne peuvent pas se remplacer silencieusement.
Rejouer le même événement avec la même demande ne consomme pas de révision.
L’auteur et la date d’enregistrement sont déterminés par le moteur.

## Ce qui est réellement couvert

L’enveloppe estimative couvre exécution, planification, assistance de page et
préparation. **Les revues indépendantes ont un plafond d’appels séparé ; leurs
coûts monétaires ne sont pas inclus dans cette enveloppe.** Les coûts rapportés
par les fournisseurs sont distincts des estimations ; absence de coût ne veut
pas dire zéro. Ce contrôle n’est pas une garantie de plafond de facturation.

Cette livraison n’ajoute pas encore de catalogue de tarifs par modèle, de
politique par défaut pour les nouvelles missions ni d’éditeur unifié des quotas
de planification/revue/tentatives. Les autorisations existantes restent inchangées.
