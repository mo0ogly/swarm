# R20 — observation eBPF, reprise du 16 octobre 2026

## Repère de reprise — 24 octobre 2026

La [décision ultérieure de report](qualification-globale-replan.md#décision-utilisateur-du16octobre--revue-autorisée-ebpf-reporté)
reste applicable. La [file APEX FR](../FILE-APEX.md) et
[EN](../../en/APEX-QUEUE.md) conserve R20 non qualifié : demande ultérieure et
environnement BPF explicitement autorisé avant essai. Les états « en cours »
ci-dessous sont historiques ; aucune sonde n’est relancée ici. Codex possède la
recette, l’opérateur l’autorisation d’environnement. AC-02/A5 inventorie les
preuves pertinentes ; sa clôture éventuelle ne vaut pas qualification R20.

Codex propriétaire ; programme Actions de confiance maintenu jusqu'au RETEX.
Aucun collecteur réel n'est établi par le scan ciblé des producteurs Go : lire les
composants d'attribution/processus et le contrat avant de conclure sur tout le dépôt.

## Précondition observée

`verification-ebpf-20261016/capability-inventory.json` : UID1001, noyau6.17,
x86_64, CapEff0, unprivileged_bpf_disabled2, BTF présent. bpftool/bpftrace/cc sont
installés ; clang/llc absents du PATH ne prouvent pas l'absence d'un compilateur
intégré à un autre outil.

La sonde C conservée crée seulement une map vide appartenant au processus, puis
la ferme si elle réussit. BPF_MAP_CREATE retourne EPERM. Aucun programme n'est
attaché, aucun événement ni processus étranger observé. Le succès du diagnostic
ne vaut pas disponibilité eBPF. Capabilités et sysctl sont des observations ; la
cause unique d'EPERM n'est pas prouvée (autres contrôles possibles).

## Décision et travaux autorisés

Comparer un helper fixe autour de l'outil installé et un collecteur embarqué avec
bibliothèque compatible Go1.24, en vérifiant les sources primaires et les versions.
Aucune dépendance ou politique choisie implicitement. Le helper scellé R14 n'a pas
les droits/réseau nécessaires et n'est pas un collecteur eBPF existant.

Avant code : définir identité de fixture possédée (PID/démarrage, descendants),
filtrage noyau avant émission, attribution au contenu/action et distinction entre
observation et décision. Aucun scan/attachement global. Définir données collectées,
minimisation, pertes/ruptures, nettoyage, reprise, refus de PID réutilisé/étranger,
et mesure du surcoût sur fixture isolée. Les politiques nouvelles doivent être
persistées, Admin et validation Go/API/CLI commune ; unités/zéro/bornes explicites.

Une indisponibilité réelle doit produire available:false et le code3 du contrat
T7, jamais des événements synthétiques annoncés noyau. Doubles isolés possibles
pour les contrats, distincts d'une collecte native. Ne pas répéter cette même
sonde sans précondition changée.

Aucune élévation Linux autorisée par la seule autorité sur droits/budgets Swarm.
Préparer une mise en route bornée et révisable avant toute décision réellement
manquante sur l'environnement. Ne pas lancer sudo, modifier sysctl/capabilités ou
observer hors projet. Continuer les contrats et la qualification indépendante
possible ; R20 reste ouvert tant que la collecte et ses exigences ne sont pas
observées. T8 peut préparer sa matrice tout en gardant ce critère en échec.

## Tranche préalable — attribution et réception Unix

Décision du 16 octobre : séparer le contrat de réception du choix du backend.
Un adaptateur bpftrace installé évite une dépendance, mais exige un protocole et
un filtrage fixes ; une bibliothèque Go compatible permet un contrôle direct,
mais ajoute un backend à vérifier. Aucun des deux ne résout le refus noyau.
La plus petite étape commune autorisée est donc une primitive interne testable,
sans activation de collecte ni modification de la supervision existante.

Codex implémente : source issue d'un Agent actif (mission/tâche/tentative,
PID/démarrage/hôte) et digest fourni par l'appelant de confiance ; pair Unix lié
à UID/PID/démarrage du collecteur attendu ; trames strictes sans contenu brut ;
attribution et source relues avant acceptation ; ordre, pertes et rupture visibles.
Les descendants ne sont jamais autorisés par leur seul parent : chaque PID exige
son propre enregistrement de confiance. Aucun endpoint d'injection public.

