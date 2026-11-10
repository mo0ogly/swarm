# Actions de confiance — reprise du stockage, 24 octobre 2026

L’utilisateur autorise la reprise avec GLM/SKYNET, avec priorité au portail. T1 est marquée accepted dans l’état historique ; T2 « Livrer le pilote avec Biba » est blocked, T3–T8 todo. Le blocage enregistré concerne l’objet Git `a6486106403b64c2c344568576479dc605742ac1`. Ce travail n’autorise ni le retour au moteur Go ni la falsification de la révision de T2.

## État vérifié

- Le dépôt géré signale un loose object vide pour cet identifiant. La lecture `cat-file -t` échoue dans le dépôt canonique et les trois copies T1/T2 examinées : cela prouve une non-résolution, pas l’absence physique universelle de l’objet.
- Onze index de packs des dépôts locaux identifiés ont été lus avec `git show-index` ; aucun ne contient cet identifiant exact. Les index ne prouvent pas l’absence dans une sauvegarde ou dans tout autre dépôt.
- La mission est suspendue par opération publique Rust, décision explicite de l’utilisateur. Mode, consentement et tables de financement sont conservés. Le bail historique n’est pas attribué à un epoch Rust ; un unpause seul ne le migrerait pas.
- Les deux contributions GLM1431 ont répondu ; leur avis ne vaut pas exécution. Codex rejette la proposition « fetch puis supprimer l’objet vide » : aucune source saine n’est trouvée, et supprimer une preuve ne résout pas un objet manquant. La qualification de corruption après saturation reste une hypothèse pour cet objet précis.

## Condition de reprise T2

Propriétaire : Codex pour le diagnostic, détenteur d’une sauvegarde saine pour la source manquante. Il faut une source identifiée contenant l’objet exact et son graphe requis : type brut, recalcul du hash Git incluant son en-tête, connexité jusqu’à la révision attendue et arbre de données. Sauvegarder les originaux, prendre l’exclusion réellement partagée avec les écrivains, puis préparer une restauration atomique vérifiée sans changer refs, historique, budgets ou tentatives. Aucun nettoyage/repack/prune/reset ni import distant n’est autorisé implicitement.

Si une source saine est fournie, vérifier d’abord dans un magasin d’objets isolé, conserver les octets endommagés et empreintes, appliquer seulement la restauration prouvée puis contrôler le graphe et les reçus de T2. Si elle n’existe pas, conserver le blocage et proposer au propriétaire une récupération documentée depuis les artefacts disponibles ; ne pas inventer un commit égal à celui perdu. Toute replanification doit conserver l’ancien identifiant, ses tentatives et preuves.

La restauration Git, la migration/fin contrôlée du bail historique et la recette Biba/identités sont trois contrats distincts. Aucun patch Rust n’est justifié par le seul objet vide sans producteur démontré. Aucun agent natif n’est relancé ni aucune mission dépausée par ce document. Suite indépendante autorisée : portail, dont les fondations ont été récupérées sans perte.

Preuves privées conservées : `.cache/skynet-post-launcher1431/object-observations.json`, `trust-copy-search.json`, `trust-pack-search.json`, requêtes/réponses GLM et sources empreintées. Les consignes n’établissent pas la réparation du stockage ; T2 reste bloquée.
