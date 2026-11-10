# PAEA28 — secret personnel dérivé et preuve typée de connaissance

## Replan et portée

Le27 est vérifié et livré (dev123, puis dev124 parallèle observé). L'autorité
courante peut contrôler la configuration, mais ne sait pas vérifier un secret
personnel. Ni principal déclaré, UID, cookie cockpit ni checkpoint ne le prouvent.
La boucle autonome demandée le26octobre reste active, Codex direct sans GLM.

Implémenter d'abord le contrat cryptographique effectivement consommable par le
futur stockage/transport : Argon2id v19 RustCrypto, sel aléatoire OS, paramètres
explicites persistés avec le dérivé, record canonique lié à installation/principal/
génération. Vérification contre empreinte attendue indépendante avant dérivation,
puis comparaison constante et preuve opaque empruntant le record exact. Aucun
constructeur public de preuve, aucune session/grant/autorité métier par ce type.

Critères28 : dérivé réel et mots de passe erronés ; substitution de record/contexte
refusée ; entrées JSON ambiguës et paramètres invalides refusés ; cause technique
conservée et secrets absents des Debug/Display ; sel non déterministe et absence
d'IO de stockage ; documentation FR/EN et revue liée aux octets. Valeurs faibles
éventuelles des tests sont des fixtures explicites, pas un profil produit.

Les paramètres KDF n'ont aucun défaut Swarm : l'appelant fournit et persiste
mémoireKiB/passes/parallélisme, validation commune de la bibliothèque. Avant
exposition du service, cette configuration doit être gérée par l'installation et
Admin/API/CLI sans plafond caché. Aucun serveur d'authentification non configuré
n'est ouvert. Pas de délai/TTL/retry/politique d'essais inventé. Le stockage
transactionnel à consommation unique de bootstrap, sa liaison au checkpoint, le
transport, les écrans et le compte système séparé suivent ;28 seul ne les prouve pas.

Sources primaires : [RustCrypto Argon2](https://docs.rs/argon2/0.6.0/argon2/),
[API du dérivé](https://docs.rs/argon2/0.6.0/argon2/struct.Argon2.html).
La version0.6 exige Rust1.85 ; outil local1.96 observé. Deux URLs Params/PHC
n'étaient pas lisibles via le navigateur documentaire : lire le source réellement
résolu par Cargo plutôt que réessayer la même URL. E2/E3/0sur17 restent ouverts.


## Vérifications intermédiaires et replan

Le rouge initial est une API manquante (aucun scénario). Première suite :4tests
passent. Revue de la dépendance0.6 résolue : sa feature zeroize ne nettoie pas
l'allocation Blocks avant dealloc. Correction du contrat et du code : allocation
Vec<Block> détenue par Zeroizing, réserve faillible avec cause TryReserveError.
La suite actuelle a **5tests nommés PASS**, dont le profil d'exemple RustCrypto
19456KiB/2passes/1voie comme entrée de test, jamais comme défaut Swarm, en1,96s ;
les9régressions noyau et Rustdoc/liens passent. Nettoyage gardé par type/source,
pas preuve forensique de toute mémoire du processus.

`recipe-01` reste **FAIL** : le troisième pas CLI n'a exécuté aucun scénario,
compilation refusée dans le chantier étranger de publication cumulative (history
absent et deux champs manquants). Le propriétaire est le chat actif « Auditer
Swarm selon Scaling Agents » ; sa reprise est observée en lecture seule, aucun
message ni modification de son code. Les5+9résultats ne deviennent pas un PASS
global. Conserver l'original et reprendre CLI/build quand ce prérequis est cohérent.
Le serveur précédent n'a pas été arrêté par ce test. La livraison28 est encore
ouverte ; le travail indépendant de bootstrap durable peut continuer.

Une invocation de recette utilisait --evidence au lieu de --output : refus du
parseur avant exécution, corrigé selon son usage réel. Une inspection des notes
supposait une liste local_changes ; son vrai contrat est un objet avec items,
relu avant la seule insertion propre. Aucun fichier produit modifié par ces refus.


### Preuve additive de livraison30

Les bibliothèques internes actuelles sont incluses dans le profil normal vérifié
dev128.39tests actuels,6tests du harnais CLI et56assertions UI réelles PASS ; voir
[incrément30](INCREMENT-30.md). Refus historiques/acquisitions incomplètes conservés.
Aucun endpoint personnel, enrôlement humain, confinement OS ni acceptation indépendante
n’est établi.
