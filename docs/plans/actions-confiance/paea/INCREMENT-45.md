# PAEA — incrément45 : capacités Linux et frontière effective

Reprise :43/44fonctionnel livré/vérifié sur168 (50bibliothèque,6supplément,8CLI/
59commandes,7anciensCLI,56UI+refusscript). Contrôle final refuse une assimilation
au checkout actuel : Cargo.lock ajoute APIclient à swarm-web, notes étrangères
modifiées, PAEA identique ; autre propriétaire a livré169 avec ancienlock. Nouvelle
qualification du lock/catalogue courant en parallèle du diagnostic indépendant.
Preuves168 historiques conservées, pas de conformité actuelle inventée.

Besoin : avant endpoint personnel et usage agentique protégé, prouver les identités
OS réelles, la protection des clés et la séparation contrôle/agent. Les UID de nos
fixtures ne prouvent rien sur l'hôte. Codex seul, sans GLM/délégation/commit/push.

Première étape bornée et lecture seule : euid/groupes/capacités/ns de ce processus,
programmes installés et modes setuid, plages subuid/subgid du seul utilisateur,
paramètres userns/AppArmor lisibles. Témoin Python normal avant expérience userns
éphémère ; pas de sysctl/chmod de dossiers partagés, compte OS, daemon privilégié
ou changement de mission. Conserver stdout/stderr/argv/retcode originaux. Un UID0
observé à l'intérieur d'un userns ne vaut pas UID distinct sur l'hôte ni confinement.
Ne pas supposer sudo, bwrap ou newuidmap ; constater leur disponibilité et refus.

Après faits : choisir le plus petit lancement pris en charge qui conserve les
UID/domaines, descripteurs et racines protégées. Préparer les raccords manquants
avec des critères observables ; une politique hôte refusée n'autorise aucun
contournement. Traiter les travaux indépendants tant que prérequis humain/OS/
évaluateur manquent. Toute erreur déclenche diagnostic/replan, aucune relance
identique ni temps d'attente silencieux. PA02/PA04,0sur17,E2/E3/PA13 ouverts.

Résultat45 : lecteur persistant lecture seule et recette permanente qualifiés,
5tests nommés + observation réelle,2étapes PASS/stables/intégrité VERIFIED.
Les essais userns/bwrap ont un témoin valide, puis UID0 interne mappée à la même
UID1001 externe : lecture du témoin privé encore possible. Contre-exemple conservé,
aucune séparation OS acquise. Helpers newuidmap/newgidmap absents, plages165536+
65536 observées ; sudo noninteractif refuse avec authentification interactive
requise. Aucun appel privilégié, paquet, compte ou service modifié. Action humaine
concrète demandée : installation locale de uidmap par administrateur dans son
terminal, sans mot de passe transmis au chat. Sa disponibilité ne suffira pas à
qualifier un lancement ; groupe/capacités/FD/montages/réseau/reprise à prouver.

Erreurs de méthode conservées : aide sudo sur stderr plutôt que stdout ; fausse
prédiction d'un test rouge alors que les5tests passaient ; premier manifeste
incomplet refusé avant exécution (environment manquant), corrigé puis recette
fraîche. Aucune preuve originale réécrite. Revue liée aux octets source-review.json,
liens et whitespace vérifiés, sources archivées. Outil autonome Python sans entrée
dans le graphe du serveur : aucune compilation/restart ni version produit ajoutée
pour ce diagnostic. Livraison43/44 requalifiée dev174 dans son propre dossier.
Suite46 : préparer le contrat de service et sa reprise avant usage, indépendamment
de l'authentification administrateur encore requise. PA02partiel,0/17,E2/E3/PA13
ouverts ; aucun confinement, enrôlement humain ou service activé déduit.

Replan documentation : trois liens antérieurs de tools/verification/README.md vers les result.json Rust104/105/106 ne résolvent plus. Originaux indisponibles, propriétaire migration Rust ; références conservées, aucun reçu recréé. Les liens PAEA ajoutés sont contrôlés séparément ; conformité globale des liens README OUVERTE. Reprise : originaux restaurés/vérifiés ou relocation prouvée par leur propriétaire.
