# M33 — MEM-P1, lectures de pause et consommateurs

17 octobre 2026, reprise par Codex seul avec replan/audit-pdca en correction.
Origine : matrice M32, lecteurs booléens et transactions restants. Preuves dans
`evidence/m33-20261017`, préimages limitées et HEAD de reprise conservés.

## Décision et périmètre

Le lecteur Store.paused absorbe les erreurs et sert à 19 appels : opérations,
ordonnancement, contexte transmis et vues. Plusieurs transactions lisent aussi
directement la pause ; certaines refusent déjà mais exposent sa valeur invalide
ou remplacent sa cause par un refus métier. Ces constats source ne prouvent pas
un lancement non autorisé. Reproduire avec témoin valide avant chaque injection.

Deux voies : adapter seulement les mutations critiques et conserver une seconde
lecture booléenne, ou migrer ses consommateurs vers readMissionPaused M32.
Retenir la seconde : aucun défaut de lecture ne devient « non pausé ». Les
prédicats d’éligibilité restent booléens et refusent, avec leur raison explicite ;
le contexte partiel utilise Missing et ne fournit pas un fait false inventé.
Les opérations avec erreur propagent une cause sûre ; les vues texte affichent
le diagnostic sans présenter les départs comme autorisés.

Les blocs paused/SELECT de prepareLaunch dans agents_store.go sont annoncés au
propriétaire identité, avec empreinte préalable et préservation du reste. Aucun
champ, enrôlement, autorité ou droit identité changé. Les lectures jointes de
kernel_attempt_source_linux.go et le prédicat SQL de lifecycle.go restent au lot
des enveloppes liées : elles refusent déjà les valeurs non nulles ; leur cause,
la cohérence des enveloppes et le diagnostic demandent une qualification dédiée.

## Critères et preuve attendue

| ID | Observable |
| --- | --- |
| M33-01 | Témoins valides établis avant les injections ; originaux rouges conservés ; défauts produit distincts des préconditions de fixture. |
| M33-02 | Plus de lecteur booléen SQL silencieux dans ce périmètre ; appels migrés ; absence historique non pausée et booléen strict M32 conservés. |
| M33-03 | Pause illisible : opération refusée avec cause et diagnostic sûr, aucun départ/réservation/publication ni mutation ; réparation publique et reprise valide. |
| M33-04 | Vues CLI/HTTP/texte et contexte ne publient pas paused=false ou un départ possible sous erreur ; donnée manquante explicitement attribuée. |
| M33-05 | Sélection ciblée normale/race, vet/build, sources racine/paquets/embeds et overlays effectifs liés aux reçus stables ; FR/EN, notes locales et RETEX. |

Les 138 ressources UI de M32 ne sont pas réutilisées comme version courante :
inventorier à nouveau les fichiers réellement embarqués et conserver un overlay
de fixture complet web/locales. Go reste courant. TMPDIR privé sur disque après
mesure (tmpfs peu libre), aucun nettoyage de cache ou preuve tiers. -timeout=0,
pas de watchdog global inventé, observation de la progression. Aucun lancement
fournisseur réel, délégation, commit/push, déploiement ou redémarrage autorisé.

MEM-P1 reste en cours ; cette tranche ne livre pas P2–P5 ou M20.

## Reproduction avant correction

red : les 16 noms (huit parents et huit sous-cas) échouent sur le comportement
attendu, après des témoins valides ; aucune précondition de fixture en échec.
Les originaux et l’inventaire Go effectif sont conservés. CLI/HTTP renvoient un
succès avec pause=false ; les vues/contexte et l’éligibilité perdent l’erreur.
Les gardes de lancement/validation/règlement et l’application du modèle exposent
la valeur privée dans le refus ; modèle/contrat/automation remplacent sa cause.
Le cycle sans pause lisible retourne nil et efface son empreinte précédente.
Ces reproductions ne démontrent pas de modèle distant ni de départ non autorisé.
Deux tests existants interrogent paused : les adapter au lecteur avec erreur,
sans réduire leurs assertions sur la pause et les compteurs.

## M33-I01 — précondition du script de migration

Une première préparation en mémoire attendait un seul appel dans console.go ;
il contient le snapshot et le rendu texte. Le contrôle de cardinalité a refusé
avant toute écriture. Aucun fichier produit changé par cette tentative. La
recette conservée déclare les deux consommateurs et contrôle toutes les préimages
avant ses écritures. Application : 21 fichiers produit, deux tests existants
adaptés et le nouveau fichier de régression ; aucun champ identité changé.

## Sélection de qualification

Le candidat initial passe les 16 noms nouveaux, entrées stables. Renforcer
ensuite les témoins d’application de modèle, l’action task.start indisponible
dans le contexte partiel et les reprises de cycle/contrat. Qualifier ces sources
renforcées avec 87 parents ciblés : les 21 modules changés touchent admission,
transactions, publication, validation, automatisation et présentation publique.
Race : 12 parents, dont les huit nouveaux et quatre cas de concurrence
(writer, conducteur, contrôle, automation). Noms/regex et justification dans
selection.json ; aucun package complet ni suite globale. Les notes locales
FR/EN sont préparées dans version_history.json, sans annoncer une version active.

## M33-I02 — inventaire exact des consommateurs

La première estimation manuelle de 24 appels mélangeait consommateurs booléens
et gardes directes. Le calcul sur les préimages constate 19 appels booléens et
sept gardes SQL supplémentaires dans 21 modules. Le contrôle d’inventaire refuse
le mauvais total ; aucun fichier Go ou reçu n’est modifié. Documents actuels
corrigés, tableaux et préimages conservés dans consumer-inventory.json et
inventory-count-incident.json. La qualification continue sur le même candidat.

## M33-I03 — contrat réel de la commande version

La commande du binaire de fixture réussit et fournit identité/diagnostic. Le
contrôle complémentaire supposait à tort binary.history : KeyError, sans
modification de source ou d’environnement. Schéma réel relu ; ne pas étendre
le CLI pour satisfaire cette hypothèse. Vérifier séparément les octets exacts
version_history.json embarqués et les deux notes FR/EN comme preuve structurelle.
L’affichage/consommation des notes dans la modale active reste NON TESTÉ, faute
de redémarrage/livraison autorisés ; aucun résultat de test Go n’est relancé.

## Résultat local de l’incrément

M33-01 à M33-05 : 163 noms normaux PASS (87 parents, 76 sous-cas), 20 sous race
(12 parents, huit sous-cas, avec recouvrement), quatre contrôles de version/notes
supplémentaires, vet/build et entrées effectives stables. Sources racine et
embarquées inventoriées ; aucun ancien reçu réécrit. Les notes FR/EN sont
embarquées dans le binaire de fixture, leur projection est testée par ses
consommateurs existants ; modale active et livraison numérotée non qualifiées.

Documentation FR/EN, deux schémas rendus/inspectés, auto-revue et RETEX conservés
dans RESULTATS-MEMOIRE-29.md. La synthèse de vérification finale doit porter PASS.
La suite revient aux replis/vues et enveloppes liées, avec les deux lectures
jointes attribuées dans AUDIT-REPRISE.md. MEM-P1 et P2–P5 restent ouverts.
