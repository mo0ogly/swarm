# Vérification de la consolidation UX du 8 novembre 2026

## Provenance et conservation

Base de consolidation : `015377e4d6fa2d3cbe495836fa16456ca92ce5f8`, tête
observée de `claude/design-mockups` / PR #19. Elle contient déjà Couloirs, les
badges, Sécurité, Audit et l’index du canvas étendu. Les retouches locales de
`design-swarm/project/` et `design-admin/project/` ont été rapprochées des
originaux antérieurs avec une fusion à trois versions sans conflit. Le sélecteur
Graphe/Couloirs, les badges et les deux nouvelles planches sont conservés.
Le correctif de grille Couloirs utilise `minmax(min(100%, 280px), 1fr)`.

Préparation dans un répertoire temporaire contenant uniquement `docs/design/` ;
commit avec un index Git séparé pour préserver l’index, la branche `main`, les
fichiers et le travail non lié de l’autre agent. Aucun worktree existant supprimé.

Archive originale des deux dossiers :
`/media/fpizzi/Lexar/swarm-evidence/design-ux-20261108/originals.tar`, SHA-256
`2f3bac8d6452c60c2a96339695e751d8d36de7eccf9fa7e47bd752ad2c46d044`.
Index local de cette archive : `.swarm-fixtures/design-swarm/README.md`.
Extraction de restauration dans un dossier vide :
`tar -xf CHEMIN/originals.tar -C DESTINATION` ; comparer avant remise en place.

## Contrats examinés

- **Politiques et constantes** : valeurs de champs, historique et niveaux de
  sécurité sont illustratifs. Les largeurs CSS sont des choix de présentation.
  La validation positive/safe-integer de la proposition Admin n’est pas copiée
  dans le moteur et ne prétend pas être sa politique. Aucune limite produit changée.
- **Fonctions** : `renderVals` fournit les données/handlers de maquette ; `initial`
  crée le brouillon illustratif. Les commentaires de l’adaptateur décrivent la
  portée locale, l’évaluation de code de confiance, les erreurs visibles et la
  conservation des saisies/focus. Les fonctions de badges restent de présentation.
- **Erreurs et effets** : absence de motif et nombres invalides empêchent aperçu
  et simulation Admin. Un changement invalide ne se transforme pas en héritage
  silencieux. L’adaptateur affiche les erreurs d’expression et les propage. La
  simulation est distincte d’une persistance ; le verdict Audit reste illustratif.
- **Sources/publication** : le chemin de compatibilité `/home/fpizzi/workspace/swarm`
  a été résolu vers le checkout courant. Les passages UI Biba/BLP, `api` et
  `settings.close` ont été lus pour borner les formulations du README. Aucune
  preuve d’exécution du moteur n’est déduite des maquettes.

## Contrôles et limites

Recette : `verify-preview.cjs`, Chrome installé, URL `file://`, dépendance
Puppeteer du projet. Premier passage : **98 assertions réussies** sur les sept
planches, clair/sombre, largeurs 1440/1024/760/390/320, expansion des variables,
logos locaux, parcours de démonstration et absence d’erreurs navigateur/requêtes
échouées. Preuves :
`/media/fpizzi/Lexar/swarm-evidence/design-ux-20261108/pr19-01/`.

Passage final : **132 assertions réussies**, preuves dans
`/media/fpizzi/Lexar/swarm-evidence/design-ux-20261108/pr19-03/`.
Il ajoute chaque onglet Sécurité et ses largeurs bureau/mobile,
la sélection de transaction et la remise à zéro du verdict illustratif Audit,
ainsi que la sélection Couloirs et le lien retour au Graphe. Le passage
`pr19-02` a révélé un débordement du libellé masqué du tableau Confidentialité
à 320 px ; le positionnement de ce libellé a été corrigé dans `preview.css`,
puis les 132 contrôles complets ont réussi. Les preuves d’échec sont conservées.

Les événements de démonstration sont déclenchés dans Chrome ; cela ne qualifie
pas tous les gestes humains, la modalité complète, les lecteurs d’écran, WCAG,
les traductions EN ou l’import dans Claude Design. Pas de qualification produit,
de revue indépendante ou d’acceptation globale. Les fichiers `web/`, le registre
de versions et la documentation produit FR/EN ne sont pas modifiés par cette PR.

Les tentatives de prévisualisation antérieures et leurs échecs restent conservés
au même emplacement USB ; leurs résultats ne sont pas présentés comme réussis.
