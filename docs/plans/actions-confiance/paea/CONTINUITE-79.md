# Reprise après79 — fichiers privés vérifiés, job restant

Codex direct, sans GLM/délégation/commit/push supplémentaire. Vérifier heure réelle,
dernier lot, sources/concurrence/binaire actif. Ne pas engager une nouvelle reprise
programmée après28oct2026 19h20Paris. Preuves brutes conservées localement selon
la décision utilisateur, publications2bec348/9ebb8e3 terminées ; pas de nouveau
commit automatique. Préserver les sessions, processus et données d'autrui.

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

## Prochaine action PA03

Composer Source77/78 + dépôt privé72 + GitPackFiles79 + runner75/76 dans un job
créé AVANT le premier processus. Consommer une seule fois l'entrée non vide/OID
validée ; garder les propriétaires et toute cause contenant Child tant que
non récolté. Aucun Err détruisant ses dépendances, aucun booléen préchecked,
aucun Vec du pack. Contrôler absence d'alias source/destination, format/objet exact,
source actuelle avant/après, refs inchangées pendant préparation et effets partiels.
Le close de fichiers ne prouve pas reap ; Clone/Stdio ne fournit pas un reçu de
close vérifié du handle enfant. Ne pas inventer confinement/séparation d'UID.

Ensuite admission65/66, vraie réservation settings/Trust/plan/mandat/writer,
Source.inspect_for_effect78, intention durable70 avant engagement69, OLD direct
exact et réconciliation terminale. Existing/Uncertain n'autorise pas retry aveugle.
Prérequis administrateur/humain et approbation indépendante déjà pendants : ne
pas redemander ou contourner. Les fixtures historiques restent immuables ;78 a
configuration révision2 après apply public légitime, pas un event neuf à rejouer.

Replan de méthode : la conception du job s'est prolongée sans code. Pour reprendre,
choisir une seule frontière vérifiable et l'implémenter ; conserver les causes
et limites réelles, sans réexpliquer indéfiniment l'architecture ni déclarer PA03
terminé sur un composant. Les sources pérennes et la recette79 transmettent les
contrats et les cas nommés ; la documentation seule ne prouve pas l'effet futur.

Cible Cargo .cache/paea-verification-target propre à ces vérifications ; ne pas
nettoyer le cache normal partagé. Éditer/formatter sous verrou build nonbloquant,
libérer avant launcher. Pas de changement des inputs en compilation. Figer ELF
unitaire avant son remplacement Cargo ; profil normal sansfixtures et assets
protégés/Version FR/EN vérifiés séparément. Lire les recettes finales avant lancer,
pas de timeout global arbitraire ni de répétition sans précondition nouvelle.
