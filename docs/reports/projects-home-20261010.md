# Accueil développement — 10 octobre 2026

## Résultat

Page autonome `web/projects.html`, identité Swarm, navigation horizontale,
intention guidée (construire/faire évoluer/corriger), préparation réelle,
modèles existants, formation et missions provenant de l’API publique.
Composition distincte de l’image de référence : aucun catalogue de cartes copié,
aucun texte ou asset tiers repris. Français/anglais et sombre/État.

Les liens de préparation préremplissent seulement un nouveau brouillon ; ils ne
sauvent ni ne lancent un agent. La protection des modifications non enregistrées
reste active. La visite du cockpit enregistre normalement l’historique de visite.
Les titres et objectifs saisis par les utilisateurs ne sont jamais traduits.

## Modèles

Décision utilisateur enregistrée dans PRODUCT-ARCHITECTURE.md : surcouche de
sujet sur la méthode commune. Cinq étapes pour le produit, puis un cycle de six
étapes **par fonctionnalité**. Le nouveau catalogue spécialisé et le suivi
persistant des étapes ne sont pas livrés par cet accueil. Les modèles existants
s’ouvrent dans la préparation. L’UX processus fonctionnels demeure un chantier
séparé ; Wattson est hors périmètre.

## Vérification

- Build canonique `sh build.sh /tmp/swarm-projects-home-20261010` : PASS.
- `node tests/projects_home_ui.cjs` sur binaire réel et racine isolée : PASS.
  FR/EN, thèmes via bouton réel, largeurs 1440/1024/760/390/320, radio au clavier,
  recherche, mission vide, erreur HTTP et reprise, préremplissage, dialogue des
  modèles, retour depuis le cockpit. Aucun POST de préparation ni appel IA.
- Catalogue `node tests/i18n_test.cjs` : PASS.
- Scanner des nouveaux fichiers : aucune couleur littérale ni jeton indéfini.
- Huit captures desktop/mobile inspectées par IA ; aucune inspection humaine
  revendiquée. Lisibilité, hiérarchie des surfaces et numéros contrôlés.
- `git diff --check` : PASS.

## Incidents et limites des contrôles

Le premier scénario de navigation automatisée bloquait sur le dialogue natif
qui protège le besoin prérempli non enregistré : le test accepte maintenant
explicitement de quitter ce brouillon de fixture. Il attend aussi le DOM du
cockpit, sans exiger la fermeture de ses connexions. Un POST de visite du cockpit
est distingué des mutations de préparation. Les traces d’échec ne valent pas
échec moteur ou refus de candidat.

Le contrôle existant design_system_ui.cjs n’est pas déclaré passé : Chromium a
perdu sa cible pendant une capture, sans diagnostic de cause démontré. Le contrôle
preparation_templates_ui.cjs a également échoué sur son scénario de méthode
indisponible (suppression locale du skill debug, mais disponibilité annoncée).
Ces deux contrôles n’ont pas été transformés en succès ni modifiés pour masquer
leur résultat. Les nouveaux parcours de cet accueil sont vérifiés séparément.
Aucune suite Go globale ni revue indépendante de ce lot UI n’est revendiquée.

## Présentation

Aperçu isolé sur http://127.0.0.1:18848/projects.html?lang=fr, lancé par swarm.sh,
racine de fixture `/tmp/swarm-projects-home-hsch8k`. Ce site contient une mission
de démonstration. Le cockpit vivant sur 18792 n’a pas été redémarré ; les campagnes
sur leur candidat figé continuent sans modification. Aucun commit ni push.


## Extension vérifiée — sujets spécialisés

La tranche suivante ajoute six sujets à l’accueil et au catalogue partagé,
avec les quatre interventions créer/faire évoluer/corriger/migrer. Le suivi
automatique des étapes reste à réaliser. La recette d’accueil a repassé sur
/tmp/swarm-subjects-20261010 avec les nouvelles entrées ; voir
[le rapport de cette tranche](preparation-subjects-20261010.md). Les contrôles
Go globaux de cette tranche n’effacent pas les incidents UI historiques ci-dessus.
