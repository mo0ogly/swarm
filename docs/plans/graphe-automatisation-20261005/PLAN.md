# Plan détaillé pour l’édition graphique et l’automatisation de Swarm

Date : 5 octobre 2026. Statut : proposition documentée, sans modification du moteur, sans création ni lancement de mission. Destinataire : Fabrice Pizzi et les personnes qui réaliseront et examineront les lots.

## Résultat recherché

Permettre à une personne non experte de préparer une mission dans un graphe éditable, de comprendre les rôles et les dépendances, puis d'autoriser une exécution observable. Ajouter ensuite des déclencheurs utiles, sans confier à une seconde application la validation des résultats de Swarm. Le CLI doit offrir les mêmes opérations métier, y compris les prévisualisations et les diagnostics.

Exemple cible : l'utilisateur choisit un modèle de mission, voit un planificateur, les périmètres délégués, les exécutants et le vérificateur. Il relie deux tâches, lit les conséquences, applique la révision, puis lance. S'il programme un départ, le déclencheur soumet une demande au moteur ; il ne force ni les dépendances ni un espace occupé. Une attente indique sa cause, qui agit et ce qui permettra la reprise.

## Existant vérifié et périmètre de recherche

Dans ce checkout, les points suivants ont été ouverts et examinés, sans prétendre avoir recetté leur comportement dans cette analyse :

- `web/graph.js` utilise déjà Dagre, des flèches SVG et les orientations LR/TB. Ajouter Dagre n'est donc pas un nouveau livrable.
- `web/pilotage.js` conserve zoom, sélection, filtres et position de lecture ; `web/pilot-graph.js` construit aussi les responsabilités et les branches repliées.
- `planning.go` contient les opérations de planification et le contrôle des requêtes ; `model.go` contient Work, Task, Event et les identités de tentative.
- `web/planning.js` envoie notamment `schema_version`, `event_id` et `expected_revision` par les opérations publiques.
- `dispatcher.go` décide des départs ; `mission_status.go` projette les conditions et prochaines actions ; `automatic_validation.go` exécute les contrôles.
- `package.json` décrit un frontend JavaScript existant et des recettes Node/Puppeteer. Il ne justifie pas une migration Vue globale.
- Le checkout comporte beaucoup de modifications préexistantes. Les lots devront partir d'un inventaire conservé et identifier leurs propres changements. Aucun rapport ancien accepté ne sera réécrit.

Les sources techniques consultées et leurs licences figurent dans l'annexe [Traçabilité](SOURCES.md). Elles ne démontrent pas les capacités actuelles de Swarm. A01 complète la cartographie avant toute réalisation.

## Choix de conception proposés

1. Conserver le moteur Go comme autorité des transitions, permissions, réservations, budgets et acceptations.
2. Conserver d'abord le frontend actuel et Dagre. Évaluer un composant de canvas indépendant dans un prototype isolé uniquement si les interactions nécessaires sont trop fragiles à réaliser sur cette base.
3. Séparer Préparer, Superviser et Historique. Un geste de déplacement n'a pas d'effet sur une dépendance ; une flèche ajoutée est une proposition de modification métier.
4. Séparer les liens de dépendance, de responsabilité et de retour de rapport. Une flèche de responsabilité ne peut satisfaire un prérequis.
5. Créer des contrôles dans le moteur avant de brancher le geste graphique correspondant.
6. Appliquer les mêmes commandes métier depuis HTTP et CLI. Le frontend ne peut décider seul qu'une dépendance est satisfaite.
7. Introduire l'automatisation progressivement : demande manuelle durable, programmation unique, programmation récurrente, puis événement externe authentifié.
8. Réaliser le noyau dans Swarm. Toute intégration avec une plateforme externe sera une extension optionnelle, après livraison du noyau.
9. Préserver l'instrumentation disponible. Aucun coût inconnu ne sera transformé en zéro et aucune reprise n'effacera la consommation antérieure.
10. Les chiffres de performances et seuils de taille restent des paramètres à calibrer, pas des promesses commerciales.

