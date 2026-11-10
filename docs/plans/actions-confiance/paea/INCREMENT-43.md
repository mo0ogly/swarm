# PAEA — incrément43 : publication privée des fichiers de service

Reprise : codec42 et noyaux40/41 livrés sur dev165,18 tests bibliothèque,7 CLI,
56 contrôles navigateur + refus de script, profils et intégrité vérifiés. Codex
direct seul, sans GLM, délégation, commit ou push. Le service personnel n'existe
pas encore ; PA02 et 0/17 invariants globaux restent ouverts.

Contrat choisi : entrée ServiceArtifact42 et pin indépendant, UIDservice doit être
l'euid réel ; parent privé déjà existant. Créer exclusivement une racine de stage
fixe voisine de la destination, sous descripteurs retenus. Six sous-dossiers privés
séparés : identity, business, personal, configuration, configuration-witness,
personal-witness. La déclaration ne peut pas être placée dans la racine commune :
le chargeur existant refuse à juste titre le chevauchement de son dossier avec
les racines de stockage. Conserver cette protection, utiliser identity.

Avant tout journal : intention exacte sous pin rendue durable. Écrire seed32 et
déclarations finale/de stage par Documents, modes0600 et aucune réécriture/adoption.
Reprise explicite compare les octets attendus avant acquittement durable. Appliquer
les deux événements signés avec vrais journaux depuis base0, seul rejeu exact
séquence1 admis ; causes et effets inconnus conservés. Vérifier grant métier actuel
et bootstrap personnel sémantique au moyen des noyaux existants, sans seconde
implémentation crypto ni création de compte/checkpoint personnel.

Après fermeture des noyaux, synchroniser fichiers et dossiers, recontrôler objets
et noms puis rename de racine NOREPLACE et fsync du parent. Une erreur conserve
intention, tentative/ack des journaux et publication finale NotAttempted/Unknown/
Visible/Durable ; toutes les fermetures indépendantes conservées. Aucun reset ou
suppression automatique d'une racine partielle. Reprise de stage et réacquittement
d'une publication distincts et explicites, pin et matière inchangés. Des états
avancés/révoqués refusent sans retour aux heads initiaux.

Tests : témoin avant défaut ; publication réelle puis personal-init/enroll/recover
sur données générées isolées, refus mauvais pin/UID/destination existante,
symlink/hardlink/modes/sidecars, signature ou grant invalide avant publication,
interruption après intention/journal/rename et reprise exacte, annulation et
fermetures pertinentes. Aucun datastore vivant, OS utilisateur ou humain créé.
DocFR/EN, fonctions, recette, revue liée aux octets, build/install normal et vérif
publique requis. Étapes suivantes : endpoint personnel, frontière UID/confinement,
UX d'installation et campagnes ; E2/E3/PA13 non acquis.


Bibliothèque qualifiée : 44 tests nommés/5 étapes PASS, inventaire stable et
intégrité vérifiée ; Rustdoc privé -Dwarnings PASS. 14 tests du publieur couvrent
installation réelle puis initialize/enroll/recover, refus physiques et autorité,
annulation, étapes interrompues, effet visible après rename, effet durable après
fermeture en erreur et causes indépendantes, état avancé jamais remis à zéro.
L’intention a été déplacée à la racine AVANT création/réparation des sous-dossiers.
Un rejeu isolé de l’ancien ordre a échoué sur l’assertion précise de réparation
prématurée ; il ne constitue pas un ancien binaire historique octet pour octet.
Sources de production inchangées pendant ce rejeu, originaux conservés.

Livraison43 groupée avec le raccord public44 : une bibliothèque non appelée par
le binaire ne prouve pas le parcours installateur. Étape suivante autorisée :
commande CLI d’installation depuis un artefact de service privé relu sous pin,
projection complète des phases/causes, recette sur binaire normal puis build/
activation supportés et navigateur. Dev165 reste la livraison42 vérifiée.
Limite à conserver : RecoverPublished est initial uniquement ; la réconciliation
physique après progression de compte doit rester distincte de l’autorité courante.
Propriétaire service/intégration ; avant exposition du listener, conserver un
acquittement durable d’installation et un parcours de reprise adapté aux états
avancés. Aucun service/UID séparé/humain/critère E2/E3/global/PA13 acquis ici.