Recette interne : socket Unix réel entre processus possédés ; autre pair de même
UID refusé, identité périmée/refus de lecture, mauvaise attribution, doublons,
rejeu, pertes, concurrence et déconnexion. Tests sous race et compilation ciblée.
Ces observations sont des fixtures de transport, jamais des événements noyau.
Le raccord au lanceur, le filtre noyau, le backend, l'UX/politique d'exécution et
la mesure de surcoût restent à livrer ; R20 demeure ouvert.

### Écart du producteur trouvé pendant cette tranche

`ingress-agent-before.out` : la première garde attendait `desired="running"`.
Le véritable producteur insère l'agent sans ce champ SQL, dont le défaut est
`""` ; `desired="stop"` porte ensuite une demande d'arrêt. La fixture initiale
reproduisait notre hypothèse au lieu du producteur. Le cas nommé
`TestKernelObservationAgentLifecycle`, construit avec `prepare` en racine isolée,
a refusé cet agent réellement actif avant correction. La garde suit désormais
le producteur et la lecture relit les colonnes SQL autoritaires, y compris stop.
Le cas reste dans la recette ; aucun état de mission active n'est utilisé.

Prévention : les fixtures de jointure doivent utiliser le producteur existant
pour les valeurs implicites, pas recopier la même hypothèse que le consommateur.
Cette vérification complète la règle commune de lecture producteur/validation.
Propriétaire Codex ; effet final à qualifier depuis le reçu nommé, pas depuis
la première série verte qui ne couvrait pas ce cas.

## Résultat intermédiaire vérifié

`verification-ebpf-20261016/ingress-race-final.json` : 37 cas sous race
(9 principaux, 28 sous-cas), helper parent explicitement ignoré et non compté.
Réception d'un enfant réel, autre PID du même UID, UID/démarrage erronés, source
périmée ou illisible, arrêt SQL persistant, mauvais contenu/ordre, concurrence,
pertes, trames multiples et annulation sont vérifiés. Les messages restent des
fixtures, pas des événements noyau. La première série est historique ; elle
n'est pas additionnée à la finale. Les échecs avant implémentation et avant la
correction `desired` sont conservés.

Compilation canonique, vet, douze contextes et intégrité de la matrice R19 passent
dans les reçus `ingress-{build,vet,contexts,mapping-integrity}.json`.
La matrice vérifie ses preuves liées sans relancer leurs tests ni qualifier le
serveur. Aucun fichier existant de supervision, aucune dépendance et aucune
politique d'exécution n'ont été changés par cette tranche. Le binaire construit
est distinct de l'aperçu qui continue de servir la version R19.

Suite concrète Codex : choisir le backend fixe et construire son lanceur possédé,
son protocole prêt/arrêt et le filtre noyau avant émission ; raccorder la source
vivante sans décision métier issue des événements. Contrat public et politiques
Admin/API/CLI avant activation, test d'indisponibilité avec code 3, puis recette
noyau et mesure du surcoût quand l'environnement le permet. Ne pas relancer la
sonde EPERM inchangée ni demander un privilège abstrait avant ce lancement
révisable. Les travaux T8 indépendants restent possibles, avec R20 ouvert.

## Backend fixe — tranche suivante

APEX appliqué avant activation produit. Choix : cilium/ebpf v0.19.0, compatible
Go1.24.3 et x/sys0.31 existants, plutôt que l'analyse du texte bpftrace et de ses
diagnostics de perte. Aucun compilateur C, ELF client ou script arbitraire.
Cela ne revendique ni maintenance fournisseur ni collecte déjà qualifiée.

