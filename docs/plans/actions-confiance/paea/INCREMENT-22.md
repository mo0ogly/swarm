# PAEA — incrément 22 : rapprocher un accord fourni

Reprise après commit21 `72e0e32dc` demandé par l’utilisateur, puis « next ».
Codex réalise seul ce lot, sans GLM. PA-01/08/09 : raccorder le contrat existant
`ApprovalBinding`, `decode_approval` et `CanonicalAction::matches_approval` à un
rapport pur commun, CLI `protection check-approval ACTION ACCORD --json`, API
native et cinquième vue du menu. Aucun nouvel accord ou effet n’est créé.

Deux chemins examinés : consommer le contrat de correspondance existant, ou activer
l’accord protégé. Le second exige IA3 EP-01/04/05/07/10, source humaine indépendante,
autorité courante et confinement encore ouverts. Choix du premier chemin : les
hashes des deux fichiers lient le rapport au geste ; une différence de contenu ou
de politique produit un rapport négatif explicite, pas une erreur de parsing.
Une entrée invalide refuse sans rapport, avec diagnostic sûr et reprise exacte.
L’égalité ne prouve ni authenticité, ni droits courants, ni attention de l’approbateur.

Critères : témoin dev101 puis API404/CLI2 avant modification ; tous14vecteurs,
clés de tentative/contenu/contexte/politique/raw distincts ; CLI/API communs,
enveloppes fermées/admission/causes/Root inchangé ; FR/EN × deux thèmes ×1440/390/320,
match/différence/refus/reprise/export/clavier/chargement et rejeu des quatre vues
partagées. Deux exemples d’accord fictif ajoutés ; huit anciens fichiers inchangés.
Documentation/recettes/revue politiques-fonctions-erreurs liée aux octets ; profil
normal, livraison supportée et Version/notes/commit/dirty/assets réels, puis RETEX.

Résultats ci-dessous. Aucun commit/push déduit de « next ».0/17 invariants protégés,
E2/E3 et RETEX final du programme restent ouverts. Preuves dans `evidence/20261026-approval22-mp4rs3ig/`.

## Résultats et livraison

| Critère | Résultat observé | Preuve |
| --- | --- | --- |
| Témoin avant raccord | CLI inspect0/nouvelle commande2 ; API existante200/nouvelle404 ; Root isolé | `baseline-public.json`, sorties CLI conservées |
| Contrat et raccord | 2 schémas +3 noyau +2 CLI +2 HTTP +2 exemples/liens = **11 tests nommés PASS**, puis **10 contrôles de génération** | `approval-recipe-01`,942 fichiers figés, intégrité VERIFIED |
| Cinq vues natives | **3 086 assertions**,12 configurations par vue, **60 parcours exemples/192 téléchargements**, **113 refus natifs attendus** et reprises | `native-recipe-02`, intégrité VERIFIED |
| Contraste primaire | **36 observations** sur12 configurations, dont24 changements réels de thème ; minimum **9,096:1**, aucune transition croisée | `native-summary.json`, `primaryContrastSamples` |
| Contrôles Rust | Clippy bibliothèques et Rustdoc privé sur3crates PASS ;2warnings moteur étrangers conservés | `clippy.json`, `rustdoc.json` |
| Instance locale | **52 assertions en lecture seule**, **23 jeux de ressources protégées**, Version/notes/commit/dirty/focus FR/EN ; aucun POST métier | `live-recipe-01`, intégrité VERIFIED |

Installation supportée `./swarm.sh build`, puis `./swarm.sh restart --no-open` :
**v0.1.1-dev.107**, PID observé963961, SHA256
`b05390d2a1b12e5f856fbd34b615b1dff4a04629169c88e4e4f9d8cc0bb17b1f`,
commit compilé `72e0e32dc735ea63e245bdd7fff90f685febd757`, modifications présentes.
Profil normal :111artefacts,4crates cœur hors test et sans fixture,472sources
compilateur archivées. Les14entrées propres de code/assets et `version_history.json`
sont identiques à l’archive effective du candidat107 ; il est aussi le binaire
exercé par le navigateur et l’instance réelle. Configuration source inchangée.
Les dix sources Rust propres restent identiques à la recette d’intégration ; sa
preuve n’est pas présentée comme une compilation des nouveaux octets CSS.