## Frontière de réutilisation

La lecture du code public sert à dégager comportements, contraintes et cas limites. Le moteur sera conçu à partir des contrats propres à Swarm. Une traduction fonction par fonction de TypeScript vers Go n'est pas une implémentation indépendante par le seul fait du changement de langage.

Les commentaires de code, libellés, aides et notes opérationnelles décrivent le comportement propre à Swarm ; ils ne citent pas de produit d’inspiration. Les références et notices nécessaires restent dans les fichiers de provenance et de licence.

A01 produit un registre des sources : URL, commit, fichiers consultés, licence, idée générale retenue, composants réellement réutilisés et notices. Aucun fichier `.ee.` ni ressource sous licence Enterprise ne sera incorporé. Les licences restrictives et celles des composants tiers sont examinées séparément dans le registre de provenance. Notre licence ne remplace pas celles de ces composants. La traçabilité réduit les risques de confusion, sans constituer une garantie juridique générale.

Pour toute copie envisagée, suspendre cette copie et qualifier son régime avant inclusion. Les fonctionnalités et algorithmes généraux sont distingués de l'expression du code, des textes et des ressources graphiques. Ne pas copier les libellés, assets, logo, CSS ou fichiers de test d'un autre produit pour les renommer.

## Organisation des responsabilités

Le responsable principal conserve l'objectif, les exigences, la synthèse et les arbitrages. Trois périmètres de sous-planification sont proposés : moteur et données ; expérience web et CLI ; contrôles et livraison. Ils ne codent pas et ne peuvent accepter leurs propres résultats.

Les exécutants interviennent uniquement dans les fichiers de leur lot. Un seul écrit dans un checkout partagé. Le parallélisme n'est utilisé qu'avec isolation démontrée et attribution de fichiers. Un rapport est transmis par le moteur au responsable du périmètre ; sa présence dans un dossier ne prouve pas qu'un autre agent l'a lu.

Le vérificateur indépendant examine le même candidat que les contrôles. Il précise ses sources et les contrôles qu'il a lui-même exécutés. La clôture est une décision publique du moteur. Un badge « vérificateur » ne prouve pas l'existence d'une revue réelle.

Pour éviter une mission de réalisation trop large, les 16 lots peuvent être adoptés en quatre tranches de quatre lots. Aucune tranche n'est lancée par ce document. Les tranches restent reliées à ce plan commun ; elles ne servent jamais à contourner un budget épuisé ou à masquer un échec. Les budgets en vigueur sont conservés. Si un plan adopté fixe 2 tentatives, 60 outils par tâche ou 40 appels par rôle, ces plafonds s'appliquent ; aucun chiffre ne sera changé automatiquement.

## Exigences observables

