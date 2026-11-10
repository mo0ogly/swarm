# PAEA — incrément62 : permission du propriétaire de namespace

Reprise61 : bibliothèque27tests PASS, livraison attend sourcespartagéesstables ;
209/60dernièrelivraisonqualifiée. Investigationindépendante sansmodifierproduction.

SourceLinux6.17 : commoncap cap_capable_helper accorde capacités dans le namespace
enfant à son propriétaire résidant dans le parent ; ptrace vérifie ces capacités
dans le userns cible ; pidfs appelle READ_FSCREDS. Hypothèseàqualifier : le service
créateur/propriétaire peut inspecter un contrôleur dans son userns enfant sans
CAP_SYS_PTRACE globale, malgré dumpable0. Cela ne prouve pas deuxUID effectives.

Testdiscriminant : mêmeUIDexterne etCapEffparent0 dansdeuxprofils. Enfantnormal
dumpable1 témoin,0refusEACCES,1reprise ; enfantdansuserns/mountprivés crééavec
unshare, mêmes3phases,0devraitresterauthoriséauparentpropriétaire. Pidfd atomique
et sujetpaquetnoyauheld, objetsnamespacecomparés, closesetwaitexplicites mêmeaprès
assertion. Aucun compte/helper/sysctl/capacitéhôte modifié, aucuneadmissionmétier.

Constantsprctl3/4 UAPI installé, paspolitiques. Recettepermanente/testsourcepropre,
profilGNU/LINUXnommérefusezérotest. PasnouvelleAPI ni versionproduit : fichiers
uniquementfixtures/tests/recette/doc ;61resteàlivrer. Helpersuidmap/UIDdistinctes/
révocationmapping/FS/descendants/confinement/humain/E2/E3/0sur17 restentouverts.

Sourcesprimaires : Linuxv6.17 security/commoncap.c, kernel/ptrace.c, fs/pidfs.c,
https://man7.org/linux/man-pages/man7/user_namespaces.7.html. Accèsgooglesource
au tagv6.17refuséparoutilweb ; miroirofficieltorvalds/linuxv6.17accessible, pas
relanceinchangée. Sourceamont≠buildexact6.17.0-41Ubuntu ; témoinlocalrequis.

## Résultat local vérifié

Deux tests Rust nommés passent depuis le binaire de test figé : un test couvre
six phases de permission dans deux profils ; l'autre garde EINVAL après deux
contrôles valides, en comparant le puits nul et la capture privée. owner09 est le
parcours courant ; owner01–08 restent historiques, avec leurs sources exactes.
Le jeu effectif du parent reste nul et les UID externes restent égales. Aucun
compte, helper, sysctl, capacité hôte, mission ou datastore vivant n'est modifié.

La permission du parent dans son namespace enfant est donc une piste qualifiée
localement pour le lanceur. La permission avec deux UID réellement différentes
reste à vérifier après installation des helpers ; aucune baisse des gates58–61,
adoption du premier appelant ou autorité métier n'en découle.

La revue a corrigé une perte possible de diagnostic de fixture : stderr de
l'enfant n'est plus jeté ; les refus de prctl/unshare restent dans la capture
privée. Le contre-exemple volontaire avec puits nul et la récupération avec
capture observent le même EINVAL après témoin. Les autres fixtures anciennes
utilisant un puits nul gardent une suite diagnostique ouverte, sans réécrire leurs
preuves. Les refus de lancement avec enfant connu sont explicitement récoltés
et fermés ; les assertions de protocole sont suivies de fermeture des extrémités,
attente et fermeture du pidfd avant remontée de l'échec.

Le collecteur retient l'artefact de bibliothèque normal (features vides,
profile.test=false) et un exécutable explicitement de test. Compilation originale,
stdout/stderr, binaire figé et métadonnées sont conservés dans frozen07. Leur
résultat SHA256 est ancré dans stdout de owner09 ; le vérificateur local contrôle
six fichiers, refuse les symlinks et noms hors inventaire. Cinq témoins intacts
et trois refus (octets, lien de métadonnées, lien de pièce) passent ; l'original reste intact. Les neuf
intégrités de recettes passent. Ce sont des empreintes locales, pas une signature,
une provenance indépendante ou une qualification de tout le graphe compilateur.

Les trois sources Linux v6.17 de référence sont retenus avec URL, SHA et notices
SPDX dans upstream-v6.17 ; aucun code distant exécuté ou intégré au produit.
Le source Ubuntu exact n'est pas établi, mais le noyau réellement exercé est
6.17.0-41-generic. Source-review et archive lient politiques, fonctions, erreurs
et documentation aux fichiers réels. Aucun Rustdoc de bibliothèque requis :
aucune API de production modifiée ; les contrats des helpers de test sont
commentés et compilés/exécutés dans le parcours réel.

Pas de nouvelle version ni installation62 : uniquement tests, fixtures, recette
et documentation, exclus du parcours normal observé. La bibliothèque61 reste
à livrer sur un candidat stable ;209/60 reste la dernière livraison qualifiée
par cette tâche. PA02 partiel,0sur17/E2/E3/PA13/RETEX restent ouverts.

Le refus des métadonnées en lien symbolique est reproduit avant correction, puis
refusé après correction avec original toujours valide. La garde permanente couvre
aussi les octets modifiés et un lien de pièce : cinq témoins et trois refus, sans
mutation des originaux. Voir replan-metadata-kind.json et frozen07/verification-copies.

Après cette qualification, la livraison61 est vérifiée dansv0.1.1-dev.211,
ELF `20444629f554417d220d882f09113ff76cf7e48a17bc40b4285a878fcc2825f7`. Les12fichiers de62 sont exclus
du graphe compilateur normal qualifié211, selon le supplément retenu.
Les mentions de61 en attente ci-dessus sont historiques.

La recette permanente ne fixe plus un ancien dossier : le collecteur crée un
enfant privé neuf du parent explicitement configuré. Le manifeste portable a
été lancé deux fois sans modification, avec deux dossiers distincts (owner08/09),
tous contrôles passant. Le collecteur enchaîne ses gardes d'intégrité avant le
reçu final. Le chemin réel et son ancre sont dans portable-replays.json et stdout
de owner09 ; les anciens frozen07 restent historiques. Aucun placeholder de
sortie non pris en charge par recipe.py n'est supposé.
