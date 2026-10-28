# Historique d’agents importé — lot CLI vertical

Owner Codex ; migration, aucun fournisseur réel ni écriture d’un Root vivant.
1. Go agent history lit imports/ID/manifest.json et rend {historical_only:true,history:Bundle.Cockpit}. Rust refuse cette entrée.
2. Inventaire réel :111types Bundle/CockpitHistory, aucun codec custom, seul externe json.RawMessage. Garder cette provenance figée ; aucun Go requis en production.
3. Réutiliser le projecteur JSON Go pour valider aussi les champs connus inutilisés du Bundle ; ne pas inventer une réimportation ou validation métier actuelle.
4. Lire les répertoires privés par descripteurs, sous réservation/association Root ; aucune création, signal, DB/agent/visite/ledger modifié.
5. Différencier null/vide, aliases/duplicates/unknown, erreurs types/UTF8 ; agents « running » restent des archives inertes, pas un processus adopté.
6. Oracle JSON Go sur le DTO figé, plus parcours CLI réel et refus/reprise ; aucun test de fournisseur ni plafond caché de lecture.
7. Docs/Rustdoc FR/EN, revue politiques/fonctions/erreurs, recette, profil normal et installation locale exacte avant livraison.
8. Annonce fichiers : nouveaux archive_history*.rs, root_store/agent_archive_history.rs, cli/agent_history.rs ; entrées additives RootStore/lib/agent/help seulement après fin des compilations concernées ; contrat/générateur/tests/recette/docs dédiés.
9. GLM autorisé pour une proposition isolée avec références, avant/après et DTO explicitement distingués ; résultat soumis aux tests, pas acceptation.

État : analyse ;111types observés, aucune implémentation revendiquée.
Preuves en cours : .cache/agent-history-next ; Go local ignoré ne rejoint pas le produit ni Git.

Choix : graphes de types historiques figés, distincts des extensions natives du
codec Agent courant ; aucun DTO Work/assistant dupliqué, seules leurs formes
JSON connues sont validées. Projection typée Cockpit/Decision/Visit/Log puis Agent
existant, extensions natives absentes après projection. Aucun accès HTTP historique
correspondant trouvé dans les routes Go ; ce lot reste CLI/données. Lecteur privé
réutilise LocalDirectory child/read/close et material_path, pas un nouvel ouvreur.
Sources nouvelles préparées mais pas encore raccordées/compilées ; aucune preuve
produit avancée à partir de leur seule présence.

Premier témoin refusé avant agent history : le producteur de fixture mkdir
parents=True appliquait0700 au seul enfant, parent imports créé0755/0775 selon
umask. Root a correctement refusé le contrôle work show. Original FAIL retenu ;
fixture corrigée en créant explicitement imports0700 puis enfant0700, sans
modifier les lecteurs ni reprendre le Root défectueux comme preuve valide.

Témoin219 corrigé : contrôle work show valide, agent history refuse et SQL
conservé (GAP3). OracleGo indépendant de tout Store/provider, sur111DTO figés,
45cas validés avant Cargo : champs connus inutilisés invalides réellement refusés.
GLM livre le lecteur Root :24058tokens déclarés/320.808s, requête/réponse5.2 ;
corps de fermeture imbriquée repris après revue. Son hypothèse name(&str)->Result
est fausse : fonction réelle name(&[u8])->bool ; corrigé avant compilation.
Rustdoc « bounded » précisé comme association physique, pas borne d’octets.
Aucun avis statique ni présence du fichier n’est une recette/livraison acquise.

Contrôles avant raccordement : générateur3PASS (rejeu byte exact sans lectures
Go, vocabulaires refusés, ENOSPC avant replace préserve l’original et reprise) ;
oracle45cas validés, témoin public219 GAP3. Deux Cargo moteur/web2734606/2734684
encore actifs : déclarations partagées différées, sources nouvelles non référencées
seulement. Aucune recette en cours d’autrui invalidée volontairement.
