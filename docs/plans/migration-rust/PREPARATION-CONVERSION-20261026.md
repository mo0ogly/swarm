# Conversion autonome Rust sans départ — 26 octobre 2026

## Besoin et décisions

Reprise après commit50a972330 de l’interface manuelle. L’index d’autrui est préservé :
108entrées, modes et blobs comparés avant/après commit. Aucun push.
APEX/replan et deux appels directs GLM SSE autorisés, sans plafond client ou retry.
Le lot cible le parcours complet préparation autonome → nouveau travail/tâches,
avec aperçu local explicite puis confirmation, aucun lancement. Travail existant,
organisation, révision et libération des départs restent refusés explicitement.

## Contrats et politiques

create-missions exige version1, identités et événement, CAS préparation,
expected_work_revision=0 et hash exact du plan. Projection de fraîcheur, verdict,
méthode/brief/plan puis limites courantes Admin de document, titre, plan et tâches.
Aucun nouveau réglage ou plafond opérateur. Les valeurs de version/révision1,
released=0 et launch_held=true expriment ce contrat sans départ ; les identifiants
hachés, noms de gates et critère global reprennent le protocole Go. Les nombres
des recettes sont des entrées de fixture, jamais de nouveaux defaults produit.

La transaction réservée conserve Work, tâches topologiques, provenance produit,
verrous, événement, préparation et reçu. Relecture finale après le reçu, puis
commit ; le wrapper existant conserve les causes rollback/commit/close et les
issues enregistrées ou incertaines. Une erreur ne prouve pas une absence d’effet.
Rejouer le même événement, jamais recréer un événement après perte de réponse.
La création ne configure aucune équipe, aucun budget ni autorisation de départ.
L’interface affiche le reçu comme observation ; l’état courant appartient au cockpit.

## Replans conservés

La première compilation réussit mais son inventaire refuse des ressources PAEA
ajoutées en parallèle (paea-lab.html/js). Ce candidat sert seulement au diagnostic ;
la qualification suivante inclut les véritables entrées. Aucun contrôle assoupli.
La première recette suppose un Work brut ; le vrai producteur renvoie {work:...}.
L’assertion corrigée consomme cette enveloppe, sans modifier les données.

Le départ verrouillé refuse correctement mais le code public était seulement
format, masquant LaunchLocked. La correction générique conserve désormais
preparation-launch-locked ; le test public exige ce diagnostic et aucune mutation.
Le dépassement de méthode conserve plusieurs causes sous root-cleanup ; la recette
exige aussi preparation-stale-plan dans la chaîne, sans supprimer les causes.

Les triggers ajoutés avant ouverture sont refusés par le schéma Root. Les deux
injections Python vérifient cette frontière puis la reprise après retrait de la
seule fixture ; elles ne prouvent pas la relecture interne. Le test interne
late_receipt_trigger_rolls_back_and_recovers injecte après ouverture de sa connexion,
avec un témoin valide avant chaque altération (Work, lock, event). Il vérifie refus,
rollback intégral et reprise. Aucun scénario d’exploitation publique n’est déduit.
Le premier essai privé tardif était bloqué par root-schema-mismatch : il reste
conservé comme preuve de ce refus, jamais comme succès d’attaque.

La comparaison Go révèle le DFS d’auteur/dépendances dans validateActionPlan.
Le premier balayage Rust produisait un autre ordre avec une branche indépendante.
Le contre-exemple trois tâches échoue sur109 ; un DFS itératif restaure l’ordre
historique sans ajouter de limite de récursion. Les IDs/reçus sont concernés.

## Vérifications et état

Les recettes109/110 et leurs limites restent historiques. La clôture finale
ci-dessous distingue binaire testé, sources réutilisées et processus installé.
105assertions de préparation existante et47contrôles de conversion passent sur110.
Le correctif de libellé postérieur exige une livraison distincte. Le test interne
passe (1test nommé) ; sa portée Witness reste identique après le seul ajout DFS.
Les preuves brutes sont conservées dans le dossier privé de ce lot.

## Suites concrètes

Owner Codex : raccord d’un travail existant et de l’organisation, release-plan
explicite sous les droits courants, puis révisions et dialogue IA. Aucune de ces
fonctions n’est considérée migrée par le lot actuel. L’installation vivante garde
ses réglages41 et ses missions suspendues ; activation/adoption Admin explicite.
La qualification globale et l’acceptation utilisateur restent ouvertes.

## Revue GLM qualifiée

Deux réponses SSE complètes : analyse328s/48534tokens déclarés, revue1003s/44570,
soit93104tokens déclarés. Aucun retry/cap/effort réduit ; modèle demandé/déclaré
glm-5.2, backend non établi. La revue couvre quatre fichiers fournis, avant les
correctifs Witness/DFS ultérieurs : elle ne vaut pas avis indépendant final.