| ID | Déclencheur et précondition | Résultat attendu |
| --- | --- | --- |
| R01 | Nouvelle préparation | Brouillon visible, rôles et responsabilités identifiables, aucun départ implicite |
| R02 | Ajout ou suppression de dépendance | Prévisualisation moteur ; cycle, ID inconnu et doublon refusés avec motif |
| R03 | Révision concurrente avant application | Aucun écrasement ; afficher le conflit et recharger pour nouvelle prévisualisation |
| R04 | Déplacement ou zoom du canvas | Position conservée sans modifier la révision métier ou les preuves |
| R05 | Groupe replié ou filtre actif | Nombre de tâches cachées et dépendances masquées annoncé ; expansion disponible |
| R06 | Sélection d'une tâche ou d'un rôle | Détails, dernière action, modèle demandé/observé et prochaine action accessibles |
| R07 | Annulation d'une édition | Brouillon restauré ; aucun retour arrière des effets exécutés ou preuves acceptées |
| R08 | Commande équivalente CLI/web | Même garde, même code d'erreur, même effet métier et même résultat projeté |
| R09 | Demande de lancement répétée | Même événement ne produit pas un second départ ; résultat précédent récupérable |
| R10 | Déclencheur sans condition de départ | Demande durable en attente ou rejetée, avec cause ; aucune tentative artificielle |
| R11 | Horaire récurrent, pause ou redémarrage | Occurrences identifiées ; règle explicite de rattrapage ; pas de rafale cachée |
| R12 | Événement externe | Authentification, déduplication, taille limitée, journal expurgé et aucun budget implicite |
| R13 | Échec ou résultat incertain | Attribution et action permise ; reprise après changement pertinent vérifié |
| R14 | Preuve sur ancien candidat | Historique conservé ; acceptation actuelle refusée si preuve pertinente périmée |
| R15 | Usage français/anglais et sombre/clair | Textes opérationnels traduits, focus et états lisibles, parcours clavier équivalent |
| R16 | Mise à jour et retour arrière | Données anciennes lisibles, migration contrôlée, binaire installé identifiable |
| R17 | Mission annoncée terminée | Résultats obligatoires acceptés et responsabilité clôturée publiquement |
| R18 | Consommation absente du fournisseur | Valeur inconnue explicitement ; outils, requêtes, jetons et coût distingués |

## Contrats du moteur à définir avant réalisation

### Brouillon et révision du plan

Un brouillon porte un identifiant, sa mission source, la révision de base, les opérations proposées et un digest du contenu. Les opérations initiales sont limitées à l'ajout/retrait d'une dépendance, au déplacement d'une tâche entre périmètres admissibles et à la modification de propriétés explicitement autorisées. Toute extension de ce vocabulaire doit disposer d'un critère et d'un contrôle moteur.

La prévisualisation doit indiquer : opérations normalisées, impact sur descendants, preuves affectées, profils/méthodes hérités, tâches actives concernées et refus. Son jeton lie le contenu, la révision de base et la politique pertinente. L'application revalide les gardes dans une transaction ; un jeton de prévisualisation n'est pas une permission permanente.

La réponse distingue validation de brouillon et application. Les noms d'endpoint et de commande du document CONTRACTS sont proposés : ils doivent être adaptés au vocabulaire public existant, sans implémenter deux autorités concurrentes.

### Identité du candidat et fraîcheur

Distinguer la version du plan métier, la version de la présentation et l'identité du candidat testé. Un zoom ou un nom d'affichage ne doit pas déclencher une production ou invalider tous les contrôles. À l'inverse, une dépendance, un contrôle, un périmètre ou une entrée pertinent modifié impose une analyse d'impact. Un changement de version globale ne peut être simplement ignoré sans contrat de dépendance des preuves.

Lors d'un contrôle long, enregistrer les entrées pertinentes au départ et les revalider au rattachement du reçu. Si elles ont changé, le résultat est conservé mais périmé. Si seul un état administratif sans impact a changé, le contrat peut autoriser le rattachement sans nouvelle suite complète. Ce comportement doit être prouvé, pas seulement annoncé.

### Commandes durables et déduplication

Une demande possède une clé de déduplication, une source, une cible, une politique autorisée et une empreinte du contenu. Même clé + même contenu renvoie l'opération déjà connue. Même clé + contenu différent retourne un conflit. La réponse perdue n'autorise pas à inventer une nouvelle clé pour contourner l'incertitude.

Les identités de demande, occurrence, mission et tentative sont distinctes. La déduplication locale ne garantit pas une exécution exactement une fois chez un fournisseur externe. Après un arrêt entre effet et enregistrement, vérifier l'effet réel avant toute nouvelle action sensible.

### Déclencheurs et concurrence

Politique initiale proposée : un déclencheur cible une mission existante explicitement autorisée et demande sa reprise selon les conditions du moteur. Il ne clone pas automatiquement une mission terminée et ne crée pas une nouvelle autorisation. Une cible déjà clôturée produit un refus explicite, sans nouvelle production. Le lancement répété de nouvelles missions à partir d'un template est différé jusqu'à définition de son budget global et de sa politique de clôture.

