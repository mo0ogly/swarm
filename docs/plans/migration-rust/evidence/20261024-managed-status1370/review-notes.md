# Revue des octets de1373 — locale, non indépendante

Baseline exacte : normal1361/after.json ; actuelle normal1373/after.json. Trois entrées effectives changées.

## mission_status_readiness.rs
Politiques : le suffixe -1 désigne la première tentative, après refus de tout historique d’essais/Agent de cette tâche. Il ne fixe aucun plafond d’essais. copies est le chemin protocolaire historique Go managedCopyRoot. Aucune nouvelle politique opérateur, aucun fallback de coût/délai. Slots/budget viennent toujours des producteurs persistés existants (readiness/dispatch_context/settings).
Fonctions : prospective_workspace est une projection sans IO ; ses préconditions, limites et refus sont documentés. observe garde validation organisation, refus historique, capacité, priorité du profil tâche et budget/brief ; seul le workspace effectif change pour repository. Deux tests isolent sous-projet, tâches distinctes, parent/voisin et métadonnées invalides ;15 tests readiness sur octets finaux1375.
Erreurs : nom/champs de chemin ambigus ou échappants refusent par code stable sans secret ; erreur de représentation UTF8 explicite. Aucun effet externe à annuler, fichier à fermer ni SQL à rollback dans cette projection. Le consommateur historique existant conserve sa chaîne de causes/cleanup et ne renvoie pas une projection partielle. Le témoin public et la reprise du Root original1376 passent. Ce calcul n'observe aucun inode/copie/commit et ne délivre aucune permission physique.
Documentation : contrat observe/AutomaticInputs conservé et Rustdoc helper nouveau ; documents FR/EN indiquent explicitement la portée initiale et les refus du lancement/règlement. Les métadonnées de chemins malformées sont une limite de cette projection ; aucune adoption universelle du stockage historique Go n'est revendiquée.

## mission_status_historical.rs
Modification Rustdoc seulement : les tâches managed initiales utilisent les observations existantes ; les exécutions managed restent exclues. Aucun producteur Git/copieur ni autre changement de politique/erreurs. Les comparaisons avant/après existantes ne constituent pas une snapshot atomique ni protection ABA.

## version_history.json
Un item FR/EN candidat, décrivant la lecture et ses absences d’effet. Les notes ne signifient pas installation ; pas de nouvelle valeur opérateur ni code de gestion d'erreur. Entrée effective du binaire gelé1373.

## Recette/documentation non compilées
Test permanent rust_managed_planning ajoute un démarrage de consentement CLI (aucun conducteur CLI lancé), puis les deux tâches en attente sans conducteur, compteurs total2/running0/validated0, zéroAgent par SQL lecture seule, absence copies, Work/ledger/marqueurs inchangés. Les deux programmes planner inertes restent les seuls processus fournisseur. Le test snapshot historique ne couvre que l'Agent session ; l'assertion SQL supplémentaire couvre bien tous les Agents de ce Work. La reprise1376 compare le dump complet de la base originale avant/après.
Le manifeste existant ajoute la source readiness et le huitième libellé ; runner sans watchdog global. La recette verte et son verify sont retenus. Aucun nouveau test/cas remplacé par exit seul.
Docs FR/EN et PLAN doivent suivre cette portée ; aucune modification UI, donc aucune recette navigateur ni preuve visuelle annoncée. Copie/exécution/règlement managed, concurrence globale, compatibilité legacy et acceptation indépendante restent ouverts dans le plan.

Apprentissage : le profil configuré et le workspace effectif du dispatcher sont deux observations distinctes. Le copier au lieu de relire profileFor produirait de fausses collisions ; conserver les cas parent, voisin et sous-projet avant de raccorder le vrai lancement.
