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

Reprise28octobre : dépôt publié HEAD9ebb8e393, ancien cache de preuve absent ;
les sources/contrat/tests persistent. Deux contributions GLM isolées lancées
(reader et tests), contexte borné et empreintes conservés dans
.cache/agent-history-next/wave-20261028. Aucun Cargo actif avant les quatre
raccordements additifs ; PAEA77 en cours reste hors de ce lot. Les anciens
résultats ne sont pas réexécutés ni déclarés disponibles sans inventaire.
Raccordements présents, compilation/recette/installation encore à vérifier.

Replan concurrence28oct : un build PAEA77 rust-t6k71_ba a démarré entre
le contrôle sans Cargo et nos raccordements. Aucun build tiers n’est revendiqué
comme notre preuve ; les sources partagées sont gelées dès observation. Le status
local attend son verrou de livraison, aucun processus tiers arrêté. Les contrôles
du générateur sont indépendants du profil Cargo ; recette publique sur notre
propre image figée et installation restent à établir.

Témoin nouveau sur image232 réellement exécutée /proc/3954299/exe :
contrôle work show valide, history refusé et SQL inchangé (GAP3).
Générateur rejoué3PASS. Recette permanente agent-archive-history-rust créée,
deux étapes : générateur et parcours public sur SWARM_AGENT_HISTORY_BINARY
explicitement figé. Aucun watchdog global ajouté ; aucun appel métier.

Build concurrent terminé en erreur : CLI compilée avec notre nouveau lecteur
mais ancienne bibliothèque sans méthode archived_agent_history (E0599 puis
erreurs d’inférence secondaires). Diagnostic conservé concurrent-build-errors.json ;
aucun faux correctif de type appliqué. Le Cargo encore actif utilise une copie
isolée docker-context, non nos sources canoniques. Nouvelle précondition :
build canonique terminé, modules complets et notes FR/EN maintenant raccordés.

Pendant les analyses GLM, ajout direct de la garde sans --json : lancement
public explicite, argv/stdout/stderr conservés, JSON et SQL inchangé comparés
au témoin. Corpus45cas,102assertions attendues ; la recette exige le compte
exact, encore non exécuté sur le candidat. Une proposition GLM ultérieure doit
être réconciliée avec cette évolution, pas appliquée aveuglément.

Livraison233 : build rust-4ptnbjhy qualifié, image
b11b4bafabff1d3bf5d216fdb44f5614fb69a41a61b3b72e8623bc2a80b385cf,
profil normal4artifacts core sans features/test. Recette2steps PASS/intégrité
VERIFIED : générateur3PASS et public102assertions/45cas. Redémarrage préparé
exact via swarm.sh ;31contrôles installés dont14Version FR/EN, et8spécifiques
(notes history, réglages, image et5sources compilées) PASS. Cinq travaux conservés.
Les GLM sont encore actifs ; aucun avis ni économie revendiqués avant réponse.

Avis GLM lecteur reçu : requête/réponse glm-5.2,25121tokens déclarés,
775.546s. Patch rejeté : il ajoute empty supposé absent alors que la fonction
figure déjà dans le contexte exact (ligne5), puis compilation normale et
102assertions l’exercent. Aucune économie ni correction fictive revendiquée.
Apprentissage : toute « fonction manquante » alléguée doit être confrontée
aux octets réellement transmis et au build réel avant ajout ; ne pas dupliquer
un helper parce qu’un avis le demande. Propriétaire Codex, effet observable :
proposition rejetée, sources inchangées, test existant conserve le témoin.

GLM tests :35239tokens/838.228s, requête/réponse5.2. Total vague60360tokens
déclarés, aucune économie Codex mesurée. Proposition sans --json rejoint le
contrôle déjà ajouté ; son compte100/101 omet baseline-historical-wire, rejeté
face à l’exécution102. Signal utile : context manager SQLite ne clôt pas la
connexion explicitement. Reproducteur sur corps sql réel : témoin valide mais
0fermeture ; correction closing(...) puis1fermeture. Recette fraîche
recipe233-closed en cours, même image installée inchangée ; pas de rebuild requis.

Recette finale3steps PASS/intégrité VERIFIED : générateur3tests,
fermetures explicites2contrôles (succès et cause d’itération conservée),
public102assertions/45cas. La garde de ressource permanente est
tools/rust-migration/check_archive_history_harness.py ; elle échoue sur
le corps original extrait de l’archive avant correction. Image produit233
inchangée : correction du banc uniquement, installation déjà vérifiée.
RETEX : intégration verticale terminée sans adoptions historiques ; un avis
GLM rejeté, une amélioration de banc retenue, aucun gain de tokens démontré.
Migration globale ouverte : agent list nécessite le véritable cockpit composite
à14clés (console.go:21), pas un alias list_agents ; aucune valeur actuelle ni
autorité de décision ne sera inventée pour combler ce contrat. PAEA77 parallèle
continue sous son propriétaire ; aucune modification de ses fichiers ni commit.

Conservation finale : evidence/20261028-agent-history contient9archives
vérifiées en lecture,4Roots inertes possédés et1ELF production lié par empreinte.
Aucun original supprimé, aucune Root vivante archivée. Inventaires de recette
historiques préservés ; annotation finale/doc-only distincte de leurs octets.
Les preuves brutes restent locales selon la décision actuelle ; plan et sources
portent la continuité. Dernier contrôle : instance233 encore active, image exacte.