États du déclencheur : brouillon, désactivé, activé, suspendu, archivé. États d'occurrence : reçue, dédupliquée, en attente, prise en charge, exécutée, refusée, expirée, annulée. « Exécutée » signifie que l'opération demandée a été traitée, pas que la mission a été validée.

Horaire stocké avec fuseau IANA et instant UTC calculé. Rattrapage initial proposé : aucune occurrence passée au redémarrage ; montrer celles ignorées, permettre une demande manuelle explicite. Vérifier changements d'heure, horloge modifiée, double conducteur et redémarrage après prise en charge. Une occurrence encore en attente peut être annulée ; une action commencée nécessite le mécanisme d'arrêt existant, sans promesse de rollback.

Concurrence initiale : coalescer plusieurs demandes de reprise identiques pour une mission déjà active ; conserver les motifs reçus. Les espaces restent réservés par le moteur. Les politiques « ignorer », « mettre en file » et « démarrer une mission distincte » ne sont pas rendues interchangeables.

### Événements externes et confiance

Introduire les événements externes seulement après commandes durables. Authentifier une signature ou un jeton à portée limitée, vérifier timestamp et rejouement, limiter taille et fréquence, valider les champs autorisés. Ni secret ni contenu non fiable dans un journal public ou un prompt de commande.

Les données externes sont des données : elles ne peuvent relever un budget, changer un fournisseur, activer une skill ou déclencher une commande arbitraire. La première intégration se limite à demander une action préautorisée sur une cible connue. Les appels sortants ne sont pas nécessaires à ce noyau.

## Parcours web prévu

### Préparer

1. Choisir la mission ou le modèle et identifier son espace de travail.
2. Afficher rôles, tâches, prérequis et critères obligatoires.
3. Modifier les propriétés dans un panneau lié à la sélection.
4. Ajouter une dépendance par poignées ou par une liste accessible au clavier.
5. Afficher l'impact et les erreurs après prévisualisation moteur.
6. Corriger, annuler localement ou appliquer la révision.
7. Choisir explicitement Lancer maintenant ou Programmer ; montrer conditions, profils et limites.

### Superviser

Conserver la position de lecture pendant les rafraîchissements. Les cartes portent un rôle, un état de processus et un état de validation séparés. Le vert de succès signifie résultat accepté, pas rôle vérificateur. Les profils et skills indiquent demandé, hérité et effectivement observé lorsque disponible.

Un clic sélectionne ; l'action Ouvrir montre les détails ; le double clic peut ouvrir la session sans modifier la mission. Les comportements doivent être annoncés dans l'aide. Le journal est synchronisé à la sélection, mais une sélection locale ne lance rien. Les détails techniques longs restent accessibles dans une modale à fermeture textuelle et retour du focus.

### Modifier une mission active

Déplacer une carte reste permis car c'est de la présentation. Modifier son contrat ou celui d'une tâche active crée un brouillon ; le moteur refuse ou exige la pause nécessaire selon l'impact. Il ne faut ni bloquer toute la lecture ni appliquer une modification partielle silencieuse.

### Dessiner et lire les liens

Dépendance : prérequis vers dépendant, trait plein et flèche. Responsabilité : responsable vers périmètre/tâche, style et légende distincts. Retour de rapport : remontée vers responsable, accessible dans une couche optionnelle pour éviter la surcharge. La distinction reste textuelle, jamais par couleur seule.

Les modes horizontal/vertical, simplifié/détaillé, repli et filtres existants restent disponibles. Une mini-carte indique le viewport, les groupes et la sélection ; elle ne remplace pas les intitulés des cartes. Le zoom ne doit pas faire disparaître les flèches et les noms essentiels.

## Parcours CLI prévu

