# Replan — débloquer la mission conservée, 14 octobre 2026

Demande utilisateur : traiter le blocage visible. Source propre c8f1069, serveur
identique. Mission conservée r839 en pause, T2/T3 bloquées, T4–T8 en attente.
Les développements directs et le RETEX livré ne remplacent pas ses acceptations.
Les originaux publics sont dans le cache opérateur existant, préfixe
`replan-native-unblock-20261014`.

## Diagnostic et décision

1. `managedScopeFiles=100` refuse 112 fichiers T2 et 272 fichiers T3. Cette
   politique non réglable contredit le contrat Admin. Rendre la limite configurable
   est préféré à réduire arbitrairement les fichiers : aucun original omis.
   Une décomposition de revue reste possible, décidée par le responsable.
2. La lecture publique T2 qualifie à tort son manifeste valide de 28 sources
   d’invalide : un seuil fixe de 24 sources est mêlé au schéma. Ses 175190 octets
   dépassent aussi le plafond fixe128 Kio. Corriger les politiques ne supprime
   pas son ancien refus sémantique.
   T3 fournit un inventaire attribué, mais pas une reprise de revue interrompue.
3. Les tentatives sont consommées et restent telles quelles. Pas de reset,
   d'acceptation manuelle sans revue ni de relance inchangée. Réemployer les
   résultats existants avec réparation attribuée, nouveaux contrôles et revue,
   plutôt que redévelopper ce qui existe déjà dans la livraison directe.

## Séquence et critères

| Étape | Propriétaire | Effet vérifiable |
| --- | --- | --- |
| Politique de fichiers | Codex direct | Même limite effective persistée via quotas CLI/API et modale Admin ; 0 sans plafond, défaut ancien100 visible ; aucune seconde limite dans un lot ou un patch |
| Régression | Codex direct | Refus avant appel au défaut100, aperçu sans mutation, seuil supérieur100 et0, négatif/périmé refusés, réouverture identique ; consommateurs scope/recovery/coordination cohérents |
| Livraison | Codex et revue indépendante déjà autorisée | Documentation FR/EN, navigateur FR/EN/thèmes, commit/push et version servie ; missions/historique conservés |
| Reprise des résultats | Opérateur puis propriétaire racine | Diagnostic du manifeste, correction générale/attribuée, décision publique reçue, contrôles/revue sur candidat exact, effet observé ; aucun Claude pour développer le moteur |
| Clôture | Superviseur | Fin uniquement après preuves fraîches des exigences ; distinguer levée d'un verrou et mission acceptée |

Les budgets/droits peuvent être augmentés dans l'autorisation existante après
shortfall constaté, avec valeurs persistées et consommation inchangée. Le modèle
configuré et les limites fournisseur restent opposables. L'ancien refus et le
candidat original ne sont jamais modifiés pour fabriquer une réussite.

## Vérification et replan de la correction

La revue indépendante a détecté trois consommateurs résiduels : plafonds de lots
24/96/128, refus individuel non reprenable, revalidation des contextes historiques
sous le réglage actuel. Un rouge reproduit la rétro-invalidation (politique1
contre28 pièces figées). Correction : les nouveaux contextes appliquent la
politique persistée ; les transports/historiques vérifient attribution et intégrité,
sans réappliquer une politique ultérieure. Les refus individuels sont typés.
Le diagnostic connexe utilise aussi les réglages de sources.

Une erreur de recette comparait maps vides et nil : JSON persisté identique,
aucune mutation moteur. La recette compare maintenant deux lectures persistées.
Un processus de tests a été interrompu143 : preuve conservée, jamais PASS ;
la reprise ciblée du cas interrompu passe. Aucun nouveau lancement de suite entière.