Périmètre initial : thread principal enregistré, sans héritage des descendants.
Trois tracepoints fixes : fork, openat, connect. Perf dirigé vers ce thread,
filtre TGID/TID avant émission ; aucun argument, chemin ou contenu de syscall.
Pidfd, PID/démarrage et porte inactive jusqu'à validation protègent la cible.

Ring buffer explicitement fourni au backend (contrainte technique pages et
puissance de deux). Compteurs atomiques tentatives/émissions/pertes ; perte finale
visible sans événement suivant. Fermeture porte/perf/reader/programmes/maps sans
pin ni changement memlock/sysctl. Tests : filtre, identité avant acquisition,
échecs partiels/nettoyage, statistiques et décodage. Une tentative native unique
du nouveau backend sur fixture possédée peut établir son indisponibilité ; pas
sa qualification noyau. Lanceur séparé, politiques publiques et recette complète
restent requis. L'insertion initiale de cette note a échoué sur une ancre coupée
en deux lignes ; lecture puis correction, aucun fichier modifié par cet échec.

### Résultat du backend interne

`backend-race-final.json` : 59 cas significatifs sous race (15 principaux,
44 sous-cas), dont les 37 cas de réception rejoués ; ne pas les additionner aux
séries historiques. Le contrôle natif inclus vérifie seulement l'indisponibilité
et son attribution. `backend-native-availability.json` : pidfd ouvert pour le
processus de test possédé, création du ring refusée EPERM, avant programme/perf.
Aucun événement noyau ni code CLI public observé. Le texte de la bibliothèque
évoque memlock ; cette suggestion ne prouve pas la cause et n'a pas été appliquée.
Le contrôle est distinct de l'ancienne sonde ARRAY : nouveau backend/canal réel,
une seule tentative avec preuve conservée, aucune relance identique.

La garde « refus avant acquisition » exige aussi une erreur hors acquisition,
pour qu'un simple refus EPERM plus loin ne masque pas une erreur de validation.
Les tests de filtre exécutent le préfixe des instructions et leur sérialisation ;
ils ne remplacent pas le vérificateur noyau. Nettoyage inversé des acquisitions
partielles, pointeur de ressource nil, erreurs de fermeture/concurrence,
échantillons invalides et pertes finales sont couverts sur doubles explicites.

Build, vet, intégrité des modules, douze contextes et intégrité R19 passent dans
`backend-{build,vet,modules,contexts,mapping}.json`. Go et x/sys restent inchangés.
Les bilans natifs historiques des autres lots ne sont pas réexécutés globalement.
Le nouveau binaire reste distinct de l'aperçu R19 ; aucun serveur redémarré.

Suite : raccorder ce backend à un lanceur séparé fixe et à la réception Unix,
avec source vivante/autorité fraîches et fermeture des FD. Définir la politique
persistée et Admin/API/CLI avant activation ; aucune URL/commande/PID client
arbitraire. Produire le refus public available:false/code3 puis la recette
noyau, pertes, exclusion étrangère, nettoyage et surcoût. R20 reste ouvert ;
aucune élévation ni nouvelle orchestration implicite. T8 indépendant maintenu.

## Parcours public fermé — diagnostic possédé

Décision de reprise : livrer le diagnostic public sur une fixture créée par le
helper Swarm, sans sélecteur de PID/agent/commande. L'observation des tentatives
réelles reste distincte. Ce périmètre permet le refus API/CLI/UX observable sans
créer de mission native ni observer un processus existant. Le helper ne cible
que son propre thread principal et reçoit uniquement un canal de contrôle.

Politique `kernel_check` : enabled, timeout_seconds, ring_bytes, valeurs explicites
positives et cohérentes avec pages/uint32 ; défaut inactif, 15s et 65536octets,
omission du bloc conserve. Admin/I3 configure et lance. Aucun polling ni retry.
Le diagnostic est synchrone et borné par son délai ; aucune nouvelle politique
de concurrence ni ordonnanceur ajouté. Une exécution ne modifie pas le métier.

Tests avant code : validation/aperçu/persistance sans mutation sur refus,
autorité actuelle, runner hors verrou et révision fraîche au retour, helper fixe,
annulation/perte parent/FD/environnement, réponse unavailable réelle avec code3,
FR/EN/thèmes/mobile et scénario public. Réponse indisponible n'est jamais une
preuve d'événements noyau. L'API d'observation d'agents restera non livrée ici.

