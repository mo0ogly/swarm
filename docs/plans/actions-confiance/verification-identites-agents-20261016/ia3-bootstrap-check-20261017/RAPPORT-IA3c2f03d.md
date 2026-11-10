# IA3c.2f-03d — précontrôle public du bootstrap, sans autorité implicite

17 octobre 2026. Codex seul ; checkout canonique, HEAD initial
f32b9f8fa6e99f75845f57872a7f8a56a0ef9dc6. Travaux mémoire/UX présents conservés.
**Précontrôle implémenté et vérifié ; service d’autorité réel non activé.** Aucun
compte, clé, fournisseur, mission, processus tiers ou installation active modifié.
Aucun commit, push, déploiement ou appel de modèle. Auto-inspection, sans revue
indépendante ni acceptation globale.

## Résultat observable

La commande `swarm identity bootstrap-check EMPREINTE_SHA256 --input
/chemin/absolu/bootstrap.json --json` est routée avant résolution/ouverture du
projet. Elle valide une déclaration V1 et son stockage existant, sans créer de
lock ou de Store. Configuration passée par projet/import, stdin, serveur distant
ou demande d’activation ne devient pas une source d’autorité.

Manifeste exact : schema_version, pins, service_uid, controller_uid,
configuration_directory, witness_directory. Pins réutilise le contrat existant
(installation, clé publique, principal humain déclaré). Les UID doivent différer
et service_uid égaler l’EUID du processus de contrôle. Cela ne démontre pas que
le contrôleur ou les fournisseurs tournent réellement sous l’autre UID.

L’empreinte attendue vient d’un contrôle d’installation approuvé indépendamment.
La CLI ne démontre pas cette approbation et ne doit pas apprendre le pin de
l’objet à accepter. Fichier régulier privé 0600, un lien, propriétaire service,
sans symlink ; parent, configuration et témoin privés 0700, chemins distincts/non
imbriqués et handles désignant des objets distincts. Ancêtres protégés selon le
contrat du stockage, y compris exception sticky root telle que /tmp.

`enrollmentconfig.OpenPrivateDirectory` contient le parcours auparavant inclus
dans Open. Le stockage et le bootstrap utilisent cette même frontière ; ouvrir
un coordinateur pour vérifier seulement les chemins aurait créé un lock.
La validation ne crée aucune clé, session, identité, configuration métier ou
autorisation. Le JSON retourne storage_verified=true et explicitement false pour
human_authentication_verified, confinement_verified et service_started. Refus :
aucun résultat, erreur conservée. Handles fermés, chemins à revalider à l’usage.

Deux fichiers existants modifiés : engine/main.go et
enrollmentconfig/store_linux.go. Cinq ajouts : enrollmentconfig/path_linux.go,
enrollmentbootstrap/bootstrap_linux.go et son test, puis
engine/agent_identity_bootstrap_linux.go et son test. Les sources 03c et travaux
des autres chantiers ne sont pas édités. before.tar.gz contient les originaux
locaux ; les diffs de cet incrément sont comparés à cette référence, pas à une
copie supposée propre du dépôt.

## Vérification et portée

| Critère | Résultat | Preuve / limite |
| --- | --- | --- |
| Bootstrap + stockage | 141 noms PASS normal/race-05 réutilisés (35 + 106), zéro FAIL/SKIP | library-reuse.json : 15 entrées de dépendances locales/test inchangées ; resources absent du graphe |
| Moteur concerné | 126 noms PASS normal/race-07, zéro FAIL/SKIP | 118 identité + 5 cadrage + 1 transmission/réouverture + 2 aide ; fixture fournisseur locale sans modèle |
| Stabilité des entrées | 1 123 entrées effectives identiques avant/après chaque lot | `*-inputs.json` : Go, modules et 274 ressources ; overlay de 137 assets web seulement, aucun Go substitué |
| Précontrôle pur | Lecture répétée et copie canonique remplacée, aucun lock/fichier créé | Tests Load, contrôle de fichiers et répertoires ; pas de preuve d’autorité humaine |
| Refus de format/stockage | Version/pins/local/same-UID, paths relatifs/imbriqués, null/casse/doublons/extra/trailing ; mauvais pin, symlink/hardlink, modes, FIFO, UID et source inexistante | Chaque injection de chargement part d’un témoin valide ; résultat nul et erreur exigés |
| CLI du binaire compilé | Acceptation valide, mauvais pin refusé, aucun projet implicite ni mutation | `public-cli-03-run.json`, `cli-*-03.json/.stdout/.stderr` ; binaire de fixture /tmp jamais installé |
| Alias physique config/témoin | Témoin valide accepté, bind mount alias refusé | `namespace-valid-03.*`, `namespace-alias-03.*` ; namespaces utilisateur/montage/PID/réseau privés, aucune modification de montage hôte |
| Analyse et compilation | vet/build-07 PASS | Entrées stables, binaire et hash liés à ces sources ; aucun serveur actif remplacé |
| Contrat des méthodes | Contrôleur commun PASS, cinq tests/sous-tests | `workflow-learning-07-run.json` et sorties ; contextes assemblés réellement contrôlés sans fournisseur |
| Aide partagée | Deux tests existants PASS normal/race, inclus dans les 126 | Même routage main.go, aide sans projet ni mutation métier |
| Documentation | Contrats/installation FR/EN, cible, plan, note scientifique ; deux schémas rendus et inspectés | [FR](diagram-fr.png), [EN](diagram-en.png), checks-final ; aucun écran/Admin modifié ou vérifié |

