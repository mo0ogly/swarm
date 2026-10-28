# Publication Git de la livraison dev78

Demande utilisateur : « commit push ». Le snapshot publié reprend les sources
compilées de dev78, ses dépendances et ses recettes, puis retire 929chemins Go
historiques et actualise les entrées de distribution Rust. Les fichiers locaux
ignorés, historiques et preuves privées restent préservés ; aucune réécriture
de l’historique Git n’est demandée ou effectuée.

Un index séparé permet de publier ce livrable sans indexer les nouvelles versions
de fichiers encore modifiées par d’autres sessions. Deux fins de fichier sont
normalisées dans le snapshot uniquement (rapport de livraison et managed_receipt.rs),
sans changement du contrat Rust ; l’archive du binaire dev78 conserve les octets
exacts réellement compilés. Les réécritures éditoriales des README et leurs médias
non publiés restent locales : seules les informations Rust/distribution sont reprises.
Les suppressions de preuves et deux ajouts Go déjà indexés hors périmètre sont
conservés dans l’index de travail, sans les publier dans ces commits.

Vérification : 49assertions publiques, 8identifiants, 2tests SDK, 6de reprise et
14contrôles de version installée conservés dans le [rapport de livraison](DELIVERY-PREPARATIONS-20261025.md).
Contrôles de publication : 11tests du lanceur sur binaire78 et Roots isolées,
4 tests de distribution, 3de numérotation, vérification des entrées Rust sans Go,
liens des README/installations présents dans l’index publié, syntaxe shell et
`git diff --check`. Ces résultats ne constituent pas une acceptation globale
de la migration ni des fonctionnalités Biba/SCALING incluses au snapshot installé.
Aucun fichier de session, clé, base ou archive privée n’est ajouté. Le push porte
sur la branche de travail existante et conserve les commits déjà présents.