Les commandes exactes seront décidées en A02 après inventaire. Le parcours cible comporte : lire/exporter un brouillon, demander une prévisualisation, appliquer avec révision et événement, lister les déclencheurs, prévisualiser une programmation, activer/pause/reprendre, consulter une occurrence et annuler ce qui reste en attente.

Prévoir sortie humaine FR/EN et JSON stable. Un code de sortie doit distinguer succès, entrée invalide, conflit de révision, action non autorisée et opération encore en attente. Aucun avertissement ne doit polluer stdout JSON. Les erreurs affichent la cause et la prochaine commande permise, sans exposer un secret. La CLI interactive ne doit pas lancer sans consentement sous prétexte qu'un champ par défaut est sélectionné.

## Lots de réalisation et dépendances

Chaque lot produit un rapport bref lié aux exigences, aux fichiers changés, aux commandes et au candidat. Les fichiers proposés sont des points d'ancrage, pas des autorisations de modifier des fichiers détenus par un agent actif. Les noms nouveaux seront stabilisés en A02.

### Tranche A Cadrage et contrats

**A01 Cartographie et sources** — dépendances : aucune ; responsable moteur/données, exécutant d'analyse.

1. Inventorier branche, diff préexistant, binaire installé, schéma et capacités publiques.
2. Reproduire un parcours lecture → prévisualisation → modification sur un stockage temporaire.
3. Cartographier frontend, CLI et moteur sans prétendre qu'une fonction nommée est recettée.
4. Classer les capacités envisagées en déjà présent, amélioration, nouvelle capacité ou hors périmètre.
5. Établir le registre de licences et provenance, figer les révisions des sources consultées.
6. Photographier FR/EN et sombre/clair sur un exemple isolé, sans secrets.

Livrables : inventaire, baseline de comportement, registre de sources et lacunes vérifiées. Critères : R01/R08/R15/R16/R18 ont des observations de référence. Échec : environnement indisponible → documenter et réparer la précondition, sans lancer une mission réelle pour compléter l'inventaire.

**A02 Contrats de plan et de commandes** — dépendance A01 ; responsable moteur/données.

1. Décider la séparation brouillon, plan et présentation.
2. Définir opérations, erreurs, idempotence et prévisualisation liée à la révision.
3. Définir garde de rôle, attribution de périmètre et garde des tâches actives.
4. Décrire l'impact sur profils, skills, preuves et dépendances héritées.
5. Définir les routes/commandes en réutilisant les contrats publics existants.
6. Écrire les cas adverses avant de réaliser : cycle, doublon, conflit, preuve périmée.

Livrables : ADR court, exemples JSON, matrice de transitions. Critères R02/R03/R07/R08/R09/R14 ; aucune ambiguïté entre déplacement graphique et mutation métier. Refus documenté plutôt qu'ajout implicite d'une permission.

**A03 Maquettes et décision du canvas** — dépendance A02 ; responsable web/CLI.

1. Maquetter Préparer, Superviser, Historique et Programmes.
2. Conserver flèches, orientations, rôles et repli actuels.
3. Prototyper déplacement, poignée de connexion, mini-carte et navigation clavier sur fixture.
4. Mesurer poids/bundle, CSP, chargement hors réseau, focus et stabilité des rafraîchissements.
5. Comparer maintien SVG/Dagre et composant indépendant ; conserver une réalisation indépendante.
6. Décider avec preuves ; une migration Vue entière demande un ADR distinct et ne devient pas un détail du lot.

Livrables : prototype jetable, captures quatre variantes et décision. Critères R04/R05/R06/R15. Si le prototype ne garde pas les flèches lisibles, revenir à la base actuelle avant d'ajouter des interactions.

**A04 Audit du plan et tests discriminants** — dépendances A02/A03 ; contexte de revue indépendant lors de l'exécution.

1. Examiner limites, attribution, hypothèses et exclusions.
2. Vérifier le graphe des lots et les conflits de fichiers.
3. Associer chaque exigence à un contrôle comportemental et son environnement.
4. Prévoir une régression volontaire montrant que chaque contrôle critique détecte l'absence de l'effet.
5. Séparer fixtures, tests fournisseur et critères de livraison.
6. Adopter la tranche B seulement après fermeture des ambiguïtés bloquantes.

