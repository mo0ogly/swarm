# PAEA — incrément63 : échange contrôlé unique

Reprise61 livrée/vérifiée211,62 permission relative de namespace qualifiée en
same-UID seulement. Codex direct, sansGLM/délégation/commit/push. PA02 partiel,
0sur17/E2/E3/PA13/RETEX ouverts. Helpers absents déjà demandés, pas contournement.

But : composer réception, strict dispatch atomique, fermeture du paquet et réponse
canonique sur le même Channel. Aucun listener/boucle/délai/retry/concurrence/policy
nouveau. Buffer, flags, annulation et approbation d'image/domaines restent à
l'appelant. UID/proc/namespace/décodage/cible/autorité courante ne changent pas.

Choix : envoyer même après refus tardif de fermeture pourrait masquer le refus
au consommateur ; non retenu. Arrêter l'envoi sur erreur de dispatch/fermeture,
conserver le Reply connu et toutes causes, retenu. Un silence ne prouve donc pas
absence d'effet ni droit de rejouer. Succès transporte le résultat métier qu'il
faut lire séparément ; acceptation locale d'envoi n'est pas reçu du pair.

Erreur typée : réception, dispatch complet, close, output ; close indépendant
conservé après erreur de dispatch. known_reply localise sans recopier le Reply,
error_branches conserve les branches métier/AfterUse. Centraliser ce parcours sur
Reply pour réutiliser la matrice existante, sans suppression de cause ou fallback.

Tests d'abord : bon paquet sur vrai canal avec callback synthétique distinct
de l'autorité ; refus réception sans dispatch ; refus UID/domaine/décodage sans
réponse ; erreur tardive de close synthétique avec résultat connu et aucun send ;
vrai EPIPE avec résultat connu ; interface Service réellement refusée sur UID
non conforme sans mutation/réouverture. Garder la matrice AfterUse/IO existante.
Aucun succès de service séparé en2UID revendiqué sans helpers et recette physique.

Docstrings/FR-EN, recette permanente portable, source review octets/politiques/
erreurs/fonctions, profil normal et livraison locale, preuve sur vrai ELF/UI.
L'échange seul ne crée ni daemon ni admission humaine/confinement/mandat.

## État intermédiaire vérifié

current03 : 5 tests de composition réels avec callback métier synthétique et
1 test Service public passent ; erreur compilation current02 conservée (fonction
nix UID hors features). Aucun ajout de feature : PID noyau comparé au PID réel.
Le test public conserve les octets du checkpoint après refus et réouverture ;
Initialize répété est correctement refusé, sans prétendre être idempotent.

API : Service::exchange_atomic_controller_packet reçoit une seule trame, applique
le contrôleur atomique existant, ferme explicitement Packet puis envoie une réponse.
Lire Sent.reply même si le transport réussit. Sur erreur, known_reply et
error_branches permettent la reprise diagnostiquée ; ne jamais rejouer du seul
fait d'un silence. Receive ne dispatch pas après refus ; dispatch/close ne send
pas après refus. Error.packet_close conserve l'échec indépendant de fermeture.
Reply::error_branches centralise les erreurs métier et les deux branches AfterUse.

Recette maintenue : tools/verification/recipes/paea-controlled-exchange.json.
Ajout en qualification : enfant atomiquement identifié réel, domaine parent pris
indépendamment, réponse reflétée sur même canal ; callback métier synthétique,
same UID seulement. Pas de changement de politique/configuration, boucle/retry,
ni daemon. Les tailles de buffers sont des fixtures, pas une limite produit.

## Livraison locale vérifiée63

v0.1.1-dev.215, ELF `742f4d37b5a3119da8213af5014f82b720997bbe91500f2b8a36a05dde3238b3`, profil normal653entrées/69propres.
45tests Rust distincts (7nouveaux), Rustdoc3sources/4pages, 5parcours
d'appel/56commandes,7personnels+3gardes,3pairespeer/6commandes et56UI+garde
sur même ELF.11intégrités de recettes :9PASS et2FAIL originaux conservés.
La nouvelle composition est qualifiée en bibliothèque ; les parcours natifs
réutilisent les interfaces existantes et ne prouvent pas son admission daemon.
Helpers uidmap toujours absents ; réel2UID/humain/confinement/E2/E3/PA13/RETEX
et0sur17ouverts. Suite : consommateur protégé, puis PA03 suivant dépendances.
