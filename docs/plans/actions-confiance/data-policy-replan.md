# T4 — admission des données, reprise du 15 octobre 2026

Besoin R14 : contrôler classification, destination, fournisseur, outil et secret
pour chaque chemin réellement maîtrisé ; refuser un chemin non prouvé. Codex
reste propriétaire. R13 conserve son écart de révocation distante des services.

## Observation et décision

Les politiques de fournisseurs, le nettoyage de texte et les copies de travail
existent. Ils ne constituent pas un confinement système ni une attribution de
secret par action. Le parcours `trustOperate` engage uniquement un registre
simulé dans le moteur ; il ne lance aucun binaire et n'ouvre aucune destination
réseau. Les originaux des commandes admises ou refusées métier sont conservés.
Une donnée non autorisée doit donc être refusée avant cette conservation.

Deux voies comparées : ajouter immédiatement un confinement système complet,
avec dépendances de plateforme et vérification des processus ; ou contrôler
d'abord l'admission du parcours simulé existant, puis construire le courtier de
secrets et les connecteurs. La seconde voie est choisie : bornée, réversible et
vérifiable sans privilèges système. Elle ne qualifie pas tout R14.

## Contrat de cette tranche

- `data_policy` : booléen `enabled` explicite et listes explicites non nulles
  `classifications`, `destinations`, `providers`, `tools`. Omission du bloc
  conserve la politique ; absence historique signifie désactivée.
- `data_context` d'une demande : `classification`, `destination`, `provider`,
  `tool`, tous explicites. Lié au digest exact, conservé dans l'action et relu
  lors de l'accord et du premier engagement. Aucune substitution sur ces étapes.
- Chemin prouvé de cette tranche : `local-ledger` / `swarm` /
  `simulated-settlement`. Aucun chemin de fichier, URL, socket ou binaire n'est
  un alias de cette destination. Une autorisation déclarée ne prouve pas un
  nouveau chemin. Les classifications `public` et `internal` sont prises en
  charge ; `confidential` et `restricted` restent refusées tant que le
  confinement des exécutables et l'accès aux secrets ne sont pas établis.
- Politique active : contexte obligatoire, chaque dimension doit appartenir à
  sa liste. Politique inactive : contexte explicite refusé ; ancien pilote sans
  contexte compatible et toujours non qualifié pour les données sensibles.
- Refus d'admission : erreur sans écriture d'original, de preuve ou d'état.
  Rejeux historiques et réconciliation d'effets engagés préservés.
- Admin FR/EN, aperçu sans mutation, sauvegarde/réouverture et même validation
  Go/API/CLI. Aucune valeur numérique ni plafond implicite.

## Ordre et preuves attendues

1. Contre-exemple public : bloc inconnu avant implémentation, original conservé.
2. Validation stricte, admission sans mutation, digest, accord/effet et reprise
   dans une racine isolée ; cas URL, traversée, lien, binaire, champ nul/inconnu.
3. Recette CLI/API puis navigateur FR/EN, thèmes, mobile et diagnostics.
4. Documentation courante et reçu lié aux entrées ; reprendre courtier de
   secrets puis révocation distante des services, sans déclarer R14 terminé.

Incident de méthode : une première recherche nommait des fichiers supposés
absents. La recherche globale des symboles a établi les chemins réels. Aucun
défaut produit n'en est déduit ; vérifier les producteurs avant réutilisation.

Justification des régressions : `TrustCommand`, le digest canonique et l'entrée
commune `trustOperate` changent ; exécuter T2 et T3 sous race en groupes bornés.
`configRequest`, l'ouverture d'Admin et le rendu partagé changent également :
réexécuter les recettes navigateur concernées sur le nouveau binaire. Les preuves
historiques demeurent conservées, sans leur attribuer cette nouvelle entrée.

## Réexamen de sécurité et incident de recette

La première tranche verte vérifie une classification déclarée par le demandeur.
Cela ne suffit pas à empêcher un déclassement de données connues de l'admin.
Ajouter `resources:[{resource,classification}]` à la politique locale versionnée :
correspondance exacte obligatoire, aucune classification choisie par l'agent ne
remplace celle de la ressource. Absence de règle et contradiction refusent avant
capture. Opaque identifiant métier, jamais résolution de chemin.

La recette publique commune a expiré après 25 secondes dans sa phase navigateur,
sans résultat final. Son appel subprocess.run interrompait le parent sans nettoyer
le serveur enfant ; PID1320896 a été identifié par argv/racine puis arrêté, reçu
public-timeout-fixture-cleanup.json. Conserver l'échec. Corriger l'arrêt du groupe
propriétaire avec délai de fermeture, retenir les artefacts via --evidence-dir,
vérifier le nettoyage avec un enfant isolé, puis reprendre avec budget adapté à
la durée mesurée de cette recette complète. Ce délai n'établit pas une panne
produit. Aucun PASS attribué à la tentative incomplète.