Livrables : avis de préparation, matrice test/risque et liste des décisions adoptées. Ce lot n'accepte aucune fonctionnalité produit.

### Tranche B Édition et pilotage

**B01 Service Go de brouillon et révision** — dépendance A04 ; ancrages planning.go/model.go et services publics existants.

1. Implémenter DTO validés et service commun aux interfaces.
2. Vérifier l'ensemble du graphe et le périmètre, pas seulement l'arête ajoutée.
3. Prévisualiser impact et permissions ; appliquer atomiquement après nouvelle vérification.
4. Garantir clés d'événement stables et non-écrasement concurrent.
5. Traiter interruption avant/après publication et lecture des anciennes missions.
6. Écrire tests de transitions, concurrence et persistance avec root temporaire.

Critères R02/R03/R09/R14/R16. Contrôles : cas Go ciblés, race pour état partagé, simulation de conflit HTTP/CLI ; jamais données de mission active. Un rejet laisse le plan antérieur complet et lisible.

**B02 CLI du brouillon** — dépendance B01 ; ancrages main.go/console_cli.go.

1. Brancher commandes sur le service B01, sans duplication des gardes.
2. Ajouter import/export bornés et validation des fichiers d'entrée.
3. Produire JSON stable, codes d'erreur et aide FR/EN.
4. Montrer prévisualisation et différences avant application.
5. Fournir une opération équivalente au geste clavier web.
6. Recetter avec véritable binaire dans stockage isolé.

Critères R07/R08/R15. Aucun retour arrière ne réécrit une tentative ou un rapport ; annuler une révision déjà appliquée est une nouvelle proposition, sous conditions actuelles.

**B03 Éditeur web** — dépendances B01/B02 ; ancrages web/pilotage.js, web/graph.js, web/planning.js.

1. Créer le mode Préparer et le panneau de propriétés.
2. Ajouter gestes de connexion et alternatives clavier/listes.
3. Conserver l'historique local de brouillon avec annuler/rétablir.
4. Afficher aperçu des effets et erreurs moteur ; traiter le conflit sans écraser.
5. Conserver viewport, sélection et position lors des changements d'état.
6. Ajouter mini-carte et groupes ; vérifier que filtre/repli annoncent ce qui est caché.
7. Faire appliquer le plan publiquement ; annoncer le résultat réel.

Critères R01–R08/R15. Recette navigateur quatre variantes, clavier, rafraîchissement et changement concurrent. Les données simulées sont indiquées comme telles ; aucune preuve fictive acceptée dans une mission réelle.

**B04 Vue du travail et fraîcheur des preuves** — dépendance B03 ; ancrages pilotage, mission_status et projection des preuves.

1. Synchroniser sélection, journal et vue de tentative.
2. Distinguer rôle, processus et validation avec textes/icônes/couleurs sémantiques.
3. Montrer cause, acteur et prochaine action pour les attentes.
4. Distinguer coûts inconnus, paramètres demandés et modèle observé.
5. Tester qu'un changement purement visuel n'invalide pas une preuve.
6. Tester qu'une modification pertinente invalide uniquement selon un contrat démontré, sans accepter un résultat périmé.

Critères R04/R06/R13/R14/R18. Inclure un contrôle long et une modification administrative concurrente. Si la séparation des versions ne peut être prouvée, conserver le refus prudent, indiquer la limite et ne pas afficher une autonomie acquise.

### Tranche C Automatisation durable

**C01 File de demandes et occurrences** — dépendance B04 ; ancrages dispatcher/mission_status/store selon cartographie A01.

