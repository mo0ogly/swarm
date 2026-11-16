# Gardes proposées des imports et originaux de fixture Gi

SOURCE PRIVÉE, NON IMPORTÉE/NON EXÉCUTÉE. Ce fichier décrit une attribution future,
pas une allocation. Les sources16 figées et le Workrévision4 de la vraie campagne
échouée restent intacts. [English](GI-SOURCE-TESTS.en.md).

## Deux cohortes et deux types de preuve

Le même gi_source_import_entry.py doit être utilisé sur assemblages16 et17, dans
un NOUVEAU processus Python -I -S -B pour CHAQUE première entrée : process,
retention, parent W/F DEFAULT, préparateur06, flow READY, consommateur
mission_manual_review et helper source. Ni suppression/remplacement sys.modules,
ni stub de module ni symbole exigé uniquement dans17. Failure/check/read_at sont
vérifiés dans leurs modules canoniques, pas réexportés pour accommoder le test.
Les fichiers existent sur les deux versions ; source manquante/setup refuse et
n'est jamais un RED de comportement. Le cycle16 observé en source peut causer un
ImportError de symbole initialisé trop tôt ; aucun RED runtime n'est encore observé.
DEFAULT/READY peuvent déjà passer sur16, ne pas qualifier toutes entrées RED.

Ces gardes importent réellement les modules et leurs dépendances ; elles ne
lancent aucune fonction candidate, fixture ou campagne. Le graphe W/F source est43
fichiers ; les gardes couvrant aussi mission_manual_review ajoutent ce module à
l'union d'admission (44). Les deux nouveaux scripts de tests sont en plus. Le JSON
par défaut physique doit être présent/admis, savedsettings réellement absents ou
qualifiés. fixture_area cache _REPO/_AREA avant tout chdir ; une garde verte n'est
ni sa mutation ni une qualification du placement W/F. Native extensions/libc et
interpréteur/origines effectifs restent des preuves distinctes du graphe statique.

La seconde cohorte utilise huit méthodes de test sur les vrais helpers/public
consumers de source16/17. Le programme inerte généré est exécuté DANS le processus
de test, avec stdin/stdout et cwd contrôlés ; aucun enfant ni modèle. Les réponses
public-call/agent/completion et SR1 sont DONNÉES SYNTHÉTIQUES : aucun moteur,
RootStore, SQL, claim, permission ou résultat natif authentique n'est fabriqué.
Les fichiers/SR1reader/read_at/hardlinks/fsync/closes ordinaires sont réels dans
une arborescence privée attribuée. Les fautes EACCES/EIO sont INJECTÉES, pas des
pannes OS reproduites. Cela ne prouve pas I0/Landlock/Authority, native wait ou W/F.

## Contrats discriminants, témoins avant fautes

1. Proposition embarquée : Root input absent, programme inerte dans cwdI0 de test,
   output exact figé même si le dict appelant change après génération ; vrai
   prompt témoin retenu. Dérive template refusée après ce témoin.
2. Planner : témoin valide avant chaque frameSHA/claim/PID/prompt altéré. Vrai
   lecteur SR1/NOFOLLOW, résultat public doublé ; refus sans modifier les
   originaux injectés, sans second appel ni binding de succès.
3. Worker/submit : completion doublée déclarée, vrai report/marker ; transfert
   exclusif exact, source intacte ; submit réel du helper face à API doublée.
   Report public altéré refuse sous SHAoriginal AVANT appel submit du double.
4. Source : témoin valide avant report déjà altéré, completion inconnue et
   changement après premier read/fstat/close. Seams OS communes16/17, pas alias
   read_at réexporté. Refus sans destination ; reprise du consommateur sur nouveau
   contexte de preuves, originaux réellement sauvegardés/restaurés, sans nouveau
   provider/claim ni refund. L'agent est relu via le même double de lecture seule.
5. Cibles : témoin hardlink avant chaque cible différente/égale/dangling symlink ;
   EEXIST et inode/bytes/lien d'origine conservés. Reprise seulement vers nomfrais.
