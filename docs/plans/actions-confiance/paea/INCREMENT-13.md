# PAEA — incrément 13 : guide opérationnel et parcours UX

[English](../../../en/plans/actions-confiance/paea/INCREMENT-13.md) ·
[Plan](../PAEA-APEX.md) · [Guide et correspondance](GUIDE-INTEGRATION.md)

## Reprise et guide

L’utilisateur demande « next », puis fournit le guide pour comprendre PAEA et
guider l’implémentation. Codex racine travaille seul dans le checkout direct,
**sans compilation ni redémarrage**, commit/push ou effet réel. Le candidat
identifié reste dev77, sans la nouvelle route HTTP PAEA. Les entrées de 12
étaient initialement inchangées. Une lecture d’un chemin de helper supposé
absent a refusé ; les chemins réels ont ensuite été recherchés.

Le plan initial visait PAEA-UX-02 (contraste). Le guide a changé la priorité :
lecture/confrontation, puis correction bornée. `plan.json` et `guide-replan.json`
conservent les deux décisions. Les prescriptions documentaires ne deviennent
pas une autorisation de session.

Le DOCX original est conservé à l’identique sous `docs/security/`, SHA256
`e621efa0953ff056cf12b318236849764feeff16f9e2d2f3c5b4016d75774298`.
Lecture complète : **288 blocs texte/tables**, deux figures inspectées, aucun
suivi de modification ni note. Le runtime fourni a produit **34 pages** ;
pages pertinentes examinées, dont §7.1/page10 et ateliers/annexe A/page29.
Aucune qualification de mise en page de toutes les pages ni nouveau DOCX livré.
L’original dans Downloads reste identique.

La [matrice FR/EN](GUIDE-INTEGRATION.md) relie chapitres, lots, responsables,
sources, état et preuve attendue. Priorité : dossier complet d’une action,
effet/risque/propriétaire, menace, accès, intention, politique, point d’engagement,
oracle et trois pannes. Règlement fictif/Git maintenus ; IAM/industrie restent
hors exécution autorisée. Le menu de correspondance reste un composant ; parcours
opérationnel cible et dossiers conception/réalisation/qualification non livrés.

Clarifications : ERR-01 s’applique aussi à la première consommation après
révocation du §7.1 ; JSON/figures pédagogiques ne remplacent pas les schémas ;
l’identité projet+facture exige une résolution protégée unique de la dette.
Aucun hash v1, seuil, classe ou schéma n’est changé implicitement. Les sources
externes du guide ne sont pas réévaluées ici.

## Correction et preuves

Dossier : `evidence/20261025-ux13-jqrmvwu_/`.

| Étape | Observation | Portée |
|---|---|---|
| `trust-before-recipe-01` | FAIL Enregistrer : **3,0698:1** en thème clair | Page réelle et témoin de chargement valide, API de lecture/aperçu simulée. |
| Diagnostic de mesure | Exécuter mesuré pendant une transition CSS | La couleur intermédiaire masquait la paire finale, ce n’était pas une preuve au repos. |
| `trust-before-recipe-02` | FAIL Exécuter stabilisé : **3,0698:1** | Attente des animations réelles, sans sleep arbitraire. |
| Correction CSS | Paire `action-fond/action-encre/action-bordure` | Une règle pour Exécuter/Enregistrer, aucune logique/droit changé. |
| `trust-after-recipe-01` | Couleurs PASS, contrôle de focus FAIL | Le test regardait avant l’événement close ; original conservé. |
| `trust-after-recipe-02` | **362 assertions PASS**, 60 lectures/aperçus, **180 mesures** | FR/EN × thèmes ×1440/1024/760/390/320 ; repos/survol/clavier, dialogue et lien PAEA. Minimum **9,096:1**. |
| Fermeture | **20 événements close, focus final admin** | Attente de l’événement après le handler de la page ; aucune correction JS produit nécessaire. |
| Rétention | Recette finale **PASS, intégrité VERIFIED** | Trois échecs antérieurs conservés ; aucune revue indépendante. |

La recette permanente `trust-primary-ux.json` sert les assets du workspace.
Elle n’accepte que lecture/prévisualisation : aucun apply ni effet métier.
Elle réexamine aussi PAEA avec la mesure stabilisée. La qualification CLI de 12
reste distincte ; aucun parcours HTTP natif ou binaire installé démontré ici.

## Revue et transmission

`source-review.json`, contrôles statiques, diff et inventaire final lient la
revue aux octets. Documentation FR/EN, suivi et notes locales actualisés ; aucune
installation revendiquée. Les seuils 4,5:1/largeurs viennent du contrat UX ;
session/politiques de fixture sont des données de test. Aucun retry, watchdog,
plafond produit ou réglage Admin ajouté. Cinq helpers JS documentés ; aucune
fonction produit modifiée, seulement la règle CSS Trust.

Erreurs : assertions/refus du double/échecs de ressource restent des échecs.
Les nouvelles préconditions observables sont fin d’animation et événement close,
pas une relance inchangée. Une édition documentaire a refusé un titre anglais
supposé ; titre réel relu et absence de modification partielle vérifiée.
Une autre édition a refusé un hunk mal formé ; le constructeur de patch a été
corrigé avant reprise, sans compilation dépendante ni original remplacé.

L’apprentissage est transmis à `docs/UI-DESIGN.md` et à la référence anglaise
`docs/en/WINDOW-IDENTITY.md`. Ces fichiers persistants ne changent pas un binaire
embarqué ou un contexte d’agent déjà lancé. PAEA-UX-02 est vérifié **au niveau
composant** ; les preuves historiques ne sont pas réécrites. Les ressources
embarquées nécessitent toujours le build supporté, actuellement différé.

## Retour au besoin

Le guide est intégré ; aucun effet protégé n’est démontré. **0/17 invariants de
bout en bout**, PA-02/03/04…, installation/E2/E3/RETEX ouverts. Suite fonctionnelle :
dossier d’une action et oracle, puis trois recettes au sink dans les droits
existants. Qualification native et livraison après levée explicite de la
consigne de compilation/redémarrage ; aucun droit créé par le guide ou les tests.