1. Implémenter réception, identité et statut durable d'une demande.
2. Dédupliquer même contenu ; refuser clé identique et contenu différent.
3. Réserver/attribuer la prise en charge par un conducteur identifié.
4. Traiter deux conducteurs, arrêt brutal, reprise et incertitude d'effet.
5. Enregistrer événements et consommation déjà connue sans remboursement automatique.
6. Montrer une attente d'espace comme attente, sans lancer un agent voué au refus.

Critères R09/R10/R13/R16/R18. Contrôles transaction/race et scénarios de restart. Ne pas promettre l'exactement-une-fois externe.

**C02 Programmation et reprise conditionnelle** — dépendance C01.

1. Définir programmation unique puis récurrente et fuseaux.
2. Prévisualiser prochaines occurrences et politique de rattrapage.
3. Ajouter activation, pause, expiration et archivage des déclencheurs.
4. Coalescer demandes sur mission active ; pas de nouvelle mission implicite.
5. Classifier incident fournisseur, environnement, code, espace, validation et décision manquante.
6. Autoriser reprise causale dans les limites existantes ; préserver cooldown et historique.
7. Ajouter horloge injectable pour tester DST, décalage et redémarrage sans attente réelle longue.

Critères R10/R11/R13. Une indisponibilité fournisseur persistante n'est pas effacée par l'expiration d'un délai. Les limites et coûts restent celles de l'autorisation explicite.

**C03 Événement externe contrôlé** — dépendance C02.

1. Ajouter endpoint minimal authentifié et scope de cible.
2. Valider signature/timestamp/clé d'événement avec limite de taille.
3. Refuser cibles inconnues, replay hors contrat et demandes non autorisées.
4. Stocker secret par mécanisme existant sécurisé et permettre rotation/révocation.
5. Journaliser l'acceptation de demande, sans contenu sensible.
6. Tester doublons, conflit de contenu, mauvaise signature, rafale, secret révoqué et ordre d'arrivée.

Critères R09/R12. Contrôle à l'entrée et à la prise en charge. Aucune concaténation de contenu reçu dans une commande système. Connecteurs complets avec des services externes différés.

**C04 Administration web et CLI** — dépendance C03.

1. Lister déclencheurs avec état, prochaine occurrence et dernière action.
2. Exposer prévisualisation, activation, pause et annulation admissible.
3. Montrer budget consommé/disponible ou inconnu, profils hérités et autorisations.
4. Afficher le traitement réel de l'occurrence et l'état indépendant de la mission.
5. Préparer une aide modale lisible, exemple et alternative CLI.
6. Tester changement de fuseau, permissions, navigation, quatre variantes et conflit de révision.

Critères R08/R10–R12/R15/R18. Aucun bouton « relancer tout » ne masque les préconditions ou ne relève les budgets. Notifications par défaut seulement sur changement significatif, fin, refus nécessitant une décision ou échec ; pas de rafale à chaque vérification.

### Tranche D Recette et livraison

**D01 Intégration de bout en bout** — dépendances B04/C04.

1. Installer un candidat canonique dans root isolé.
2. Préparer par UX une mission de fixture à rôles réels configurés.
3. Ajouter une dépendance, provoquer un cycle et corriger.
4. Appliquer, lancer, sélectionner un agent et consulter le journal.
5. Programmer un départ ; injecter doublon, conflit, attente d'espace et restart.
6. Reprendre après correction démontrée ; vérifier même mission et historique.
7. Tester réel fournisseur borné seulement après capacité, coût et autorisation établis ; ne pas utiliser une fixture comme preuve d'autonomie fournisseur.

Critères : couverture R01–R18 ; fiche des écarts et limites. Aucun appel direct SQLite ; setup par opérations publiques et fixtures isolées documentées.

**D02 Documentation et migration** — dépendances D01.

1. Documenter installation, mise à jour, droits et stockage.
2. Mettre à jour guides FR/EN, aide CLI et aide web avec termes utilisateurs.
3. Capturer graphe, préparation, conflits, journal et programmations sur le candidat.
4. Documenter compatibilité, sauvegarde et rollback, y compris occurrences durables.
5. Mettre à jour version/historique et licences des dépendances réellement ajoutées.
6. Vérifier chemins, commandes, liens, images et absence de données privées.