### Écarts vérifiés pendant le raccord

`check-second.out` : la fixture changeait Revision sans History ; état incohérent refusé conformément. La fixture passe maintenant par trustConfig. `check-empty-before.out` reproduit un défaut réel du nouveau client : verdict positif sans événements rendu code0. La validation commune impose maintenant cible unique, source/digest/nonce cohérents, séquence/horloge, trois catégories, absence de perte et nettoyage. Le cas reste une garde ; originaux conservés.

### Incident du parcours navigateur

`check-browser-first` et `check-browser-diagnostic` observent un 403 ; diagnostic de l'en-tête : autorité transmise différente, sans défaut moteur établi. `check-browser-final` ajoute une assertion de saisie et observe seulement `adm` au lieu du jeton complet. `check-focus-before` reproduit le focus repris par l'événement `close` différé du dialogue Admin. Correction générale : rendre le focus seulement s'il reste dans le dialogue, sur body ou sur le déclencheur ; préserver un autre champ déjà focalisé. Garde de régression explicite conservée, sans assouplissement d'autorité.

La garde anglaise attendait « power of two » au lieu du libellé réel « power-of-two » ; recette corrigée, aucun changement produit pour cet écart. Une insertion documentaire a écrit le FR puis échoué sur l'ancre EN réelle « Verification and remaining work » ; lecture puis reprise ciblée, originaux et contrats conservés.

### Résultat public fermé vérifié

`check-verification.json` : 223 cas Go distincts sous race, 63 principaux et160 sous-cas, en groupes10 diagnostic/184 partagés/29 connecteurs. Les184 comprennent126T2 et58 backend/réception ; les anciennes59 incluaient une sonde native non répétée ici. Six contrôles publics kernel, sept scénarios pilote et55 contrôles navigateur (32 kernel/23 pilote) passent ; aucun diagnostic inattendu. Les captures Admin en viewport sont inspectées ; les captures clip erronées restent conservées. La modification du focus est vérifiée par le cas qui échouait avant et par Échap.

Build/vet/i18n/douze contextes/diff et manifeste1483 sources, contrôle nommé sous race, passent. Aucune exécution unique223 ni acceptation indépendante. Les tests navigateur portent sur le binaire focus ; le binaire final ne change que la matrice statique et les références architecturales. Branches Go/UI concernées inchangées ; le CLI/API final est réellement rejoué. La matrice v2 lie un nouveau reçu d'effets aux tests126T2/29 connecteurs exécutés, sans modifier les originaux.

`check-preview-process.json` : aperçu possédé remplacé après identité/argv/empreinte vérifiés, reçu écrit dès spawn. Deux demandes/un effet et octets métier préservés, kernel_check inactif, API/HTML/assets vérifiés. IAB non rechargé observé. Aucun droit Linux ajouté. R20 reste ouvert sur attribution aux tentatives métier, noyau positif, exclusion étrangère, pertes/reprise et surcoût. T8 indépendant préparé dans qualification-globale-replan.md ; RETEX final non déclaré.

## Liaison au registre des tentatives — tranche interne

Reprise : le diagnostic possédé est livré ; le transport prend encore un digest
fourni par l'appelant interne. Comparaison : ajouter un nouveau sceau persistant
au lanceur (modifier les anciens enregistrements et deux chemins PTY) ou dériver
un identifiant de contenu depuis le snapshot autoritaire déjà persisté. Choix
borné : dérivation versionnée, sans nouveau champ ni modification de lancement.
Elle décrit les entrées enregistrées (prompt et configuration), jamais l'image
binaire effective ni les entrées interactives futures.

