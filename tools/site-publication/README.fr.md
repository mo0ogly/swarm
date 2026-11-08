# Publication et sitemap

Le déployeur maintenu régénère le sitemap de la future release avant activation. Une erreur de génération/écriture, un inventaire attendu périmé ou une divergence HTTPS fait échouer la publication ; après bascule, la release complète et son sitemap sont restaurés.

La liste publique vient des six pages de présentation et des paires FR/EN d’articles liées aux accueils. Pas de parcours récursif des fichiers privés/relectures. Une page manquante, une langue incohérente, noindex ou une canonical contradictoire refuse. Cela ne remplace pas le contrôle d’accès.

Le préparateur d’ajout et le builder éditorial appellent le même générateur. Les fichiers privés du dossier `docs/communication/` restent ignorés ; les points d’entrée éditoriaux redirigent vers ces sources maintenues. Le retrait est explicite : slug, fichiers retirés, accueils et inventaire final doivent correspondre. Aucun article réel ajouté ou retiré pendant la recette.

Parcours VPS : `/var/www/swarm-agentic/publication-tools/current/deploy_article.py CONFIG.json`. L’option `--check-current` est une lecture seule sur ce VPS, pas une commande d’aperçu local. Version initiale installée `publication-c19289ede93d`. Les anciens dossiers incoming ne sont pas le parcours maintenu. Aucun service redémarré, reload ou contenu public changé pour installer les outils.

Neuf tests nommés passent ; ajout/retrait et rollback sont exercés dans des racines isolées avec doubles réseau/services explicites. Le contrôle installé et la reconstruction complète retrouvent les 60 URL publiques actuelles. Les 420 fichiers publics, les configurations, identités de services et trois témoins LIA sont inchangés. Ce n’est pas la preuve d’une future publication réelle ni une garantie d’indexation Google.

Contrat JSON, commandes et limites : [README anglais](README.md). Toute publication requiert toujours son autorisation ; les outils n’accordent ni droit supplémentaire ni permission de modifier LIA ou les travaux parallèles.