Critères R15/R16 ; captures des deux langues et deux thèmes ; distinction feature en option et installée. Aucun « tout fonctionne » pour un parcours non recetté.

**D03 Contrôles finaux et revue indépendante** — dépendance D02.

1. Figer manifeste du candidat et portée des contrôles.
2. Exécuter tests ciblés manquants, puis suite finale pertinente une fois ; conserver tous les codes de sortie.
3. Exécuter go test/go vet/race selon changements, frontend et navigateurs ; config et diff ciblé.
4. Fournir au reviewer les fichiers pertinents, rapports et reçus, pas seulement des noms de fichiers.
5. Revalider l'identité au rattachement et à l'acceptation.
6. Si défaut concret, corriger le périmètre concerné puis rejouer ce qui a changé ; aucune boucle de full-suite pour un simple changement administratif.

Critères R14/R17 ; aucune acceptation par manipulation de DB. La revue du plan écrite pendant sa préparation n'est pas cette revue indépendante.

**D04 Livraison autorisée et RETEX** — dépendance D03.

1. Confirmer clôture publique et fonctions réellement installées.
2. Préparer livraison, notes de version et preuve de rollback.
3. Commit/push/release uniquement si autorisés pour cette livraison ; ce plan ne les autorise pas.
4. Mettre à jour le serveur seulement après absence d'agents/contrôles actifs et avec sauvegarde prévue.
5. Vérifier version visible CLI/web, actifs sans cache périmé et parcours de navigation ciblé.
6. Produire RETEX avec attribution moteur, agent, fournisseur, environnement, test et supervision.

Mesures : délai jusqu'à acceptation, temps d'attente par cause, tentatives supplémentaires, contrôles répétés, interventions humaines, outils/requêtes/jetons et coûts rapportés séparés. Comparaison avec baseline à mission comparable, sans inventer de coût absent. Définir les actions restantes, responsables et preuves attendues au lieu de promettre un résultat universel.

## Conditions d’arrêt et retour arrière

- Une preuve de résultat supprimée ou une garde contournée arrête l'adoption du lot.
- Une incompatibilité de licence bloque seulement l'incorporation concernée ; une implémentation indépendante reste possible.
- Une régression du graphe, des flèches, du clavier ou d'un thème impose retour au renderer précédent avant déploiement.
- Une erreur d'environnement ou de recette ne consomme pas automatiquement une nouvelle production ; diagnostiquer son propriétaire.
- Ne jamais modifier les fichiers détenus par un agent actif, réinitialiser une tentative ou créer une mission de remplacement pour échapper à un plafond.
- Migration : sauvegarde cohérente, ancienne version de lecture validée, nouveau schéma testé sur copie ; interdire rollback binaire si le schéma n'est pas rétrocompatible. Dans ce cas restaurer seulement la sauvegarde avec procédure explicite et occurrences mises hors départ pour éviter doublons.
- Pause d'un déclencheur arrête les futures demandes ; elle n'arrête pas magiquement l'agent actif. Afficher cette portée.

## Prochaine étape concrète

Réaliser A01 puis adopter A02 et A03 avec leurs preuves. Ne pas démarrer immédiatement les 16 lots, ne pas lancer une nouvelle mission avec des contrôles génériques check/diff comme seule preuve. Avant toute exécution, figer la révision source, les rôles réellement configurés, les contrôles de chaque lot et les budgets déjà autorisés.

## Documents associés

- [Contrats et exemples proposés](CONTRACTS.md).
- [Scénarios de recette](RECETTE.md).
- [Audit de cette spécification](AUDIT.md).
- [Sources et licences](SOURCES.md).
- [Architecture Mermaid](architecture.mmd) et [enchaînement des lots](etapes.mmd).
