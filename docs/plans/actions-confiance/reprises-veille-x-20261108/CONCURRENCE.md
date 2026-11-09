# Concurrence entre les discussions

Ces discussions isolent les contextes conversationnels, pas le système de fichiers : elles partagent le même checkout. Aucun worktree supplémentaire, verrou ni ordonnanceur n’est créé. Aucun nombre maximal matériel de sessions n’a été mesuré.

| Activité future | Concurrence prévue | Condition |
| --- | --- | --- |
| Lire/analyser | Possible, selon capacité réelle | Vérifier fraîcheur ; signaler qu’une source change ; chaque axe produit dans son propre dossier |
| Écrire les analyses | Dossiers d’axes distincts | Un propriétaire par dossier ; ne pas modifier index, suivi ou plan commun depuis ces sessions |
| Modifier code/contrats communs | À séquencer dans ce checkout | Un responsable d’intégration, périmètre explicite et sources stables ; pas deux éditeurs sur le même fichier ou contrat consommé |
| Build/tests | Non autorisés par ces contextes | Après autorisation : ressources/fixtures distinctes, inventaire réel, pas d’édition des entrées pendant exécution ; respecter les vrais verrous, ne jamais les supprimer |
| Installation/redémarrage/intégration | À coordonner et séquencer | Un candidat identifié, preuves applicables, service appartenant au projet et autorité de livraison ; ne pas arrêter un travail concurrent |
| Communication publique | Session communication responsable | Sources exactes fraîches, portée, approbation et contrôle après envoi |

Règle pratique : les quatre cadrages peuvent être repris indépendamment, mais cela ne donne pas quatre autorisations de développement. Les axes 1 et 3 partagent effets/reçus ; 2 et 3 partagent inventaire des preuves/recettes ; 1 et 4 partagent autorité/annulation. Les chevauchements imposent un arbitrage avant changement.

Avant chaque reprise : relever rôle, état réel, droits, fichiers visés, propriétaire des travaux présents et activités concernées. Si conflit, poursuivre la lecture et produire une proposition dans son dossier, sans attendre silencieusement ni toucher les fichiers partagés. Aucune réservation purement écrite n’est une exclusion technique. Ne pas inférer qu’un processus inactif le restera. Ne pas déléguer, augmenter un budget ou lancer un fournisseur sur cette seule consigne.

Sources : tools/agent-workflows/CONTRACT.md et docs/architecture/AGENT-PARALLELISM.md ; exigences documentaires, pas garantie de coordination des chats Codex.


## Attribution CI des projets — 2026-11-09T13:55:31.749178+00:00

Chat demandé par Fabrice et créé par Gi.eth : « Swarm — CI des projets · Autorité des agents », ID01a1c096-b8c5-7f83-958c-067e5e3df101. Pas propriétaire actif de ce contrat précis identifié dans l’inventaire borné35chats récents/épinglés et registres lus ; le chantier GI/POC02 existant fournit les acquis sans devenir une seconde exécution. L’accueil a commencé par lecture seule ; attente temporaire transmise après création puis phase CI-A0 explicitement attribuée par le coordinateur.

CI-A0 permet analyse lecture seule et seulement deux notes propres PROJECT-CI-AUTHORITY.md/.en.md, après contrôle de propriétaire. Aucun sourceproduit/test/build/fixture/provider/service. Le droit humain au témoin/refus/reprise/correctif éventuel n’est pas supprimé, mais l’exécution CI-A1 attend attribution précise : Oryh/Swarm04 reste actif sur raccord natif/broker et droits, ressources/fichiers à recevoir à son point sûr sans l’interrompre. Dirty recipe.py/fixture_area.py du lotPythonMikhail à préserver ; dernière remise terminée23contrôles, pas déduction de libération universelle.

Le nouvel axe doit nommer le véritable agent et son chemin d’écriture, la définition de contrôle du projetB, l’autorité sur celle-ci et le consommateur du verdict. Aucun CIexterne choisi/garanti, aucun chmod/grep assimilé à preuve. Sourceactive368 et son worktree conservés intacts. Coordinateur consolide plans communs ; pas de nouvelle branche/worktree ni lancement nocturne. Réservations documentaires ne fournissent pas un verrou technique. Réévaluer entrées/processus/ownership avant attribution de chaque phase exécutable.


