# Sujets de préparation — 10 octobre 2026

## Périmètre adopté

Six sujets FR/EN : outil interne, portail client, API et intégration, gestion des
stocks, traitement de données et tableau de bord. Chaque sujet propose un
résultat, un exemple, trois livrables, trois preuves et des risques. Quatre
interventions : créer, faire évoluer, corriger, migrer. Les quatre structures
historiques et leurs identifiants restent disponibles.

Le catalogue est une surcouche de cinq étapes communes au produit puis six
étapes répétées par fonctionnalité. Il n’implémente aucun suivi automatique des
étapes et aucune UX d’exécution de processus métier. Wattson reste séparé.

La projection publique CLI/API complète le besoin ; corriger propose debug,
les autres interventions le parcours produit. Les réponses, choix et brouillons
existants sont préservés. Le choix ne lance aucun agent ni appel IA, ne crée
aucune mission et n’autorise aucun budget, lancement ou migration réelle.

## Candidat et vérification

Branche codex/clear-product-method-names, checkout sale observé et préservé.
Binaire canonique /tmp/swarm-subjects-20261010 ; racines de tests temporaires
isolées, sans accès au stockage des missions vivantes. Les modifications
préexistantes de recherche et de formation ne sont pas attribuées à ce lot.

- Test de régression rouge avant catalogue : internal-tool absent.
- Go ciblé TestPreparation(Template|Subject) : PASS, 1.710s ; 48 projections
  sujet/langue/intervention, refus inconnus, réponses conservées et aucun départ.
- npm test : PASS, y compris parité et formats i18n.
- Configuration agent-workflows/check.py : PASS.
- Recette navigateur preparation_subjects_ui.cjs : PASS sur dernier build.
  Six sujets, quatre variantes, FR/EN, deux thèmes, CLI/HTTP identiques,
  conservation des réponses et du brouillon, focus/Échap, champs distincts en
  clair, largeurs 1440/1024/760/390/320, panne du catalogue et reprise, panne du
  contrôle et reprise, enregistrement explicite relu par CLI. Aucune mission
  ni aucun agent créé ; seuls contrôle déterministe et enregistrement explicite.
- Recette projects_home_ui.cjs : PASS sur le même binaire.
- Suite `go test -json ./...` : PASS, 461.511s ; 1 787 tests/sous-tests passés,
  aucun échec. Huit contrôles optionnels ignorés explicitement (voir
  qualification.json) ; les recettes navigateur applicables ont été exécutées
  séparément. Aucun fournisseur réel n’a été appelé.
- `go vet ./...` : PASS.
- Scanner statique : 25 jetons définis, aucun littéral de couleur ajouté ni
  style inline coloré. Le premier scanner confondait white-space avec une
  couleur ; l’analyse corrigée exclut ce faux positif.
- 13 captures du catalogue/modales desktop/mobile et huit captures d’accueil
  inspectées par IA. Hiérarchie, champs, focus, deux thèmes et langues examinés.
  Aucune inspection humaine revendiquée.
- `git diff --check` : PASS.

Les logs Go et stderr sont conservés /tmp/swarm-subjects-validation-20261010.
Le résumé versionné est docs/screenshots/preparation-subjects-20261010/qualification.json.
Les huit skips : TestOperatorPreview, TestCorrectiveRecoveryBrowserRecipe,
TestEngineContractTruthBrowser, TestHistoricalRequalificationBrowser,
TestManagedRecoveryBrowserRecipe, TestMissingReportRecoveryBrowserRecipe,
TestRecoveryHealthBrowserRecipe, TestLiveProviderStreamReplay.

Le manifeste UI est un instantané d’intégrité courante, pas un avis indépendant.
Son état préexistant est sauvegardé /tmp/swarm-subjects-manifest-before-20261010.json.
Aucune preuve ni acceptation historique n’est renouvelée par son recalcul.

## Incidents des recettes

Deux assertions de la nouvelle recette ont été corrigées : attendre l’événement
asynchrone close avant de vérifier le focus, puis accepter la majuscule réelle
du message anglais Incomplete. Elles ne prouvaient pas un défaut du produit.
La première recette complète a ensuite passé. Le contrôle visuel a conduit à
mieux distinguer les champs et les en-têtes de la modale en thème clair, puis
à relancer la recette sur ce candidat modifié. Les captures du sujet sont
replacées en haut de la modale pour montrer ses métadonnées.

Les échecs antérieurs des recettes design_system_ui.cjs (cible Chromium perdue)
et preparation_templates_ui.cjs (simulation locale de méthode indisponible
incompatible avec le fallback embarqué) restent des échecs historiques ; ils
ne sont pas présentés comme réussis. Aucun appel à un relecteur LLM ni aucune
acceptation gérée du lot n’est revendiqué.

## Présentation vérifiée

Aperçu isolé 18848 actualisé par swarm.sh avec le binaire canonique, racine
/tmp/swarm-projects-home-hsch8k et une mission de démonstration. État ready
confirmé, puis navigateur existant rechargé : six sujets et liens de préparation
réellement affichés. Vue présentée : projects.html?lang=fr#models. Ne pas redémarrer 18792
ni modifier le candidat figé des campagnes de facturation. Aucun commit/push.


La capture d’erreur a été recadrée sur l’alerte réellement visible après la
suite Go, avec une recette ciblée sur une nouvelle racine isolée : PASS,
réponse préservée. Ce dernier changement touche uniquement le cadrage de la
recette et la documentation, sans modification du candidat produit testé.


Le compteur du catalogue dans la recette historique preparation_templates_ui.cjs
est adapté de quatre à dix, pour correspondre au contrat actuel. Son scénario
historique de fallback n’est ni supprimé ni déclaré passé ; cette recette entière
n’est pas rejouée comme preuve de ce lot. L’intégrité du dernier instantané est
contrôlée par TestGraphDeliveryD03QualificationManifest, séparément de la suite
Go déjà passée et sans renouveler d’acceptation historique.