267 noms par mode, dont 36 nouveaux au précontrôle ; ne pas cumuler normal et race comme des
attaques distinctes. Les noms comptent tests et sous-tests, y compris le helper
03c inactif sans environnement dédié. Les nouveaux tests se servent de racines
temporaires isolées ; les injections ne touchent aucune mission.

L’expérience de bind mount utilise un UID 0 **dans un namespace utilisateur**
mappé à l’utilisateur hôte non privilégié. Un /tmp privé donne des ancêtres
cohérents dans ce namespace. La même fixture sans alias passe avant l’injection
de l’alias ; le refus n’est donc pas déduit d’un environnement déjà invalide.
Cela ne crée pas de compte système distinct, n’authentifie personne et ne prouve
pas le confinement d’un fournisseur réel. Les manifests de fixture et leur pin
calculé servent seulement à vérifier le protocole, pas l’approbation opérateur.

Go utilise -timeout=0 ; les helpers 03c gardent -test.timeout=0, aucun délai
global ou nombre de tentatives ajouté. Anciennes deadlines de socket IA2 (5 s)
inchangées, sans expiration dans ces lots. Aucune configuration opérateur retirée,
nouveau plafond politique ou test interrompu présenté comme PASS.

## Échecs conservés et correction de méthode

Lot 02 : 139 noms PASS, un FAIL, donc **FAIL**. Les 106 gardes config passent.
Le Load censé réussir refuse son parent. Le diagnostic 03, ciblé et instrumenté,
échoue encore mais apporte les modes originaux : parent temporaire 0775 alors
que les feuilles sont 0700. Le rejet de la frontière existante est correct.
Les anciennes fixtures config appliquent déjà Chmod(0700) ; les nouvelles ne
le faisaient pas. Le correctif protège leurs racines, sans modifier le produit.

Les cas négatifs auraient tous pu passer sur cette seule précondition invalide.
Chaque injection Load établit désormais un Load valide de référence avant de
changer le défaut ciblé. FIFO et null UID isolé ajoutés ; le test CLI change son
répertoire courant vers une racine privée, pour prouver l’absence de projet.
Le lot 04 contient ces corrections et tous ses noms ont effectivement terminé.
Les archives, stdout/stderr et FAIL 02/03 restent conservés.

Le collecteur 01 a échoué **avant Go** : la boucle d’archivage réutilisait name,
écrasant le nom de lot avec un chemin. Script original conservé, path_name
distinct, reprise sur label neuf ; aucune exécution de test inventée. Un patch
groupé a aussi été refusé sur une ligne vide supposée absente ; les hashes
confirment l’absence de modification des deux producteurs, puis l’extraction
est bornée sur leur contenu réel. Les chemins de quelques lectures supposées
étaient absents ; la recherche a été corrigée par rg --files. Ces défauts de
notre méthode ne sont ni des pannes produit ni une acceptation du service.
`method-incidents.json` et les extraits du tour conservent ce diagnostic.

## Correction de la couverture des entrées embarquées

Le premier collecteur suivait internal/cmd et les modules : 848 entrées, mais
oubliait resources.go à la racine et les ressources go:embed. La coordination
M32 sur ses assets figés a conduit à inspecter ce producteur et go list -json . :
274 ressources, dont 137 web. Les preuves 04 restent conservées comme preuve de
comportement et de stabilité Go, avec cette limite ; aucune couverture complète
du binaire n’en est déduite. Le code produit ne change pas pour corriger la preuve.

Le lot 05 suit 1 123 entrées : sources/modules, racine et ressources. Les 137
assets web sont figés dans web-fixture et seuls leurs chemins sont substitués par
l’overlay ; les fichiers Go ne sont jamais substitués. Les autres ressources sont
hashées avant/après et liées au commit ou à un snapshot explicite pour reconstruire
le candidat. La liste effective EmbedFiles est recontrôlée à chaque lot.
Le compilateur Go local construit embed.Files avec fsys.Actual, ce qui applique
l’overlay aux fichiers embarqués ; sa source et sa version sont conservées.

Tests normal/race, vet/build-05, CLI du binaire fixture-05, namespaces, méthodes
et deux gardes d’aide sont vérifiés sur cette même entrée complète. Le binaire
ne prétend pas refléter les dernières modifications UI concurrentes. Ce n’est
pas une qualification visuelle de l’interface ou un déploiement. L’apprentissage
I2f03d-04 est réalisé/vérifié dans le collecteur : inventorier le graphe réel de
compilation, jamais seulement les répertoires Go supposés. La propagation au contrat
commun et à la consigne personnelle est ensuite réalisée après la clôture M32,
avec nouvelle qualification du consommateur et du binaire (voir ci-dessous).