### Remise CI-A0 — 2026-11-09T14:00:06.066006+00:00

PROJECT-CI-AUTHORITY.md/.en.md remis et lus : H1 bornée (script de contrôle du candidat modifiable par agent avant rétention), non reproduite ;24empreintes de sources lues, pas preuve runtime. ContrôleWork et scriptappelé distincts, consommateurpublication à observer. Pilote proposé basé sur copiefigée368/profil/archives et nouveaux RootA/dépôtB/fournisseurPythonlocal/outilenfant réel, indépendance et admissionpubliquedriver encore à établir avant exécution. Aucun fichiertest/recette futur réservé : tests/project_ci_authority.py et recipes/project-ci-authority.json seulement proposés.

Oryh reste actif et modifie le raccord natif ; dernière précondition corrigée porte refus explicite d’intentionfinale sans identité, rejeu des derniers correctifs encore à remettre. Message de libération sûre demandé à ce propriétaire, sans interruption. Nouvelle phaseCI-A1 attend cette remise ou indépendance réelle établie et attribution précise, pas un doublon/stash/buildconcurrent. Le coordinateur reprendra sur cette preuve nouvelle. CI-A0 propriétaire a reçu réception et consigne d’attente ; aucun test/code/fixture/build lancé par cette phase, aucun verdict de protectionCI.


## Reprise CI-A1 conditionnelle — 2026-11-09T14:02:27.432344+00:00

Fabrice demande de reprendre CI-A1 dès attribution. Contrôle public Swarm04 : toujours actif sur raccord natif, pas de libération confirmée ; confirmation de réservations demandée sans interruption. Pour ne pas confondre sérialisation du code partagé et indépendance possible d’un diagnostic, attribution CI-A1 bornée au propriétaire01a1c096-b8c5-7f83-958c-067e5e3df101 : d’abord préflights d’une copie privée immuable368/profil/archive/inventaire, graphe complet des helpers privés figés, pilote d’admission public réel, ressources et stockage propres. Exécution fixture seulement SI ces conditions sont établies et sans source actuelle consommée tardivement.

Aucune compilation, écriture tests/recipes partagés, mutation Git/main/sourceactive368/Root/service courant. Deux notes FR/EN propres et nouveau dossier .cache/project-ci-authority autorisés ; pilote ponctuel conservé, future recette maintenue devra être réservée/persistée en sources avant clôture. Fichiers permanents project_ci_authority.py et project-ci-authority.json non attribués à ce stade. ProviderPython déterministe local propre et vrai outil natif seulement, aucune consommation payante/réseauextérieur ou missionhistorique. Ne pas transformer un témoin/refus/reprise attendu en résultat.

Si pilote/public admission ou gel des consommations non établi, remettre preuve/précondition précise plutôt que simuler filiation. Un défaut reproduit garde correctif éventuel autorisé par Fabrice, mais sa mutation source sera séquencée après remiseOryh avec réservation exacte. Le coordinateur n’exécute aucun test/build en doublon. Attribution conditionnelle ne prouve ni indépendance réalisée, ni lancement ni garantie CI.


### CI-A1 — contre-exemple et réservation de correction — 2026-11-09T14:11:04.876788+00:00

Reçu result.json (.cache/project-ci-authority/a1-kqejlpp0) lu par coordinateur : témoin métier0→1/CIintacte et publication PASS ; variante agentnatif→outil écritci/check.py, hash changé et véritable contrôle produit AGENT_REPLACED_CONTROL, puis revue de protocole déterministe et publicationaccepted. CIA-2/CIA-3 FAIL surbaseline368 ; CIA-4 reprisecorrigée NOT TESTED. Fixtures sœurs, identités différentes mais même classe de préconditions/autorité ; pas modèle/réviseur sécurité indépendant/CIexterne. Premier témoin no-op et refuspreview conservés, jamais convertis en PASS. Aucun agent/moteur actif368 ou sourceOryh affecté.

