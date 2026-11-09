# Lecture complète d’une mission historique — 8 novembre 2026

Demande : corriger le refus, sans relancer les agents ni perdre l’historique ;
vérifier CLI/web, conserver les preuves et installer. Codex seul, debug/replan/
verify-fix. État initial358 : mission-root-unavailable, cause
managed-storage-binding-mismatch ; les métadonnées agents restent lisibles.

Cause observée : référence sous /home/fpizzi/workspace/swarm et Root canonique
/home/fpizzi/Documents/ChatGPT/swarm. Même répertoire physique, mais garde lexical
dans managed_repository::observe et managed_repository_worker::with_storage.
49 tables indexées en lecture seule par hash de lignes, Work et manifestes
originaux empreintés. Aucun contenu privé de table n’est copié dans le rapport.

Décision : corriger le contrat partagé, pas migrer/réécrire les anciens chemins
et preuves ni introduire une exception de mission. with_storage_binding conserve
descripteurs des deux états privés et dossiers Work, exige dev/inode identiques,
traverse les enfants sans symlinks et garde manifestes/reçus/commits inchangés.
Autre datastore, alias disparu, divergence de manifeste : refus et restauration.
Les identités retenues ne constituent pas un confinement same-UID.

Le témoin Rust réel a d’abord échoué sur une précondition de fixture : le helper
Git du produit supprime la configuration globale, donc ne pouvait lire l’identité
de commit qui y est configurée. Lecture de configuration propre à la fixture
corrigée, garde Git du produit intact. Le deuxième essai atteint le témoin
canonique valide puis reproduit managed-storage-binding-mismatch sur l’alias.
Les deux échecs sont conservés, aucun assimilé à un succès. Autre incident de
méthode : indexeur SQL initial refusant un log historique non UTF8 ; corrigé en
hachage binaire, sans décoder ou modifier la donnée originale.

Preuves en cours : .cache/historical-mission-read-20261108. Résultats de correction,
contrôles, parcours réel, installation et conservation à compléter après observation.

Deuxième refus réel, candidat normal359 : OpenFile/ENOENT sur le verrou. Le
producteur Go conservé f32b9f8fa:internal/engine/managed_git.go utilise
operation.lock ; le lecteur Rust attendait le verrou de réglages. Nouveau contrat
reserve_managed : conserver chaque verrou existant, refus strict sauf ENOENT
typé, aucun fichier créé en lecture. La préparation neuve garde son format Rust.
Tests après correction : 4/4 managed_repository::tests, dont alias réel, refus
d’autre datastore, absence, divergence puis reprise et format Go sans création.
Test de verrous séparé : résultat dans lock-tests.stdout. Une compilation lancée
après échec d’un script d’édition a été interrompue ; sortie130 conservée dans
unit-lock-after, exclue des preuves. L’essai corrigé unit-lock-after-02 passe.

Le candidat360 refuse ensuite managed-worker-proof-read/ENOENT : les originaux
Go copy-*.json ne sont pas les reçus native-worker-*.json Rust. Des branches de
revues/reprises historiques restent aussi explicitement non portées. La parité
complète de `mission status` n’est donc pas déclarée corrigée. Décision : séparer
la consultation des données enregistrées de leur qualification actuelle, plutôt
que contourner les contrôles de preuve. Le cockpit offre une lecture historique
explicite, par défaut pour une mission en pause, avec toutes les données de chaque
tâche dépliables ; le statut actuel reste accessible séparément avec ses refus.
Les deux premières corrections restent utiles au contrat partagé. Aucun reçu
historique n’est transformé en preuve Rust ; aucun état accepted n’est présenté
comme une validation actuelle. La lecture CLI work show refuse elle aussi managed-review-fragment-protocol-not-ported.
La lecture publique work list passe : ses huit objets Task sont structurellement
identiques aux Task du Work original, dont dix tentatives et sept revues antérieures.
C’est le producteur réellement consommé par le cockpit ; les agents et journaux
gardent leurs lecteurs publics.

La recette permanente historical-mission-recorded-read couvre le DOM isolé et
les deux nouveaux témoins Rust dans le SDK retenu ; managed-repository-bootstrap
couvre le consommateur public de préparation, refus et reprise. Elle ne qualifie
ni le SDK ni la production à elle seule : profil normal et navigateur réel requis.
Le premier test DOM a échoué sur un double incomplet (méthode render absente) ;
précondition corrigée puis reproduction du champ historique absent, conservées.
Le problème du contrôle navigateur après reprise a été corrigé par une remise à
zéro de son seul REPL ; l’onglet et la connexion utilisateur sont conservés.

## Vérification et livraison

Recettes history (2 étapes, DOM + deux tests Rust nommés) et bootstrap public
(7 assertions) : PASS et intégrité VERIFIED. Les 4 tests du module managed
passent aussi. Profil normal sans test-fixtures : candidat361,
SHA256 98d32027dae6d285f9607767e806471125a4838392c72723d119ce626f72b3c9.
Installation par restart --prepared-build, aucun nouvel input installé hors candidat.
/proc/1930645/exe correspond à ce SHA ; Version réelle361, base Git
d084c1487726f0d240de84d521d19d14d7938250 et modifications non commitées visibles.
Évolutions FR/EN lues dans le dialogue réel. Pas de publication ni acceptation.

Navigateur authentifié réel : historique reçu FR/EN, clair/sombre ; huit tâches,
dix tentatives, sept revues antérieures ; dépliage effectif et texte préformaté
avec retour à la ligne. À 320px en anglais sombre, pas de débordement horizontal
observé. Statut actuel : refus visible, historique retiré ; retour à l’historique
réussi. Compteurs de validité actuelle et d’activité inconnus. Aucune erreur
console observée sur ce parcours. Observations interactives conservées dans
browser-observations.json, distinctes d’un test navigateur autonome.

Après CLI, installation et navigation : les empreintes des 49 tables, Work et
manifestes sont toutes identiques au témoin initial. Contrôle public paused=false
conservé (l’arrêt historique des agents n’implique pas paused=true) ; aucune
opération de relance, reprise, revue, acceptation ou modification de budget.

## RETEX et limite restante

Cause générale : une consultation de données enregistrées dépendait de preuves
d’exécution d’un autre format puis d’une qualification actuelle non portée. Deux
compatibilités physiques sont corrigées sans réécriture (alias et verrous), et la
consultation publique est séparée explicitement du calcul actuel. Les contrôles
de relance et d’acceptation ne sont pas affaiblis. Leçon : qualifier le consommateur
réel et ses producteurs avant de promettre une lecture complète ; un premier
refus corrigé peut seulement révéler la frontière suivante. Témoin valide, refus
originaux et reprise restent conservés, sans transformer les incidents de fixture
ou une compilation interrompue en résultats fonctionnels.

Reste ouvert, propriétaire migration Rust : parité de qualification actuelle
des reçus Go, fragments de revue, reprises et anciennes acceptations. Condition
de reprise : contrat documenté des originaux Go et fixtures positives/refus/reprise
portant leur chaîne complète, sans lancement de cette mission ni adoption implicite.
La livraison permet la consultation historique ; elle ne clôt pas cette parité.
Documentation FR/EN persistante et recette maintenue ; aucun contexte embarqué
d’agent n’est modifié, aucun ancien processus d’agent n’est relancé.