## Recette et capitalisation

Depuis le checkout canonique, utiliser un label neuf pour conserver les preuves :

```sh
python3 docs/plans/actions-confiance/verification-identites-agents-20261016/ia3-bootstrap-check-20261017/run_checks.py recheck-01
python3 tools/agent-workflows/check.py
git diff --check
```

Le collecteur contrôle stabilité et tests nommés non vides, archive les sept
sources possédés et refuse d’écraser un record existant. verify_cli.py utilise
le binaire du label fourni en second argument (07 pour le lot final), ou un label de compilation explicite en second
argument ; conserver chaque version de recette et ses originaux. Le binaire
en cours d’installation ou le serveur actif ne sont jamais choisis implicitement.

I2f03d-01 sépare observation et création du stockage ; I2f03d-02 établit un témoin
valide avant chaque injection. Ces apprentissages sont vérifiés, transmis par
tests, contrat, plan et note scientifique. La mise à jour des fichiers ne modifie
pas les processus déjà démarrés. I2f03d-03 est seulement une hypothèse de
durcissement statique : ReadAll alloue le fichier privé avant comparaison du pin.
Avant exposition à des entrées non approuvées, qualifier lecture d’empreinte en
flux puis liaison aux octets décodés ; aucune limite métier cachée à inventer.
Propriétaire : Codex au futur raccord service, pas un défaut distant démontré.

Une demande de stabilité est reçue du chantier mémoire M32 pendant la qualification
finale. Nos sources Go sont stables après les lots 04 ; seules preuves et docs
sont terminées ensuite. Aucun message intersession n’est envoyé, conformément
à la demande ; un second message confirme la réception du point public et le
lancement de sa qualification, pas sa fin ou son acceptation.

## Capitalisation effective et contrôle du destinataire

La fenêtre M32 est confirmée close par son propriétaire. Les deux leçons sont
ajoutées au contrat commun embarqué, aux méthodes FR/EN et à la consigne
personnelle persistante /home/fpizzi/.codex/AGENTS.md. Le contexte AGENTS reçu
ensuite contient ces nouvelles clauses : réception observée pour cette session,
pas une preuve de mise à jour de tous les processus ou d’autres sessions.

Le premier texte fait réellement échouer le contrôle : corps planner 32034 et
subplanner 32037 octets, au-delà de la borne moteur existante 32000. Les deux
autres rôles passent ; le parent et deux sous-tests échouent. Le fichier et les
événements JSON originaux sont conservés (workflow-learning-06, derived.json).
Le paragraphe de preuve est reformulé sans retirer d’exigence ; 63 octets en
moins, aucun plafond relevé ni source tronquée. workflow-learning-07 passe.

La borne de 32000 s’applique au corps assemblé, avant en-tête de métadonnées :
les logs des cadres retournés mesurent planner 32196, subplanner 32202, worker
31355 et reviewer 18794 octets. Ce ne sont pas des mesures de limite fournisseur.
L’origine/qualification externe de cette borne et la faible marge du corps des
planificateurs restent un suivi distinct, sans nouvelle politique ou seuil.

Le lot final normal/race-engine-07 vérifie les quatre rôles et la transmission
au processus worker de fixture, puis le prompt et son empreinte après réouverture.
Le test compare le préfixe complet attendu au contenu réellement reçu par le
script et à celui conservé en base ; un fichier de méthode local hostile ne le
remplace pas. Aucun Claude/modèle réel, acceptation de tâche ou service d’identité
actif n’en est déduit. Les Go sont inchangés ; seule une donnée embarquée évolue.
Les 141 noms de bibliothèque 05 sont réutilisés avec graphe de dépendances et de
test vérifié, quinze entrées inchangées, aucune dépendance resources. Vet/build-07
et CLI du binaire fixture-07 lient le tout aux 1 123 entrées effectives courantes.

## Reprise du besoin initial

Le service réel reste non activé. Le point manquant est une donnée et une décision
d’installation : fournisseur humain existant et URL d’émetteur, ou absence de
fournisseur afin de choisir le parcours d’enrôlement adapté. La question a été
posée à l’utilisateur ; aucune réponse reçue au moment de ce rapport. Ni issuer,
subject, clé, compte ou approbation humaine ne sont inventés pour continuer.

Après cette désignation, raccorder la preuve humaine et l’autorisation métier,
puis provisionner le compte/service avec source protégée et vérifier le
confinement effectif, les lancements/délégations/révocations publics et setup/Admin.
Le choix local pilote et distant futur reste adopté. F01–F07, EP-01/04/05/07,
IA3–IA5 et RETEX global restent ouverts. Ce précontrôle ne clôt pas le chantier
ni ne démontre de nouveauté scientifique ou de supériorité sur un autre système.
