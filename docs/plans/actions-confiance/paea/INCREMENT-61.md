# PAEA — incrément61 : réponse sur paquet attribué

Reprise60 livré209, Codex seul sansGLM/agent/commit/push ; PA02partiel/global0sur17.
But : retour exact d'une réponse du service sur SOCK_SEQPACKET, sans transformer
une erreur d'envoi en absence d'effet métier. Source send(2)/socket(2) man-pages
primaire : MSG_NOSIGNAL évite une mutation globale SIGPIPE ; succès local ne prouve
pas réception. Aucun plafond/tentative/délai caché ; blocage/nonblocage reste état
explicite du socket fourni par le consommateur, à configurer avant usage.

Option1 adaptateur Write : risque write_all en plusieurs messages et phase flush
artificielle ; non retenu. Option2 send natif un appel et API réponse spécialisée :
retenu. Cause/étape tentative/compte accepté connu conservés, Reply/Observation
complets après erreur. Contrôles type/options avant envoi ; jamais SCM_RIGHTS ou
credentials forgés, aucun shell/PID/admission implicite.

Tests d'abord : vrai aller-retour/deux paquets/buffer secret intact, refus vide et
options altérées avant envoi puis reprise, vrai pair fermé/EPIPE sans SIGPIPE,
réponse exacte liée au consommateur et résultat métier gardé après échec. Témoins
avant refus. AnciennesAPI stdout Write/flush inchangées. Docstrings/FR-EN/recette
permanente/revue octets/politiques/erreurs et livraison normale sur même ELF.
Daemon/lancement approuvé/UID réels/humain/confinement restent ouverts.

Sources : https://man7.org/linux/man-pages/man2/send.2.html ;
https://man7.org/linux/man-pages/man2/socket.2.html. Le contrat documenté ne remplace
pas les témoins noyau réels. Recherche initiale d'une recette sansidentity dans son
nom a refusé fichier absent ; chemins réels retrouvés avec rg --files, aucun code
ou test exécuté sur chemin supposé.

Deux contrôles supplémentaires après revue des limites : SO_SNDBUF observé212992,
paquet trop grand refusé EMSGSIZE sans entrée en réception ; saturation réelle
non bloquante après278enregistrements, EAGAIN puis vidage exact/reprise. Valeurs
mesurées de fixture, jamais defaults ou bornes produit. complete10 passe neuf
nouveaux tests et Rustdoc4/4 ;18régressions à relier au lot final. Build09 refusé
pour25fichiers Web modifiés en parallèle,209 conservée. Le propriétaire annonce
préparer son candidat final ; attendre ce candidat partagé et vérifier68entrées
PAEA actuelles, sans boucle de rebuild ni source étrangère modifiée.

## Livraison locale vérifiée

v0.1.1-dev.211, ELF `20444629f554417d220d882f09113ff76cf7e48a17bc40b4285a878fcc2825f7`, PID1904817 au contrôle final. Profil normal
650entrées effectives/68PAEA, sans test-fixtures ; sources unitaires/integration
de61 exclues.27tests Rust distincts PASS :9nouveaux (5canal,1SIGPIPE isolé,
3réponse service) et18antérieurs (9réception,4Write/flush,5consommateur).
Rustdoc strict4sources/4pages,8HTML originaux archivés. Tous liés aux sources
exactes ; ni volume de tests ni build vert ne vaut acceptation globale.

Même ELF installé pour5appels/56commandes,7personnels+3gardes,3paires peer/6commandes,
56contrôles UI FR/EN+garde d'échec. Dialogues clairFR/sombreEN inspectés ; version,
notes61, vrai commit et modifications non commitées vérifiés. Ces parcours natifs
existants ne prétendent pas exercer un nouveau daemon. Entrées réelles et ressources
protégées vérifiées par les recettes locales ; pas de binaire fixture livré.

Quatorze intégrités :10PASS conservés et4FAIL originaux (API absente, visibilité interne,
API+fixture libc absentes, API réponse absente après correction fixture). Aucun test
exécuté sur les refus de compilation ; ils ne sont pas des PASS. La fixture a repris
Errno existant sans nouvelle dépendance et emprunte le View avant son envoi. FFI
reste dans kernel ; visibilité interne corrigée. Échecs et reprises conservés sous
`evidence/20261027-reply61-8xxdwar_/`. Revue des politiques/fonctions/erreurs liée aux octets,
source-review.json/reviewed61-sources.tar.gz ; closed-local-result.json.

Build09 compilé puis refusé pour25sources Web concurrentes ; originaux préservés.
Deux candidats communs intermédiaires ont aussi refusé des entrées Trust instables ;
les originaux sont dans retained-failed-builds.json. Le candidat commun211 est
qualifié puis sélectionné par SHA sans reconstruction supplémentaire.
EMSGSIZE et EAGAIN réels sont vérifiés avec capacité observée212992 et saturation278
enregistrements, vidage exact et reprise ; mesures de fixture, pas politiques produit.
Après les contrôles natifs, seule une note de source ajoutée par un autre lot
différait du catalogue compilé. Les notes211 archivées et les dialogues inspectés
font foi pour cette livraison ; la note61 est inchangée, les67autres entrées
propres sont identiques. Le contenu plus récent reste non livré dans211.
Résultat connu après EPIPE conservé et relu après réouverture, sans rejouer la
mutation. Encode avant envoi vérifié avec date invalide synthétique après témoin
réel ; un retour court positif du noyau n'a pas été observé/injecté. SIGPIPE défaut
débloqué uniquement dans un worker éphémère, restaurations contrôlées.
Apprentissage : un adaptateur Write/write_all ne doit pas fragmenter une réponse
SOCK_SEQPACKET ; succès local reste distinct du reçu pair et de l'effet métier.

Suite : composition réception/admission/réponse et consommateur protégé, puisPA03.
Daemon, approbation/image, UID réellement distinctes, permission ptrace inter-UID (piste propriétaire mêmeUID qualifiée62),
humain, confinement et revue indépendante restent ouverts. uidmap admin attendu
sans nouvelle demande ni contournement. PA02partiel/0sur17/E2/E3/PA13/RETEX ouverts.
Codex seul, aucun commit/push/GLM/agent ; aucune mission DB vivante utilisée.
