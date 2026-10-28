# PAEA — incrément 55 : lecture du paquet opérateur et remise de service

Reprise54 installée/vérifiée dev192, preuves bornées au binaire réel et à la
note PAEA54 inchangée ; une note étrangère ajoutée ensuite reste en attente
du prochain build. Codex seul, sans GLM/délégation/commit/push. PA-02 partiel,
0/17 globaux et helpers UID/humain/E2/E3/PA13/RETEX ouverts.

Le paquet privé et sa lecture durable existent en bibliothèque ; la CLI ne
permet encore ni son inspection protégée ni l’export du seul artefact de
service. La création initiale requiert une garde de pin indépendant avant
publication : elle reste explicite dans la suite, sans improviser une adoption
depuis les octets reçus ou une régénération d’entropie après résultat incertain.

Tranche : `identity operator-check PIN --input DOSSIER_OPERATEUR`,
`identity service-export PIN SORTIE_PRIVEE --input DOSSIER_OPERATEUR`,
`identity service-export-reconcile PIN SORTIE_PRIVEE --input DOSSIER_OPERATEUR`.
Le lecteur relit/synchronise le paquet sous pin indépendant, ferme avant retour
utilisable et exige que son UID contrôleur déclaré corresponde à l’UID effectif
réel. Cette égalité n’authentifie pas un humain. Un getter immuable de Request
ne donne aucune autorité. Le paquet ne quitte jamais le domaine opérateur.

L’artefact et son pin sont déterministes depuis ce paquet protégé : le pin de
sortie ne vient jamais du fichier exporté. La création privée NOREPLACE ne
réécrit rien ; une erreur conserve publication/fermeture/sortie connues. La
réconciliation explicite vérifie les mêmes octets attendus et leur acquittement
local, sans adoption ni nouvelle entropie. Seul l’artefact de service est écrit,
jamais clé métier ou preuve d’installation ; le passage de ses octets vers un
dossier appartenant à l’UID du service reste une remise protégée distincte.

Comparer génération immédiate et lecture/export déterministes : retenir ces
derniers pour fournir une reprise utilisable avant la future création. Une
sortie encore dans un dossier opérateur n’est pas une installation du service.
Aucun compte/UID réel, mandat, journal, humain, listener ou service créé.

Critères : route absente reproduite ; paquet/UID/permissions/pin/options refusés
après témoins et reprise ; export exact, clé métier/preuve absentes, destination
existante intacte, erreur après publication et réconciliation sans duplication ;
causes indépendantes/IO custom, docstrings et FR/EN/protocole fermé/recettes,
profil normal sans fixture puis build/restart/Version/ressources réellement
servies. Nouveau helper opérateur de test explicitement séparé : il peut écrire
les secrets synthétiques dans son seul coffre /tmp ; ne pas lui appliquer la
promesse des helpers de service qui n’écrivent jamais de clé métier.

Extraction par responsabilité : noyau `provisioning::operator_handoff`, adaptateur CLI de grammaire/sortie. Cinq tests FS du noyau, un test de traversée CLI, six parcours CLI publics et trois tests de schéma restent requis ; aucune réduction de recette native ni d’installation. Recette noyau indépendante des entrées moteur/web en mouvement, avec dépendances internes déclarées. Qualification encore ouverte.

## Résultat local vérifié — incrément55

Installation **v0.1.1-dev.200**, PID774327, ELF
`1928ed9ae11e150d6e58bb81fd677b61719c4dedeb730ba654d4514859f482e2` :614 entrées effectives,60 entrées PAEA identiques aux
sources, profil normal sans test-fixtures et canal sans feature. Build/restart
pris en charge ; sélection préparée par SHA200 après refus d’un hash pris sur
l’autre chemin binaire. Ni commit ni push.

- `core06` :5 tests FS/cleanup/nonce,3 schéma et Rustdoc stricte ;
  `cli07` :6 publics CLI,5 noyau,1 traversée CLI,3 schéma et Rustdoc stricte.
- `helper08` figé : fixture opérateur qui écrit les seuls secrets synthétiques
  dans son coffre de test ; `roles09` :6 substitutions de rôles refusées,
  chacune après témoin valide et avec reprise. Aucun helper exécuté par la garde.
- dev200 `delivery20/active-operator11` :6 parcours,154 commandes réelles ;
  `active-ui16` :56 contrôles FR/EN et garde de script en échec ;
  `active-peer17` :3 paires/6 commandes. Schémas et dialogues FR/EN rendus/examinés.
- dev199 :précontrôle5/84commandes,installation9/66,appel opérateur5/56,
  personnel7+3gardes. Preuves conservées sous leurs ELF199 ; sources des contrats
  PAEA et de la CLI inchangées entre199/200. Seuls activity_entry.rs et les notes
  diffèrent ; aucun agrégat de tous ces tests sous ELF200 n’est revendiqué.

Preuves :`evidence/20261027-operator55-jyxrf421/`. `active-ui16` original sur199
refuse les notes courantes ajoutées après build ; intégrité VERIFIED FAIL,
5 contrôles avant refus. Nouveau candidat200 et recette native fraîche passent.
`core05` refuse deux erreurs de test d’extraction avant exécution ; originaux,
source drift et ancienne panne étrangère E0252 des tentatives02…04 restent intacts.
Deux lots de cache de tests identifiés sont archivés/vérifiés avant retrait ;
aucun cache partagé, binaire de service, preuve historique ou donnée supprimé.

L’export ne régénère aucune clé/preuve ; le nom temporaire atomique utilise
toujours l’aléa OS. Rapport de phases et causes indépendantes conservés après
publication/lecture/close/flush ; pin dérivé de la source protégée, jamais appris
du fichier exporté. Aucun humain/compte/UID réel/mandat/service admis ou démarré.
Suite :création du paquet initial avec pin/intention durable avant publication,
reprise sous le pin conservé puis remise protégée au service. PA-02 partiel,
0/17 globaux, helpers UID/humain/admission/daemon/E2/E3/PA13/RETEX ouverts.
