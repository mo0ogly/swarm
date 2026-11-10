# Reprise prioritaire du portail — prochain incrément proposé

État du 24 octobre 2026 : le lanceur local Rust `v0.1.1-dev.7` est actif et vérifié. Les fondations F1–F5 étaient déjà acceptées dans l’état historique ; leurs dix fichiers Markdown/JSON sont maintenant récupérés **octet pour octet** depuis le candidat `643c8a8686f8b1811b83c1e5c6d252c8c1ca694c`, lié au reçu original vérifié. Cette récupération ne constitue pas une nouvelle acceptation ni une adoption des décisions réservées à l’utilisateur. Source : `docs/plans/migration-rust/evidence/20261024-native-launcher1411/portal-foundations1434.json`.

L’utilisateur confirme la priorité du portail et la reprise avec GLM/SKYNET. La pause des nouveaux départs historiques est maintenue : le bail ancien n’est pas migré et aucun compteur ou budget n’est remis à zéro. La contribution GLM est une proposition sans outils, examinée par Codex ; les faits d’exécution sont ceux des preuves locales.

## Décision en attente

| Option | Premier résultat visible | Conséquence |
| --- | --- | --- |
| A, recommandée | Service Rust indépendant servant découverte et catalogue FR/EN en lecture seule | Prépare la frontière de déploiement future ; comptes et base ne sont pas ajoutés maintenant. |
| B | Vitrine statique FR/EN, backend différé | Moins de service initial ; l’entrée de livraison dynamique sera ajoutée ensuite. |

Question envoyée à l’utilisateur : choix A ou B avant scaffold/réalisation. Le choix ne confirme ni hébergement externe ni décisions de comptes/sessions. La proposition GLM confondait le lanceur du moteur et un service portail indépendant : cette confusion est rejetée ici. Le portail ne sera pas ajouté aux routes authentifiées du serveur local par simple commodité.

```mermaid
flowchart LR
    P[Visiteur FR/EN] --> W[Portail indépendant : découverte et catalogue]
    W --> D[Documentation de démarrage]
    D --> H[Installation locale décidée par l’utilisateur]
    H --> L[Swarm Rust local : préparation puis consentement]
```

Il n’existe pas dans ce premier incrément de canal distant vers le Root, de reflet de missions, de transfert de clé locale ni de lancement d’agent depuis une fiche de modèle.

## Parcours vertical à adopter

1. **s0-decouverte** : expliquer moteur déterministe/agents et montrer un graphe explicitement illustratif, avec limites et exemple de preuve ; accéder au catalogue.
2. **s0-catalogue** : présenter des modèles réellement identifiés et leurs limites, sans inventer un modèle exécutable, un résultat obtenu ou une autorisation. Le choix mène à un guide, jamais à un POST d’exécution moteur.
3. **Livraison locale** : construction indépendante, adresse locale d’aperçu explicite, test FR/EN et navigation au clavier, ressources locales sans CDN ; installation de ce seul service après vérification selon la préférence utilisateur. Aucun hébergement externe implicite.

Fichiers proposés, à vérifier contre les producteurs lors de la recherche avant adoption : manifeste/crate portail indépendant pour A ; ressources propres au portail et catalogue public versionné ; recette permanente de parcours ; docs FR/EN. Le service ne dépendra pas de `swarm-engine` ni des données `.swarm`. Réutiliser les jetons et motifs visuels documentés ; préserver la construction et le déploiement indépendants des interfaces existantes. Les chemins finaux seront annoncés avant écriture du code.

## Critères observables

- Deux parcours FR/EN complets, liens de retour et guide ; absence de texte prétendant une fonctionnalité non livrée.
- Clavier, focus visible, thèmes et largeurs mobile/bureau ; aucune information accessible seulement par survol.
- Chargement/vide/erreur/succès quand une source de données peut produire ces états ; ne pas inventer des spinners pour des données statiques.
- Ressources embarquées/locales servies identiques aux entrées réelles du build ; requêtes et console contrôlées.
- Catalogue avec provenance réelle, sélection sans mutation de Root, absence de clé locale ou d’appel de lancement.
- Échec de construction/précondition conserve l’instance précédente et la preuve ; limites de recette justifiées, pas de watchdog global hérité.

## Décisions suivantes, avant les comptes

F4 propose authentification locale, droits client/admin avec propriété des ressources, stockage relationnel, audit transactionnel et sessions révocables. Ce sont des propositions à confirmer. Les « 24h » ne seront pas une constante cachée : source persistée, validation serveur et interface Admin, avec valeurs initiales explicites et conséquences. Aucun plafond minimal/maximal opérateur inventé. Le choix SQL/hébergement reste à documenter. F5 signalait déjà le champ de préférence de thème manquant : c’est une décision connue à résoudre, pas une découverte nouvelle ni un champ déjà implémenté.

Reprendre les cas de deux comptes, anonyme, URL directe/devinée, expiration et révocation effective dans le parcours de comptes futur. Aucun appel GLM, avis ou état historique accepté ne remplace les tests ni l’acceptation de ce nouveau produit.

Propriétaire : Codex, appui GLM interne autorisé. État : proposition prête à décision ; réalisation non commencée tant que le choix A/B et l’adoption de cette tranche sont en attente.
