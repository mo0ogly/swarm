# Livraison candidate — graphe et automatisation

État au 6 octobre 2026 : **candidate à qualifier, non publiée**. Cette note décrit le contenu vérifié dans la copie D01–D04 ; elle ne constitue ni une release, ni une acceptation moteur, ni la preuve que le serveur partagé exécute ce candidat.

## Contenu du candidat

- préparation et édition de graphe avec aperçu, détection des cycles, révisions et conflits ;
- journal d’agents et restitution des preuves liées au graphe ;
- programmes d’automatisation, demandes idempotentes, planification, pause et reprise ;
- migration/archives et programmes importés désactivés ;
- diagnostic de version séparant CLI installée, serveur actif et sources candidates ;
- documentation d’installation, sauvegarde et rollback en français et en anglais.

Les parcours D01/D02 utilisent des racines et navigateurs isolés sans autonomie fournisseur réelle. Le coût fournisseur reste inconnu.

## Installation et version vérifiées

Le test D04 appelle l’installateur natif officiel dans un préfixe temporaire contenant des espaces. Il vérifie le mode `0755`, l’absence de démarrage, puis l’identité stable renvoyée par :

```sh
swarm --json version
```

Il vérifie aussi les actifs web embarqués, leur ETag fondé sur le contenu et la revalidation `304`, afin qu’un remplacement de binaire ne conserve pas silencieusement un point d’entrée JavaScript périmé. Ce contrôle isolé ne remplace pas la comparaison, après supervision autorisée, des trois identités CLI/serveur/candidat.

## Mise à jour et rollback

Avant toute mise à jour réelle : attendre l’absence d’agent et de contrôle actifs, arrêter le serveur, sauvegarder ensemble `.swarm/`, le dossier des agents et le binaire correspondant, puis contrôler leurs empreintes. Ne jamais utiliser un ancien binaire sur des données déjà migrées ; restaurer le couple sauvegarde/binaire compatible.

La recette D04 vérifie qu’une mise à jour aux métadonnées invalides échoue sans remplacer le binaire installé. Elle vérifie ensuite un passage ancien→nouveau, puis la restauration atomique du binaire sauvegardé avec SHA-256 identique et sans altérer un état sentinelle hors cible. Les commandes opérationnelles complètes restent dans [INSTALL.md](../INSTALL.md).

## Autorisation et limites de livraison

Aucun commit, push, tag, paquet, image publique, release ou redémarrage du serveur partagé n’est autorisé par cette note. La supervision ne peut installer le candidat qu’après :

1. contrôle D04 public sur les inputs finaux ;
2. revue indépendante du même candidat ;
3. acceptation D03 puis D04 enregistrée par le moteur ;
4. absence constatée d’agents et de contrôles actifs ;
5. sauvegarde et plan de rollback prêts ;
6. autorisation explicite de l’installation du serveur.

Après installation, relire la version CLI et la santé/version du serveur, puis effectuer le parcours de navigation ciblé. Toute identité divergente, actif périmé ou régression du graphe impose l’arrêt et le rollback documenté.

## État des preuves

- D01 et D02 : contrôles/revues historiques rapportés comme acceptés, avec limites fixture explicites.
- D03 : reçu hôte présent (suite Go découverte une fois, race, vet, npm et gardes à code 0), mais la preuve locale consultée ne démontre pas encore une revue/acceptation postérieure à ce reçu.
- D04 : contrôles worker installation/version/actifs/rollback à code 0 ; contrôle public, revue indépendante, installation supervisée et clôture moteur encore attendus.

Voir [le rapport D04](D04.md) et [son dossier](D04-dossier.md).