Contrat avant code : lecture SQL unique agent/travail/pause ; colonnes SQL de
statut et arrêt autoritaires, identités agent/travail/tâche cohérentes, dernière
tentative de tâche toujours running, mode batch seulement. Empreinte canonique
liée aux identifiants et aux digests des entrées enregistrées ; aucun contenu brut
restitué. Source PID/démarrage/hôte actuelle et relecture après I/O. Factory du
récepteur issue de cette source, sans paramètre digest client. Une pause ou une
erreur de lecture refuse. Aucun nouveau collecteur, API d'injection, politique,
activation ou privilège dans cette tranche. Le futur lanceur métier doit encore
utiliser cette factory ; observation noyau et surcoût non qualifiés.

Assertions : préparation réelle en racine isolée ; digest stable sous heartbeat,
changement d'entrée détecté, mauvais rôle de source/identité/champ/ancienne
tentative/pause/arrêt refusés, lecture sans mutation. Connexion Unix possédée,
refus de trame attribuée à un autre contenu et retrait pendant attente ; aucun
octet noyau simulé. Codex propriétaire ; originaux conservés et tests nommés.

`attempt-source-first.out` conserve un refus erroné d'une fixture issue de prepare : Task.Status vaut running, mais Attempt.Status vaut recorded jusqu'à outcome, selon store.go. La garde vérifie désormais ces deux contrats distincts et Ended vide ; l'identifiant doit rester celui de la dernière tentative. Le test positif passe toujours par le producteur réel, sans modifier son statut pour satisfaire le consommateur. Les recherches ont aussi tenté des chemins supposés absents : utiliser désormais les chemins retournés par rg --files / rg -l, pas des noms présumés.

### Résultat de la liaison interne au registre

`attempt-source-verification.json` : 75 cas Go distincts sous race (14 principaux/61 sous-cas), dont38 nouveaux cas de source et37 cas de réception rejoués. Ne pas ajouter ces37 aux campagnes précédentes. Un processus de fixture est réellement préparé/lancé/arrêté par les opérations moteur ; source après arrêt refusée. Les autres trames sont des messages Unix possédés, jamais annoncés noyau. Build/vet/modules/douze contextes/intégrité R19/diff et manifeste1485 sources avec contrôle nommé passent.

La factory n'est pas encore activée par un lanceur métier et aucune politique d'observation d'agents n'est introduite. Le digest décrit les entrées enregistrées, pas l'image binaire ni les adaptations d'arguments effectives. Tâche running / tentative recorded sont des contrats distincts validés sur le vrai producteur. Le noyau reste non qualifié, sans nouvelle sonde ni privilège. Aperçu public précédent identifié et conservé sans redémarrage.

La campagne fonctionnelle T8 commune est conservée séparément dans verification-globale-20261016/public-campaign.json : un binaire, sept étapes de recettes en racines distinctes, statut PARTIAL. Les recettes d'identité et l'indisponibilité eBPF précédentes ne sont pas annoncées rejouées. Les erreurs du conducteur et sa reprise sans nouvel effet restent dans qualification-globale-replan.md.

## Collecteur métier explicite — contrat avant code

Comparer l'activation automatique dans supervise (modifie tous les départs et
les budgets) à un appel I3 explicite. Choix : `trust kernel-observe` / POST action
`kernel-observe`, corps strict `{agent_id}` dans la racine courante, session
opérateur/CSRF habituels ; aucun PID, commande, digest ou option de sonde client.
La source vient de la factory du registre. Aucun départ de mission n'est créé.

Politique distincte `kernel_observation` persistée/Admin : enabled, window_seconds,
timeout_seconds, ring_bytes, max_events. Défaut inactif,5s/15s/65536octets/128.
Fenêtre et délai total positifs, fenêtre inférieure au délai ; ring technique
pageOS..2^31 puissance de deux, maximum d'événements positif représentable.
L'omission conserve ; champs internes explicites, zéro/null/fraction refusés,
même inactif. Pas de polling ou retry automatique, ni plafond de concurrence
caché. La limite d'événements est une politique visible ; aucune troncature muette.

