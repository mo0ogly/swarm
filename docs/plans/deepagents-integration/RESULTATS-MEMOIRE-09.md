# Cohérence du tour et du checkpoint avant reprise

16 octobre 2026, reprise du chantier mémoire après la parenthèse candidature.
Incrément : [INCREMENT-MEMOIRE-09.md](INCREMENT-MEMOIRE-09.md).
Codex seul ; candidat local, sans migration, modèle réel ou déploiement.

## Défaut reproduit et correction

L'examen des préconditions d'une restauration a révélé un contrôle absent : le
lecteur M11 ne comparait pas la session du tour répondu à celle du checkpoint.
Les fixtures reproduisent une ligne divergente ou vide et un statut inconnu :
la CLI, l'API et le diagnostic acceptaient ces états. Le même lecteur admettait
une session non utilisable par l'adaptateur et certains champs d'usage négatifs,
malgré une empreinte recalculée correcte.

Une seconde reproduction modifie la session après l'écriture de fin et avant la
confirmation : le wrapper écrasait cette valeur et démarrait l'échange suivant.
Il s'agit d'un contre-exemple en stockage isolé, pas d'un incident déclaré sur
une mission utilisateur. Les échecs d'origine et la source exacte restent dans
`evidence/m13-20261016/boundary-red.json/.log` et `preimages/`.

Le lecteur commun refuse désormais un statut inconnu, une session non conforme,
une session répondue divergente et un usage incohérent. La vérification du format
de session est partagée avec l'adaptateur existant ; aucun nouvel interpréteur
de commandes n'est ajouté. La confirmation compare la session de départ dans
son `UPDATE` conditionnel. Un remplacement empêche `answered` et l'appel suivant,
sans écraser la valeur concurrente. Cette garde est exercée sur les premier et
deuxième tours des deux adaptateurs Codex et Claude, avec fournisseurs scriptés.

La fin persistée et le statut restent deux écritures successives. Un état
`confirmed` dont la ligne est encore `running` ou déjà `interrupted` reste
consultable, mais non confirmé. Les compteurs existants, les usages inconnus,
les tours anciens sans checkpoint et les preuves historiques sont préservés.
Le format de stockage reste 34 ; aucun redémarrage implicite n'est ajouté.

## Vérifications et limites

| Contrôle | Résultat | Preuve sous `evidence/m13-20261016/` |
| --- | --- | --- |
| Contre-exemples avant correction | FAIL : trois tests, contrôle positif PASS | `boundary-red.json/.log`, source exacte dans `preimages/` |
| Lecteur commun, confirmation conditionnelle, sessions/usage, consommateurs M11/M12 et deux adaptateurs | PASS : 15 tests nommés, 36 sous-tests | `boundary-final.json/.log` |
| Concurrence, wrappers et usages cumulés | PASS : six noms, inclus dans les quinze | `boundary-race.json/.log` |
| Compilation et analyse statique | PASS | `build.json/.log`, `built-binary.json`, `static.json/.log` |
| Catalogue FR/EN et rendu existant | PASS | `i18n.json/.log`, `web-unit.json/.log` |
| Vraie CLI, superviseur, serveur et bouton ; état incohérent et retour cohérent de fixture | PASS : six contrôles, une invocation locale, zéro modèle | `browser-flow.json/.log`, `browser-flow/result.json` |
| Erreur FR/EN, deux thèmes, 1440/320 px, absence de confirmation, Échap/focus ; parcours positif jusqu'à 320 px | PASS : 16 captures, refus attendus distingués | Même recette `--integrity`, sous-dossier `browser-flow/` |
| Décision autorisée et restauration fournisseur après arrêt | NOT IMPLEMENTED / NOT TESTED | MEM-C15/MEM-C16 restent PARTIAL. |

Les refus CLI/API comparent les tables avant/après et le nombre d'invocations :
aucune mutation ni nouvel appel. Les deux endpoints renvoient HTTP 400 pour ces
incohérences, sans exposer les identifiants de session. Les valeurs NaN/infinies
sont contrôlées directement sur l'objet Go : elles ne sont pas présentées comme
des nombres JSON valides. Les tests de session et d'usage ne démontrent pas une
restauration réelle ni une disponibilité fournisseur.

Les sources et entrées des contrôles sont empreintées avant/après. Les quinze
noms ne sont pas additionnés aux six déjà inclus dans le lot `-race`. Les
exécutions initiales et finales restent distinctes. La revue du diff est locale,
sans approbation indépendante. Le serveur utilisateur reste séparé du candidat.

La recette finale observe quatre HTTP 400 attendus, sans erreur de page ni
échec réseau inattendu. Elle compare les sept tables avant/après les lectures
de l'état altéré et rétablit seulement sa fixture pour une lecture positive.
Les captures FR claires à 320 px et EN sombres à 1440 px ont été inspectées.
Le contrôle de contraste du parcours positif est conservé ; le contraste de
l'alerte d'erreur n'est pas mesuré séparément par cette recette.

## Documentation et RETEX

Le contrat, le refus, l'état transitoire et les recettes sont documentés dans
[PROJECT-MEMORY FR](../../PROJECT-MEMORY.md) et
[PROJECT-MEMORY EN](../../en/PROJECT-MEMORY.md). La leçon est appliquée par le
lecteur partagé et sa confirmation : vérifier l'accord entre représentations,
pas seulement l'empreinte de chacune ; confirmer avec la valeur attendue plutôt
que corriger silencieusement une écriture concurrente. Les lecteurs actuels et
les futurs consommateurs du même producteur reçoivent ce contrôle de code.

La première recette navigateur échoue sur sa propre sélection de langue : elle
écrit une clé de stockage supposée et conserve l'URL anglaise. Le code réel
donne priorité au paramètre `lang` ; la recette reprend le parcours déjà utilisé
par ses contrôles positifs, puis vérifie le thème par `setTheme`. Le refus moteur
attendu est distinct de cet échec de recette. Les sorties et la source d'origine
sont conservées dans `browser.json/.log`, `browser/failure.json` et `preimages/`.
La fixture était déjà arrêtée avant son altération ; le nettoyage ne tente plus
de lire sa vue volontairement incohérente pour confirmer ce même arrêt.

La seconde recette atteint les quatre vues d'erreur attendues, puis échoue sur
une requête `snapshot` annulée lors d'une navigation. La recette marque désormais
la durée de navigation et conserve séparément les seuls `net::ERR_ABORTED`
du suivi de cette même fixture pendant cette durée. Tous les autres échecs
restent bloquants. `browser-final.json/.log` et sa source restent conservés ;
la recette finale `browser-flow` passe et n'observe aucune annulation. Aucun
nouveau correctif moteur ni nouvel appel de modèle n'a été nécessaire.

Quelques recherches supposaient encore des noms de fichiers absents. Ces
résultats ne constituent aucune preuve et n'ont entraîné aucune modification
de produit ; les producteurs et le contrat de langue réels ont été relus.
Le premier audit documentaire recherchait « en cours » dans toutes les pages et
a rejeté une explication légitime sur le serveur déjà démarré. Le contrôle est
resserré sur le statut du présent résultat ; le texte opérationnel reste intact.

## Suite du besoin initial

La décision fraîche, la réacquisition de l'espace et des réservations, le contrôle
de session et l'application d'une restauration restent à réaliser. Une nouvelle
tentative garde son contrat et ses coûts ; elle n'est pas renommée en reprise de
conversation. Cette correction ferme le défaut de cohérence reproduit, pas les
volets mémoire encore ouverts (restauration, résumé, suppression et qualité LLM).
