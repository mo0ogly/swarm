# PAEA79 — fichiers privés possédés pour le transfert pack

Reprise14:39UTC, dernier78 et instance234/ELF883cdb18c7f1574c4b3f6207791960062874721fe7a8e5848f55cc6c09192dc9
observés ; HEAD86c5ab2f3, travaux77/78 conservés, aucun Cargo actif. Codex seul
sansGLM/délégation/commit/push nouveau ; échéance28oct19h20Paris.

Replan PA03 : avant de composer le job, éviter la perte des fichiers/dépendances
sur erreur. Comparer callback éphémère et propriétaire de fichiers explicitement
fermé. Choisir un espace privé détenu : mkdir strict sans adoption, fichiers
nouveaux NOFOLLOW/EXCL, flux parent conservés et close vérifié. Ce lot ferme la
précondition de stockage et la vérifie avec vrais flux pack Git sur disque ;
il ne prétend pas déjà gérer les processus non récoltés ou la route protégée.
La discussion de conception du job a été trop longue : retenir cette tranche
bornée plutôt qu'un propriétaire asynchrone incomplet. Prochaine étape demeure
le job possédant source/destination/fichiers/erreur, pas une clôture de PA03.

Critères : création exclusive/modes/alias/symlink/hardlink/erreur et reprise,
Git réel export/import par ces fichiers, objet exact et refs inchangées, fermeture
explicite. Aucun modèle payant, base utilisateur ou seuil d'exécution arbitraire.

## Résultat vérifié

Preuve : `docs/plans/actions-confiance/paea/evidence/20261028-pack-files79-9l4n39hv`. Core01 : **quatre tests unitaires nommés**
PASS/VERIFIED et Rustdoc strict, sources stables ; ELF unitaire figé séparément.
Création exclusive/mode0600, révision non réécrite, symlink/hardlink refusés avec
reprise valide, substitution d'inode du pack refusée/restauration acceptée.
Le cas Git réel utilise le runner de production sur deux bare isolés : PACK et
SHA256/taille vérifiés, objet commit exact présent après import, refs source
inchangées et destination vides. Renommer le staging et créer un homonyme vide
ne détourne pas le parent détenu ; fermeture explicite laisse les artefacts.

Les cinq fichiers originaux et l'entrée d'import restent possédés jusqu'à close,
qui tente toutes les fermetures en conservant les causes indépendantes. Une
création partielle peut laisser un répertoire/fichiers ; pas d'adoption ou de
troncature. Le contrôle d'inode n'est pas une immutabilité de contenu contre le
même UID. Le caller doit fournir une révision non vide déjà validée ; une écriture
vide ne consomme pas le fichier. Aucune injection ENOSPC/fsync/close nouvelle,
aucune garde d'enfant non récolté ou admission métier démontrée par ce composant.

Livraison normale **v0.1.1-dev.236**, ELF `087df5a966e2c4d346505b11c0e4c01aa96c49a2a50516249150b80c4cf363e4`,
PID221840, commit compilé 86c5ab2f3b60512eecb46729c47ac02924aaf65b, dirty déclaré.
Build/restart --prepared-build exact par launcher pris en charge ;
688 entrées normales, 487
communes au lot unitaire, zéro différence. Cœur sansfeatures/test=false,
sansfixtures, graphe normal conservé et trois différences transitives unitaires
justifiées séparément. Les tests sont cfg(test)=true, pas un SDK normal ni un
service protégé. Version04 : **56 assertions navigateur + garde d'échec script**
PASS/VERIFIED, assets/notes réellement compilés FR/EN, captures inspectées.
La construction concurrente antérieure n'a pas été adoptée après modification
des sources : les preuves restent attachées à leurs octets. Aucun commit/push,
GLM/délégation ou processus d'autrui interrompu.

Le job possédant source/destination/fichiers/cause de processus reste à composer.
PA02/03 partiels, 0/17 invariants de protection démontrés globalement ; UID,
humain, confinement, indépendance, E2/E3, PA13 et RETEX final restent ouverts.