Captures examinées visuellement : FR/état/390 règlement et EN/sombre/320 Git
(`native-browser-02/*-approval-*/result.png`). Titres, limites, formulaires, résultat,
hashes et export restent lisibles, sans débordement. Les états négatifs, erreurs,
chargement et reprises sont vérifiés par les interactions, pas déduits des captures.
Les huit anciens exemples sont inchangés, deux accords fictifs sont ajoutés.

## Incidents, revue et RETEX du lot

1. Le premier collecteur attendait sortie0 après SIGTERM de sa fixture ; la preuve
   distingue ce défaut de méthode du témoin HTTP200/nouvelle route404. L’arrêt
   SIGINT de la recette maintenue rétablit la précondition, sortie0 conservée.
2. Le précontrôle des aides détecte des libellés anglais omis et un titre hérité.
   Correction après inventaire effectif des textes ;23entrées propres, titre
   exact vérifié en navigateur. Une tentative de précontrôle échouée a été suivie
   par erreur de vérifications dépendantes inchangées : aucun PASS n’en est déduit ;
   la suite exige la réussite du prérequis avant compilation ou recette.
3. Le collecteur a d’abord choisi l’archive du `bin/swarm` installé101, au lieu du
   candidat préparé105. Le code du lanceur confirme que build conserve le serveur
   actif jusqu’au restart. Sélection du vrai résultat, comparaison des hashes et
   profils ; aucune faute de fraîcheur du constructeur n’est établie ici.
4. `native-recipe-01` échoue après2462assertions réussies sur le contraste anglais
   sombre. Contre-exemple conservé : changement réel clair→sombre,8frames,
   minimum **1,28158:1**, contre témoins stables9,096 et14,488. La transition globale
   de140ms croise les couleurs inversées du primaire. Attendre sa fin aurait caché
   le défaut. Le composant primaire PAEA commun change désormais ses couleurs
   ensemble ; les cinq vues antérieures/actuelle sont rejouées, les sources
   historiques restent intactes. Nouvelle compilation107 et recette02 complète
   requises et PASS. Les autres contrôles de l’UI globale restent une hypothèse à
   examiner par leur propriétaire UI, avec témoin et mesure des frames lors d’une
   évolution autorisée de ces surfaces ; aucune conformité globale n’est déduite.

Revue de même session sur **43 fichiers/hunks propres**, hashes et portée dans
`source-review.json` : invariants de protocole/représentation et données de fixture,
aucune nouvelle politique opérateur ; fonctions/docstrings et documentation FR/EN ;
propagation des erreurs jusqu’à la CLI/API/UI, refus sans succès vide, résultat
négatif distinct et reprise. Écriture/flush réutilisent le contrat existant, sans
nouvelle injection de panne de flush revendiquée. Les recettes HTTP complètes
nomment maintenant12tests ; seuls les2nouveaux tests HTTP ont été exécutés ici.
Le contrôle de diff propre passe ; les constats étrangers éventuels sont conservés
séparément. L’index préparé d’autrui n’est ni commité ni effacé par ce lot.

Apprentissages transmis dans le contrat FR/EN et la recette : vérifier chaque frame
observable d’une paire de couleurs inversée, pas seulement son état final ; lier
la preuve au **candidat préparé**, puis au binaire réellement installé ; inventorier
les libellés effectifs et respecter les prérequis des vérifications. Ces fichiers
persistants ne mettent pas à jour les contextes embarqués ou déjà lancés des agents.

Incrément22 réalisé, vérifié et installé, sans GLM/délégation, commit22 ou push.
Le commit21 demandé est **72e0e32dc**,40fichiers,108entrées préparées étrangères
préservées, au-dessus du commit moteur concurrent7af8189c8. Le refus initial de
publication21 sous ancien HEAD est conservé dans `docs/plans/actions-confiance/paea/evidence/20261026-inspection21-irsgmbcu/publication-01/`.
PAEA reste **0/17 invariants protégés de bout en bout** : IA3 EP-01/04/05/07/10,
AC-02, autorité humaine, E2/E3, revue indépendante et RETEX final ouverts. Prochain
raccord protégé : obtenir ces sorties réelles ; un nom d’approbateur dans un JSON
ne peut les remplacer. Cette tranche ne clôt pas les dépendances du programme.