D1 : les cycles sont déjà refusés par preparation_plan::validate (l’inférence
contraire du reviewer est écartée), mais des familles pré-écriture manquaient
au déblocage UI. Reproduction sur111 : refus déterministe puis éditeur bloqué ;
les codes statiques work/task-invalid et les gardes de matérialisation libèrent
désormais la saisie. Les causes SQL/IO/commit inconnues gardent le même événement.
Le test navigateur couvre refus sans publication puis nouvel événement, distinct
du rejeu obligatoire après publication incertaine.
D2 : l’hypothèse de non-append est réfutée par apply_task_addition_with_defaults,
qui push exactement la tâche ; la recette observe toutes les identités/verrous.
D3 : preparation_project relit effectivement catalogue et hashes ; la recette
ajoute une modification réelle du fichier de méthode, refus sans mutation et
restauration de cette seule fixture avant reprise.
D4 : créer un brouillon lié à un travail existant reste un contrat Go utile ;
seule sa conversion est hors lot, refus déjà testé. Aucun contrat antérieur retiré.
D5/D6 : contrôles cachés en absence de document et dialogue modal/navigable
empêchent les chemins avancés ; pas de défaut atteint ni correction inventée.
D7 : les avertissements de réglages facultatifs répétés sont une amélioration UX
retenue (owner Codex, prochain lot de retours d’erreurs ; critère : un avertissement
stable, effacé après récupération). Elle ne change ni reçu ni autorité du lot.
D8 : JSON malformé reste volontairement une issue incertaine, jamais un succès vide
ou une preuve d’absence de mutation. Diagnostic plus précis retenu pour ce même
lot UX, sans assouplir le rejeu. Les 105assertions couvrent déjà cette prudence.

RETEX méthode : une revue statique doit distinguer interfaces fournies, inconnues
et réellement contredites par les producteurs. Ne pas ajouter de branches de
sécurité sur la seule supposition d’un module absent. Un contexte devenu ancien
ne qualifie pas les derniers octets ; les vérifications principales les rattachent
aux binaires retenus. Les originaux et les remarques reportées restent conservés.

Le contrôle sémantique final des Rustdoc corrige aussi les commentaires de sortie :
materialize retourne un Witness à vérifier après le reçu (ne le vérifie pas seul),
et LaunchLocked est une exception délibérée à l’ancien code format Go. Cette
correction documentaire invalide le profil source, même sans changement logique.
Les reprises futures doivent clore cette revue de contrats avant le build pour
éviter ces reconstructions tardives ; la présence de Rustdoc ne prouve pas sa justesse.

## Clôture et limites exactes

Livraison active v0.1.1-dev.115 ; SHA256 3635bd296a419b7d7c8f5c845a5caa87efea689b028358ec228c43c36d9fae62.
477entrées effectives du profil normal. Le lanceur a reconstruit115 pour le nouveau
commit Git079635b2 ; ses entrées sont identiques à114. La comparaison113→114
établit uniquement trois modifications de Rustdoc, sans autre ligne de code.
Les notes sont cohérentes : l’ancien item manuel ne dit plus que toute conversion
est indisponible. Le build de notes initial refuse version_history.json modifié
par une livraison parallèle ; le candidat qualifié suivant est conservé.

50contrôles CLI/API et51assertions navigateur FR/EN, etat/sombre, desktop/mobile,
dialogues/focus, refus connu et publication incertaine passent sur113 exact.
105assertions de non-régression de préparation passent sur112. Les différences
113 depuis112 sont les notes et trois modules de publication/revue gérées d’autrui ;
le parcours conversion est retesté sur113, sans revendiquer leur qualification.
114/115 ne changent ensuite aucune logique de ces parcours. Les preuves sont
réutilisées avec les comparaisons exactes, pas attribuées à une autre exécution.
Le test Witness (1test nommé) est réutilisé sur son suffixe source byte-identique.
Intégrités des recettes retenues vérifiées, captures desktop/mobile/dialogue inspectées.

Sur115 :16contrôles HTTP/processus/ressources,14Version FR/EN et6lecture de la page
installée PASS. Config schema41 et missions pause révisions50/469 conservées.
La préparation vivante exige toujours adoption/activation explicite Admin.
Aucun appel fournisseur métier, aucune mission native lancée ou ancienne reprise.

Deux défauts de méthode restent tracés : fichier candidat éphémère consommé par
une installation parallèle ; contrôle enchaîné malgré sa copie échouée. Le SHA du
processus/artefact immutable remplace cette dépendance au nom candidat. Les sorties
sans prérequis établi sont conservées comme FAIL ; aucune ne devient PASS. Les
séquences dépendantes doivent partager un arrêt sur échec, y compris entre appels
d’outils, et attendre la fin réelle du restart avant leur vérification ; le test
contre114 a ainsi refusé le nouveau115, puis un contrôle frais cible115 exact.
Le code de ces contrôles et leur contexte sont retenus dans la preuve privée.

Les remarques D7/D8 de présentation d’erreurs restent des suites UX explicites.
Travail existant, organisation, release/révision et dialogue IA sont encore ouverts,
owner Codex. Aucun pourcentage global, équivalence Go complète ou acceptation
indépendante/utilisateur n’est déduit de ce lot. Le seul commit créé ici est
50a972330 pour le lot précédent ; aucun push, aucune attribution d’autrui inventée.

## Conservation et espace disque

Les preuves sont conservées hors cache sous evidence/20261026-preparation-conversion/private.
Le volume n’avait plus que1,6Go disponibles après les constructions. La seule copie
temporaire .cache/preparation-conversion-next est supprimée après vérification
SHA256 de chaque fichier contre la copie retenue et absence d’exécutable actif
dans ce dossier. Les binaires servis, caches d’autrui et preuves retenues ne sont
pas supprimés. cache-retention.json conserve la correspondance ; les chemins
absolus historiques ne sont pas réécrits. Les recettes se rejouent en choisissant
le binaire conservé (private/campaign/candidate115) ou un nouveau profil qualifié.

## Reprise avant commit — portée de plateforme

preparation_read expose sa projection sous cfg Linux, comme le writer. Le module
preparation_conversion est désormais déclaré sous la même condition. Cette
correction de déclaration conserve le parcours Linux testé et évite de consommer
une interface Linux sur les autres cibles. Aucune compilation Windows n’est
revendiquée. La relance suivante reconstruit le profil livré et vérifie son identité.