6. Link : témoin avant faute injectée AVANT ou APRÈS vrai link. L'erreur après
   effet laisse une vraie destination visible bien que link_return n'ait pas été
   observé. Ne pas lire destination_link_observed=False comme absence/rollback.
7. Close : témoin avant faute après vraie fermeture de stage ou lecteurtarget.
   Cause EIO exacte/projetée, aucune répétition de close ambigu ; aprèslink,
   destination réelle reste visible. Reprise vers cible fraîche, original préservé.

Les générateurs de données ne remplacent pas l'algorithme testé. Aucune assertion
ne compare la source du helper à son implémentation. Ne pas exécuter ces nouveaux
APIs sur061b pour déclarer un faux RED parce qu'ils y sont absents. Le vrai échec
W/F FileNotFoundError061b reste le contre-exemple d'exécution distinct, déjà conservé.

## Recette permanente proposée et admission

recipes/gi-source-fixture-originals.json contient sept premiers-imports isolés puis
une cohorte explicite de huit méthodes, sans discovery large, skip, cap ni watchdog.
La recette est nouvelle et NON EXÉCUTÉE. Pour baseline16, root doit conserver les
échecs par entrée et leur cause réelle ; la recette17 attend toutes gardes0, elle
ne doit pas faire disparaître un refus16 ni transformer setup en preuve RED.

Variables à fournir par l'allocation, sans secret et sans fallback :
SWARM_GI_SOURCE_IMPORT_RECEIPTS (parent privé neuf déjà admis),
SWARM_GI_SOURCE_TEST_AREA (zone de fixtures UNITAIRES neuve admise, jamais /tmp ni
Root de mission), SWARM_GI_SOURCE_TEST_RECEIPT (fichier neuf). Les unités n'adoptent
pas le resolver de placement des vraies fixtures W/F : ne pas transférer cette
observation à une campagne. Tous les répertoires de tests restent comme preuves.

Root doit vérifier hashes/source/config/UUID/espace/verrous/interpréteur, fermeture
réelle et origins AVANT imports. Les scripts retiennent les origins réels, _REPO,
_AREA et résultats sans écraser de receipt. Une publication/close/origine refusée
laisse preuve incomplète, jamais PASS ; l'appelant conserve wait/status/canaux AVANT
validation faillible des origins. Ouvrir une receipt neuve en x+fsync ne garantit
pas sa complétude sur ENOSPC. Aucun replay non attribué ni nouveau budget autorisé.

Passage ultérieur au vrai W requiert revue source, réception des preuves de ces
cohortes et son admission source/profile/OS/I0/Authority/storage/coût propre. Les
huit méthodes sur doubles ne constituent pas un témoin réel de source W/F.

## Huitième garde proposée de diagnostic (R09-16-DIAG)

La relecture du temporaire, auparavant hors enveloppe, conserve maintenant la
cause originale avec son nom aléatoire, son identité de création si observée et
l'étape tentée. Un fstat du descripteur nouvellement créé collecte cette identité ;
son propre refus est conservé et ferme une seule fois le descripteur. L'identité
de création n'affirme pas l'identité actuelle du chemin. Les erreurs de lecture,
fermeture et comparaison restent imbriquées dans Failure canonique.

Le diagnostic distingue tentative de lien et retour réussi du lien ; aucun refus
ne prouve absence ou rollback, y compris si la cible préexistait. Pas de retry,
écrasement, suppression supplémentaire ni changement d'octets, hash ou Authority.
L'unlink historique du temporaire après validation reste inchangé : un refus de
fsync ultérieur peut donc laisser la cible visible et le temporaire déjà supprimé.
Les gardes proposées dans GI-SOURCE-TESTS couvrent témoin, refus et reprise vers
un nouveau nom en conservant le temporaire refusé lors de la relecture. Elles ne
sont pas exécutées ici ; aucune qualification de17 ni reprise W/F n'est acquise.

La huitième garde couvre aussi le refus injecté du fstat de création : cause
originale, identité inconnue, temporaire vide conservé et reprise vers nom neuf.
