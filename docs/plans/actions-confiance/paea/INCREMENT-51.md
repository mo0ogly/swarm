# PAEA — incrément 51 : frontière Linux du canal attribué

Reprise après 50 livré et vérifié dans dev180. Objectif PA-02 : recevoir un paquet SOCK_SEQPACKET et le pidfd réellement attaché par le noyau à son émetteur, conserver les namespaces via ce descripteur, puis remettre ces données à une admission indépendante. Le transport seul ne donne aucun mandat, session humaine ou preuve de confinement.

## Décision structurelle et préconditions

Nix 0.30.1 expose SO_PEERPIDFD (connexion), mais pas SO_PASSPIDFD/SCM_PIDFD ; ses cmsgs() refusent une troncature avant l’itération, alors que les FD reçus doivent être fermés. Rustix 1.1.5 ne couvre pas ces opérations et ses credentials ont le défaut corrigé en 50. Le getter de connexion ne sera pas utilisé pour attribuer les messages.

Options examinées : attendre/mettre à niveau un wrapper amont effectivement disponible ; maintenir un fork complet de Nix ; isoler une petite frontière FFI Linux. Les recherches et sources locales ne fournissent pas encore de wrapper complet qualifié. Retenir la frontière explicite : nouveau crate Linux dédié, emprunt de socket, octets initialisés, propriété unique des FD venant du noyau et aucune API prenant un numéro de descripteur choisi par l’appelant. Les crates Swarm existants conservent forbid(unsafe_code). Le nouveau code FFI doit être déclaré, documenté et revu ; ne pas cacher son existence dans une macro ni prétendre que tout le dépôt est sans unsafe.

Utiliser les types/constantes de libc verrouillé et vérifier les rares noms UAPI absents contre les en-têtes et sources Linux, sans seuil métier inventé. Aucun listener ni commande ne s’active avant qualification. Préserver la séparation entre protocole technique et politique opérateur ; pas de délai global ou retry implicite.

## Ordre et critères observables

1. Propriétaire Codex : API minimale configuration seqpacket, réception gardée, processus/pidfd détenu et namespaces ; erreurs de syscall, refus et nettoyage conservés séparément. Pas de conversion de PID0 en type non nul, pas de PID rouvert dans /proc.
2. Tests avant raccordement : témoin réel puis troncature de données/ancillaires, mauvais type de socket, droit envoyé inattendu, paquet vide, options manquantes, fermeture/reprise. Vérifier le compte de FD et leur identité, pas seulement le code retour. Un émetteur enfant sur une connexion créée par le parent doit être attribué à l’enfant ; après sa fin, les opérations dépendantes refusent.
3. Rustdoc strict, revue liée aux octets sur politiques/fonctions/erreurs/unsafe et recette permanente sans watchdog arbitraire. Revoir les couches de nettoyage, y compris un refus après réception de FD.
4. Seulement après ces preuves : raccordement au service et consommateur lié, nouvel examen de l’admission et de la livraison locale. Une bibliothèque sans consommateur reste telle ; ne pas annoncer un daemon installé.

## Limites et reprise

Les helpers uidmap sont toujours absents. Compte OS réellement séparé, humain, daemon personnel, E2/E3, PA-13 et 0/17 invariants globaux restent ouverts. Le profil de test et la frontière FFI ne constituent pas une évaluation indépendante. Codex seul, sans GLM, délégation, commit ou push. Toute anomalie conserve son original et déclenche replan avant une nouvelle tentative.

## Résultat bibliothèque — raccordement encore ouvert

La bibliothèque est implémentée et vérifiée sur x86_64 / Linux 6.17.0-41-generic.
Neuf cas Rust sont établis : huit dans `recipe-07` puis le cas nommé non bloquant/
shutdown dans `nonblocking-10`. Les corps lib/kernel sont identiques à ceux de 07,
et les huit fonctions de test initiales restent un préfixe exact du fichier courant.
Rustdoc privé strict PASS. Après revue ABI/propriété, la fixture a été reconstruite
(`fixture-build-08`) et les deux parcours visible/PID invisible ont été rejoués dans
`native-09`, sur son nouvel ELF SHA256
`76bc13227691bb8c460538ea59325eee97e1ed00a601ad02ec504477a3342086`.
Tous ces résultats sont stables et leurs intégrités vérifiées. Pas de déduction
à partir de zéro test : `red-01` est le refus attendu du stub, `recipe-03` échoue
avant tests faute de feature uio, `recipe-04` conserve huit PASS puis FAIL Rustdoc.
La feature uio est réservée aux dépendances de test ; aucun paquet existant n’a
changé de version. Les layouts c_int/ucred/cmsghdr sont vérifiés à la compilation,
et le parseur privé exige explicitement des octets/FD issus du recvmsg courant.

Le sujet de credentials est celui vérifié selon les privilèges Linux, pas une
identité physique absolue ni un humain. Les namespaces et le pidfd restent des
données d’admission. La fermeture partielle d’un second ioctl est traitée dans
le code avec causes séparées ; sa fenêtre exacte de faute noyau n’a pas été
injectée. Aucun Miri, sanitizer, autre architecture ou revue indépendante annoncé.
Schémas FR/EN rendus et examinés, lecteur Markdown vérifié.

Dossier conservé : `evidence/20261027-channel51-nunzg4wm/`. Revue liée aux octets,
fonctions/erreurs/politiques/unsafe dans `source-review.json` ; paquet de sources
et en-têtes Linux retenus. Aucune clé, preuve personnelle ou mission vivante dans
les fixtures. La bibliothèque n’a pas encore de consommateur public : raccordement
et livraison locale groupée restent à faire en 52, sans annoncer son activation.
Le dernier serveur qualifié par cette tâche reste dev180 (lot 50, preuve datée).
Les prérequis UID/humain et l’assurance globale restent ouverts.

Contrôle documentaire élargi : 177 références historiques manquantes, vers 90 chemins, dans README et les deux grandes pages de migration. Toutes précèdent 51 ; aucun original dans index/HEAD courant. REPLAYS les déclarait déjà absentes. Inventaire et diagnostic conservés, propriétaire migration ; rétablissement conditionné à une archive originale vérifiable. Pas d’affirmation sur tout l’historique Git ni les sauvegardes. Nos liens 51 sont contrôlés séparément ; conformité globale de ces pages reste ouverte.