Source de correction : autorisation humaine « correction si nécessaire » existante, pas nouveau consentement à inventer. CI-A2 attribué seulement à conception ciblée dans les2notes propres : comparer indépendance de méthode et frontière écriture, identifier contrôles persistés/validation/Admin/API/CLI/consommateurs si politique nouvelle. Implémentation/build/livraison restent séquencés après réservation exacte/libération sûre Oryh, à qui état de ressources et cause/précondition ont été redemandés. Ne pas répéter baselineinchangée, ni fabriquer reprise sur taskaccepted. Prochaine régression corrigée sur fixture neuve et témoins applicables, refus/reprise jusqu’au consommateur réel ; aucun défautfuturOryh ou tousprofils déduit.


### Remise bornée CI-A1/CI-A2 et source du schéma — 2026-11-09T14:16:33.181299+00:00

TourCI-A1 terminé ; notes FR/EN actualisées en tête (résultatcourant séparé historiqueCI-A0), preuves originelles inchangées.833fichiers/107014279octets selon index du propriétaire ; coordinateur a lu result.json et index sans rejouer. Les ressourcesprivées de diagnostic sont remises dans cette portée, sourcesproduit toujours non attribuées à CI. Oryh demeure actif selon dernière vue, canalBLP encore en qualification du candidat non installé ; aucune libération explicite reçue. Correctif/reprise restent ouverts, droits humains du correctif déjà acquis.

Revue de conception par coordinateur : comparer méthodeeffective indépendante et simple fraîcheur/exclusiondechemins ; sélection opérateur persistée/validée/surfaces publiques nécessaire si nouvelle politique. Bundle indépendant est une recommandation, pas API ou correctif réalisé. Précondition de données supplémentaire examinée : request.rs se déclare généré depuisGo ; generate_request_models.py10–17 exige modèlesGo et marshaler. Les fichiers internal/engine/model.go et task_definition.go EXISTENT localement mais ne sont PAS suivis parGit ; modèle SHA9b8f2f15d6af6c046cc3b7c909a4a8bf3155b9dd1e6c4ce04612758a763c9086 correspond au headerhistorique. Ne pas confondre absent de l’inventaire versionné et absent du disque. Cette présence historique ne les rend pas sourcecanonique pour une évolution Rust. Clarifier schéma natif durable/production desDTO avant ajout de champs, sans recréer ou modifier un Go ignoré pour fabriquer une autorité de schéma. Lecture seulement, aucun générateur exécuté ni source éditée.

Reprise demandée à la remise sûre Oryh avec fichiers/contrats/ressources stables ; pas de build global ou secondéditeur en attendant. Aucun résultatCIA4 corrigé ni installation nouvelle revendiqué. L’annonce utilisateur de connexion368 a été rapprochée des contrôlesUI déjà reçus : pas de nouvelle navigation/restart.


### Réservation native explicitement confirmée — 2026-11-09T14:17:43.997841+00:00

Remise directe Swarm04/Oryh reçue ; point sûr POC06-TASK-MANDATE.md117… lu. RÉSERVATION NON LIBÉRÉE : sources moteur/Trust/CLI/web et documentation/recette POC06 encore attribuées ; task_validation/Biba/PAEA non modifiés par ce lot mais consommés lors de qualification, donc stables jusqu’à remise. Aucun Cargo/build/test/GLM actif à l’instant du relevé ne vaut libération. Prochaine séquence propriétaire : workflows/compatibilité ciblée, imagefinale, recette native, revue, installation puis remise sûre, ou créneau explicitement coordonné avant mutation.

Le point écrit conserve aussi des observations intermédiaires366/367 : le message actuel du propriétaire corrige le runtime observé à368PID753774/SHA dc14744a… ; révalidation exigée avant son futur redémarrage. Candidat367 préparé seulement, derniers correctifs non qualifiés par lui.78testsPASS/2PAEAignorés rapportés par propriétaire, pas vérification indépendante de ces logs par coordinateur. Aucun mappingnatif/journalTrust nouveau rapporté dansRootactif.

Le raccord Trust simulé ne protège pas les fichiersCI ; il ne clôt pas le contre-exemple CIA2/CIA3. CI-A1 sur copieprivée368 a préservé cette séparation. CI-A2 source/correctif non attribué pendant cette réserve ; conception/preuves remises, autorisation humaine du correctif déjà acquise. Libération finale avec fichiers/contrats et ressources effectifs demandée pour attribution du plus petit incrément CI et refus/reprise sur nouvelles fixtures. Les deux propriétaires ont reçu l’état, pas de nouvelle campagne ni mutation partagée.
