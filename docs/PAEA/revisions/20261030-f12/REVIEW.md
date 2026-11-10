# Révision F-12 du 30 octobre 2026

Besoin : corriger les guides DAF FR / CFO EN, numéroter les pieds de page,
améliorer la présentation aux experts et conserver les schémas appréciés par
l’utilisateur. Périmètre limité aux guides et à leur générateur ; aucune
modification du moteur ou livraison d’un serveur n’est requise.

## Décisions et incidents

- Origine : revue des deux guides dans ce chat. JSON visiblement dispersé dans
  le PDF anglais p.4 ; bulle d’export visible p.12 ; absence de pagination.
  Originaux préservés dans `originals/`, notamment le PDF source.
- Graphviz absent : la première génération a refusé avant livraison. Le mode
  explicite `--reuse-diagrams-from` réutilise les images intégrées. Alternative
  écartée : installer Graphviz/redessiner sans nécessité. Les données de graphes
  ne changent pas. Responsable : mainteneur des guides ; route de réutilisation
  vérifiée, route de nouveau dessin non exercée ici faute de Graphviz.
- Revue visuelle intermédiaire : encadré coupé entre FR p.11/p.12. Correction
  générale `cantSplit` pour les encadrés ; nouveau rendu vérifié. Les rendus
  intermédiaires restent dans /tmp/paea-f12-edit-20261030 ; seuls les rendus
  finaux sont conservés dans ce dossier.
- GLM HTTP direct sans outils : une contribution éditoriale terminée, sans retry,
  vérifiée puis raccourcie/corrigée avant intégration. Suppression des mentions
  de processus internes et de décision de dépôt. Pas de revue indépendante
  prétendue. Identifiant demandé/déclaré glm-5.2 ; cela ne prouve pas la version
  réelle du backend. Requête, réponse et usage conservés dans `glm/`.

## Revue des octets modifiés

Le diff exact du générateur est conservé dans `generator.diff`, les empreintes
du générateur, du JSON, du README, des DOCX, des PDF et des images dans
`verification.json`. Le JSON éditorial comporte les textes réellement consommés,
FR et EN ; il est inclus dans le kit complet.

Politiques : aucune nouvelle politique d’exécution, tentative, délai, budget ou
plafond métier. PAGE/NUMPAGES/cantSplit/tblHeader sont des mécanismes OOXML ;
0,3 pouce, largeur imprimable et styles sont des choix de mise en page.
Le nombre onze est l’inventaire du document de référence, pas un plafond métier.
La date identifie cette révision éditoriale, sans exception de comportement.
Les montants fictifs et plafonds illustratifs existants restent inchangés.

Fonctions modifiées : draw (réutilisation, refus et précondition Graphviz), setup
(champs et rendu), note (encadré entier), tbl (en-têtes et lignes), code (alignement,
texte illustratif), figure (largeur imprimable), build (sources éditoriales,
validation, document et remplacement atomique), main (entrée CLI et erreurs).
Docstrings relues contre leurs corps. Pas d’API Swarm ajoutée. Les changements
n’altèrent pas les onze définitions de graphes de chaque langue.

Erreurs : JSON absent/invalide, champ manquant et référence incomplète refusent
avec cause Python conservée ; pas de résultat vide déclaré réussi. L’absence de
Graphviz reste explicite hors mode de réutilisation. La sauvegarde DOCX est faite
par temporaire adjacent, flush/fsync puis remplacement, avec nettoyage du seul
temporaire détenu ; les exceptions ne sont pas absorbées. La génération d’un kit
complet n’est pas transactionnelle entre langues ni archives ; utiliser un dossier
neuf, contrôler les deux résultats puis livrer les fichiers vérifiés. Aucun test
ENOSPC n’a été exécuté et aucune atomicité inter-fichiers n’est revendiquée.

## Vérifications effectuées

- Compilation Python du générateur : PASS.
- Génération FR/EN avec le runtime Python fourni : PASS ; sans Graphviz.
- Référence sans image : refus ValueError attendu avant génération : PASS.
- Images intégrées : 11/11 par langue, empreintes identiques et ordre conservé.
- Rendu par LibreOffice fourni : FR 18 pages, EN 17 pages ; toutes inspectées.
  Après correction des encadrés, les pages FR 1–10 réutilisées sont identiques
  octet pour octet aux PNG déjà inspectés ; FR 11–18 et EN 1–17 inspectées sur
  le dernier rendu. Pas de chevauchement ou texte coupé observé. Les schémas
  horizontaux restent compacts, conformément au choix de préserver leur dessin.
- Chaque page PDF contient son numéro et le bon total ; champs PAGE/NUMPAGES
  présents dans les DOCX ; aucune bulle d’interface « Exports page by page ».
- JSON lisible, synthèse et annexe experte présentes, titre « fully specified
  experiments » remplacé. Les scénarios ne deviennent pas des tests exécutés.
- Livraison DOCX/PDF par temporaires adjacents synchronisés et empreintes comparées.
- `git diff --check` : PASS. Le dossier PAEA étant déjà non suivi au début du
  travail, le diff utile de ce générateur a été produit contre l’original sauvegardé
  et relu ; la commande Git seule n’établit pas cette revue.

## Limites et suite

Cette correction est éditoriale et vérifiée dans le rendu fourni, sans qualification
bancaire ou acceptation indépendante. Word peut recalculer la pagination selon
ses polices. Le manuel général anglais n’était pas demandé dans cet incrément :
sa contradiction d’introduction et ses diagrammes avec échappements restent à
traiter sur demande de modification de ce manuel. Propriétaire : mainteneur PAEA ;
critère de reprise : réviser son propre générateur puis vérifier son rendu.
La comparaison scientifique détaillée et le paquet public de preuves Swarm restent
à construire avant toute revendication de contribution validée ou d’adoption.