Helper fixe séparé par pipes/Unix, environnement minimal, FD fermés et perte du
parent fermée. Le parent authentifie I3/politique/révision et source avant départ,
avant armement et chaque restitution, sans verrou métier pendant l'attente.
Messages de contrôle et d'observation distincts ; pair/démarrage/nonce vérifiés.
Le helper ne reçoit aucun jeton métier ni fournisseur. La fenêtre normale arrête,
lit les compteurs puis ferme ; délai total/annulation tue le groupe et refuse.
Statistiques distinguent pertes noyau et événements non restitués sous limite.
`complete` et qualification globale restent faux ; disponibilité ne prouve pas
que toutes les catégories ont été vues. Indisponibilité : API résultat explicite,
CLI code3. Aucune élévation ou nouvelle délégation implicite.

Assertions avant implémentation : refus sans mutation avant helper (corps,
autorité, politique, source/pause/arrêt), cycle exact de contrôle/événements,
révocation/révision après attente, limite visible, pertes, nettoyage/parent mort,
entrée incohérente/rejeu. Doubles marqués, fixture possédée isolée ; pas de
nouvelle sonde inchangée ni observation étrangère. Admin sauvegarde/réouverture,
FR/EN/thèmes/mobile et recette publique. Le positif noyau/surcoût reste ouvert.

### Écarts de cette reprise

`observe-cleanup-before.out` reproduit un défaut de contrat : une erreur de fermeture après acquisition partielle pouvait être confondue avec une indisponibilité nettoyée. L’erreur de fermeture porte désormais un type commun ; diagnostic et nouveau collecteur refusent de qualifier un nettoyage échoué. Garde de régression ciblée, tests en cours.

`observe-client-before.out` constate l’absence de validation de réponse dans le nouveau composant ; les champs obligatoires, attribution, politique, compteurs et qualification partielle sont maintenant contrôlés avant affichage. Garde client après correction passée. Les tests de protocole possédé passent, sans BPF ; la série race reste en cours.

Deux recherches ont encore supposé des noms de fichiers inexistants. Cause de méthode : utilisation d’un nom supposé avant inventaire. Propriétaire Codex ; reprise par rg --files puis lecture du chemin réellement retourné, sans diagnostic produit associé. Les commandes et leurs erreurs restent dans le fil.

### Résultat du raccord explicite, vers04:32UTC

Le lanceur fixe consomme maintenant la factory du registre : kernel-observe, politique kernel_observation persistée/Admin et parcours CLI local/distant/API/UX. Portée thread principal enregistré, aucun PID/commande/digest client. Fenêtre, limite d’événements et pertes distinctes ; aucun journal durable ni couverture complète promis. Protocole possédé, mauvais digest, changement de révision, annulation et mort du parent contrôlés. Le helper est exécuté depuis l’image courante ouverte par FD.

141 cas Go collecteur/diagnostic/backend/source/réception passent sous race,155 régressions T2/deux connecteurs et1 parcours public distinct :297 cas distincts en trois groupes, pas une exécution297. Les quatre helpers/prérequis ignorés sont exclus. Entrées Go et web figées durant exécution. Le parcours public utilise une source de test possédée enregistrée, sans fournisseur : CLI distant/API réellement disponibles, code3/available:false, attribution exacte, aucun événement synthétique, état inchangé. Cette nouvelle tentative vérifie le nouveau consommateur public ; elle n’est pas une relance de la sonde antérieure dans l’espoir d’une réparation.

33 contrôles navigateur FR/EN/thèmes/largeurs passent, sans diagnostic inattendu ; réponses de collecte interceptées explicitement comme fixtures navigateur. Sauvegarde/réouverture, refus malformé, annulation/focus et champs visibles contrôlés ; captures finales inspectées. Build/vet/i18n/douze contextes passent. Matrice R19 : ancien reçu correctement refusé sur trust_actions.go, nouveau reçu observe-effects-verification.json issu des155 régressions ; autres preuves conservées. Consolidation/manifeste final encore à terminer avant bilan.

Une sonde minimale Go montre que json,string normalise 01 en1 ; aucune modification moteur inventée pour cette observation : la forme canonique stricte est requise pour les trames et contrôlée dans leur parseur. Les compteurs produits restent des chaînes décimales, et le nouveau client web refuse les formes non canoniques.
