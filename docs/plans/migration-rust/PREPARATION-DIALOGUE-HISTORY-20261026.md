# Historique des échanges IA — lecture livrée153

OwnerCodex ; helper GLM20682 jetons déclarés (3271entrée+17411sortie),97,327s,
contexte11716octets. Modèle demandé/déclaré glm-5.2, backend5.3 non prouvé.
Proposition de modèle de tour reçue ; aucun outil ni test exécuté par GLM.
Corriger avant intégration : Debug du prompt privé, promesse « sérialisation sans
échec », helper inutile de suppression du prompt. Codec réellement existant :
work::decode avec shape ; aucune trait ModelCodec n’existe dans ce dépôt.

Périmètre : CLI prepare dialogue ID, GET/HEAD protégé, historique dans l’UX
existante. Données SQL et octets de méthodes observés ; aucun appel, réservation,
remboursement, nouveau délai ou arrêt de processus. Tous les tours conservés,
prompt jamais publié. Fraîcheur/marquage used_in_* calculés sur les documents.
Le lecteur Go peut tuer/finir un tour actif après15s. Ce comportement n’est pas
acquis en Rust : refus explicite501 des tours actifs jusqu’au conducteur porté,
sans politique15s copiée, sans affichage trompeur d’une fin ou succès vide.
Les échanges finis peuvent être lus ; send/stop/adoption de proposition restent
à porter. Réutiliser le rendu de conversation existant, commandes IA inactives.

Critères : absence[], ordres/alias/null, identité SQL, prompt absent, used/stale,
statut SQL autoritatif, refus actif/corruption et reprise sans mutation, UI texte
inert FR/EN×thèmes/mobile, recette durable et installation locale observable.
Fichiers réservés preparation_turn.rs, root_store/preparation_dialogue.rs,
preparation_read.rs et exports/adaptateurs additifs ; rendu partagé dialogue,
nouveau raccord readonly natif, locales/docs/tests. Préserver mémoire/S2/identité.

Replan édition : le catalogue EN est une map de traductions directe, avec une
clé littérale strings ; script initial l’a prise pour un conteneur et a refusé
AttributeError avant l’édition locale/notes/docs. Structure réelle vérifiée et
précondition corrigée ; génération i18n précédente sans nouvelle traduction ne
qualifie pas ce lot. Refaire la génération après insertion contrôlée.

Amélioration de transmission : le premier état du PLAN décrivait encore115 et
annonçait équipe/release manquants malgré124/149. Historique conservé, tableau
courant placé en tête, limites et preuves distinctes ; aucun nouveau pourcentage
ni statut d’autrui déduit. Sa lecture prochaine doit utiliser ce tableau avant
l’historique ; un fichier enregistré ne prouve pas son chargement par un agent.

Qualification150 initiale refusée : identity_personal.rs ajouté pendant le build ;
construction concurrente suivante refusée pour modification du même fichier.
Aucun contournement : sources modifiées prises en compte avant un nouveau build
stable150 normal SHA5478ae5fd06e464c0b2cbe26346e84980b404435341f1596f7281c170a2416f9.

Disque :764Mo libres observés, premier nettoyage refusé par garde compilateur
actif (aucune suppression). Après constat d’absence de compilateur, nettoyage
sous doubles verrous Cargo de2987 fichiers incrémentaux projet,2814507035octets
inventoriés/hachés. Caches non attribués, deps, exécutables, sources et preuves
préservés ;3,2Go libres observés ensuite. Build normal utilise déjà incremental=0,
pas d’attribution de cette croissance aux seuls builds de ce chantier.

Tests directs150 :35 CLI/API et38 navigateur readonly PASS. Extension de la
recette :3 assertions du wrapper de rendu historique éditable (callback/identité
du nœud conservée), puis ancienne recette manuelle. Son garde « routes non portées »
listait encore dialogue/templates/project-profile malgré les livraisons ; adapter
aux routes réellement absentes send/stop/conversion/providers de préparation.
Ancien fichier conservé, aucun retrait des contrôles de données/erreurs.

Replan UI : la recette installée sélectionnait synthétiquement une option ; le
handler isTrusted l’ignorait, sans erreur produit ni données changées. Diagnostic
CDP du seul navigateur de recette : chat caché, titre initial, aucune erreur.
Navigateur de recette fermé proprement (échec Target closed conservé), service
préservé. Reprise par URL publique id ; attendre aussi l’erreur visible comme
issue terminale plutôt qu’un seul état de succès. Aucun watchdog arbitraire.

Revue de contrat : !answer/!model_route/!plan_summary acceptaient false/0/chaîne
vide comme absences. Contre-exemple150 history-falsy-answer-refuses FAIL retenu.
Correction générale optionalObject : seuls null/undefined signifient absent ;
les trois champs invalides sont couverts avec reprise. Contrat des données
moteur inchangé, correction de notre nouveau validateur UI ; build à établir.

Recette finale152 PASS/VERIFIED :35 CLI/API,45 navigateur (dont3 objets falsy
refusés/reprise et3 contrôles du wrapper éditable),105 préparation manuelle.
Déploiement local partagé153
SHA122bf7ab0961e2ea2095d95d00ae7e94a0bc3c58b3c9e1613546d7ae82c856a4.
Les25 entrées compilées sélectionnées sont identiques152→153. Instance réelle :
18 HTTP/process/assets/état,14 Version FR/EN,8 page readonly PASS ; historique
CLI/API identique et ouvert par URL publique. Réglages41 et missions pausées
préservés. Inspection visuelle du fixture EN sombre390 : texte inerte lisible,
conversation défilante, champs/actions et contrastes sémantiques distincts.

RETEX : falsy n’est pas synonyme de null pour un objet optionnel. Conserver
le reproducer200 malformé et tester absence/valeurs invalides/reprise ; ne pas
réparer un diagnostic manquant par un timer. Les tests doivent emprunter une
interaction de confiance ou une URL publique, jamais une mutation synthétique
ignorée par le produit. Le DTO public supprime le prompt même si le caller oublie
de le vider ; il ne doit pas sérialiser la persistance privée du futur runner.
Limites : send/stop/appels et supervision IA non portés, actif501 maintenu ;
pas de preuve d’authentification à partir d’un ProjectContext historique, pas
d’acceptation indépendante ni clôture globale de la migration.

Investigation de suite : le projector organisation historique semblait simplifié
par rapport au Go. Lecture de son vrai caller confirme supported_observed→
validate_mission_organization avant organisation ; sa Rustdoc et ses tests
existants énoncent ce prérequis. Aucun défaut établi par la seule lecture du
projector ; ne pas le réutiliser sans validation dans une future conversion.
Ces anciens tests ne sont pas déclarés rejoués par ce lot.
