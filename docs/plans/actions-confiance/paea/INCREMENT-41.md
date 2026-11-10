# PAEA — incrément41 : réacquittement de durabilité sous pin

40paquet opérateur12tests/Rustdoc/revue vérifiés ; livraison40 ouverte. Trois
builds normaux refusés, originaux40 conservés : changements de sources concurrents,
deux références RootInstanceIdentity corrigées ensuite par leur propriétaire,
puis nouvelles entrées API/worker/web changées pendant compilation. Aucun candidat
158 non qualifié activé ; serveur157 dernier qualifié. Travail indépendant Codex.

Avant de publier des fichiers de service, un paquet relu après Visible/Unknown ne
doit pas être traité comme durable par simple présence/read. Comparaison : réécrire
les mêmes octets par CAS (rename/objet remplacé inutile) ou fsync du fichier détenu
et de son dossier sous réservation après vérification du pin. Choix deuxième.

read_private_durable(name,expected_sha256) complète le parcours39, sans modifier
read_private d'observation. Pin canonique avant IO ; EOF/métadonnées puis digest
exact avant fsync fichier, puis fsync dossier en gardant le FD, contrôle final et
fermeture unique. Erreur conserve validation (tentative/octet/pin) et durabilité
non acquittée/fichier acquitté/dossier acquitté, y compris après close/cancel.
Aucune durabilité matérielle universelle, autorité humaine ou fraîcheur déduite.

PackageStore::load_durable est une demande mécanique explicite, pas politique
opérateur cachée : même pin extérieur, décodage exact, clés retenues jusqu'à fin
de réservation. ReadError garde la durabilité connue, même après décodage/échec
de libération. load historique ne fait toujours aucun fsync implicite. Aucun
plafond/délai/retry/KDF/droit ajouté. Tests d'abord : témoin réel, mauvais pin
avant sync, ancienne lecture inchangée sous faute fsync, FileSync/DirectorySync/
ReadClose/Unlock/reprise avec phases exactes et pas de retour de clé en échec.

Revue politiques/fonctions/erreurs/FR-EN/recette et profils requise. Sources config
partagées changées : requalifier lecteurs et consommateurs concernés, sans suite
globale sans raison. Publication effective des fichiers/journaux reste42, avec
séparation physique des objets vault/service et clé métier hors service. PA02partiel,
0/17/E2/E3/PA13/rotation/confinement ouverts, aucun commit/push/GLM/délégation.


## État de livraison41

67tests/9étapes PASS stables, Rustdoc privé deux crates PASS. Profils normaux159/160 qualifiés, mêmes sources PAEA que les tests ;7CLI sur159 puis7sur160 PASS.160activé par parcours supporté, PID/SHA vérifiés. UI01 :0contrôle, ancienPID disparu après redémarrage concurrent pendant acquisition native. Statut frais après natif et mêmes octets ELF réutilisés ; UI02 :3contrôles puis refus des ressources actuelles. HTTP protégé confirme i18n-en servi identique au profil160, différent du workspace modifié avec les notes. Redémarrage frais162 refusé : provider_wait.rs/provider_wait_store.rs changés pendant compilation, aucun candidat non qualifié activé. Originaux/profils/tests/diagnostics/revue des octets conservés. Livraison conforme aux ressources actuelles et56UI reste OUVERTE ; ni contournement du build ni affaiblissement de la recette. Continuer42artefact destiné au service uniquement, puis publication43. Amélioration conservée : statut lecture derrière lifecycle.lock bloque lors des builds ; diagnostic établi, aucun correctif de concurrence revendiqué.
