# PAEA — incrément 25 : originaux de laboratoire liés au rapport

[English](../../../en/plans/actions-confiance/paea/INCREMENT-25.md) · [Plan](../PAEA-APEX.md)

## Reprise, défaut et décision

Lot24 commité `079635b295b68c9422c1276f31f87488aa849c50`,14 fichiers,108 entrées
préparées étrangères préservées. Codex seul, sans GLM. Le programme PA-00…13
reste adopté ; IA3/PA-02/03 protégés, confinement, responsables humains et E3
gardent leurs préconditions. PA-09 peut progresser sur la preuve du laboratoire
sans inventer ces autorités. Aucun nouveau compte/service ni paiement autorisé.

Reproduction PAEA-EVIDENCE-25 : dans une copie de la campagne24, supprimer une
sortie d'oracle puis rappeler `export_report` publie encore les huit PASS.
`reproducer.json` conserve le fichier, son hash et la copie ; originaux24 intacts.
Cause : l'index d'appels n'imposait pas la présence/hash de chaque flux, et le
rapport ne liait pas l'arbre des observations. Les anciens PASS de recette restent
ceux de leurs sources ; ils ne prouvaient pas cette intégrité complémentaire.

Deux routes : faire évoluer le rapport embarqué, ou conserver le rapport v1 et
ajouter un inventaire explicite à vérifier avec ses originaux. Choix additif :
compatibilité du lecteur, périmètre clair et aucune nouvelle activation métier.
Les appels enregistrés v2 lient stdin/fichiers d'entrée, stdout/stderr ; les
workers lient document/sorties. L'export refuse un flux obligatoire manquant ou
altéré avant toute publication. L'inventaire contient les fichiers exacts des
deux sous-arbres, plus le hash du rapport. Publication atomique exclusive de
l'inventaire puis du rapport. Un échec IO peut laisser un dossier incomplet OU
les deux fichiers publiés. Diagnostic UNKNOWN de publication avec empreinte attendue ;
vérifier les originaux avant toute reprise. Aucun rollback des originaux.

## Parcours et critères

Après la commande existante du menu `run_pilot.py`, conserver séparément
`evidence_inventory_sha256` annoncé sur stdout. Le nouveau fichier est
`paea-lab-evidence.json`, schéma `laboratory-evidence-v1.schema.json`.

```sh
python3 tools/paea/verify_pilot.py --campaign /DOSSIER/CAMPAGNE --inventory-sha256 EMPREINTE_CONSERVEE
```

Cette commande ne rejoue aucune campagne et n'exécute jamais les argv du dossier.
Elle ouvre les originaux par descripteurs, refuse liens/fichiers spéciaux/traversée,
vérifie deux inventaires stables et la recette dans une copie temporaire privée
de ses octets capturés. La copie est retirée ; les originaux ne sont pas modifiés.
Des diagnostics à la racine hors rapport/inventaire/recipe/observations restent
hors portée. Les deux sous-arbres doivent correspondre exactement.

1. Vraie campagne huit tests, v2 des appels et schéma d'inventaire ; rapport v1
inchangé et lecteur historique compatible par son contrat de données.
2. Témoin avant chaque faute, suppression/altération/ajout, liens/FIFO/répertoire,
index/rapport/recette/manifest malformé et mauvaises empreintes refusés ; reprise.
3. Régression avant publication, erreur ENOSPC injectée aux deux phases et vraie
erreur stdout ; fautes avant publication sans nouveau rapport, faute après publication
avec UNKNOWN et vérification du dossier conservé. Originaux conservés.
4. Contre-exemple conservé : un auteur du même UID peut modifier les sorties,
réécrire tous les hashes et fournir un nouveau pin cohérent. L'ancien pin refuse ;
le nouveau n'authentifie aucune observation. Marqueurs simulé/non établi conservés.
5. Recette permanente et inventaires des appels dérivés ; revue liée aux octets
politiques/fonctions/erreurs, documentation FR/EN et sources/profil exercés.

L'inventaire ne prouve ni qu'aucun événement n'a été omis avant enregistrement,
ni l'auteur, ni la vérité de l'oracle, ni des clés protégées. Un pin issu du même
canal non fiable n'apporte pas d'authenticité. Les anciennes campagnes sans cet
inventaire restent des preuves historiques ; aucune signature ou migration
silencieuse n'est ajoutée. Aucun E2/E3 ou invariant protégé revendiqué.

## Vérification et livraison

Preuves neuves : `evidence/20261026-evidence25-3dw0sevr/`. Résultats à compléter après
exécution. Sources de CLI native inchangées : binaire normal dev109/profil du
lot23 épinglé, dix sources de protection à rapprocher avant réemploi. Le service
local courant peut évoluer concurremment, distinct du binaire de test. Les outils
Python sont consommés depuis le checkout par la commande déjà affichée ; aucun
asset ni source compilée modifié. Aucune nouvelle version embarquée/reconstruction
ou installation serveur n'est donc attribuée à cette tranche d'outillage.


## Résultats vérifiés et RETEX de la tranche

`recipe-02` : **deux étapes PASS, intégrité VERIFIED**. La commande publique lance
les **huit tests nommés** et exporte rapport/inventaire, puis contrôle mauvais hash,
sortie existante et recette corrompue sans écrasement. `campaign-02/recipe` est
également PASS/VERIFIED : quatre contentions24, oracles des deux domaines et
contre-exemple des aliases restent exercés. Les **onze tests du vérificateur**
passent, avec **68 appels CLI retenus**, contrôles avant faute et reprises.
Inventaire réel : **1020 fichiers d'observation**,
**10 fichiers de recette**, **197 appels v2**,
dont12workers et56copies d'entrées fichier. Aucune somme d'assertions n'est une
mesure d'essais humains indépendants. Le schéma v1 de l'inventaire est validé.

Le nouveau garde refuse l'export avant publication lorsqu'une sortie d'oracle est
supprimée ; restauration exacte puis export/vérification passent. Suppression,
ajout, altération, index/rapport/recette corrompus, mauvaise empreinte, doublon JSON,
BOM, version booléenne/flottante, chemin traversant, champ inconnu, autorité
inventée, liens symboliques, FIFO, répertoire substitué et /dev/full refusent.
Les originaux restent inchangés ; les fichiers secrets de fixture ne sont ni lus
par lien ni affichés. Les copies de recette sont temporaires privées, jamais exécutées.

Amélioration récursive PAEA-EVIDENCE-25-PUB : après le premier PASS à10tests,
la revue a reproduit une erreur de synchronisation APRÈS publication exclusive :
les deux fichiers existaient, malgré l'exception. `replan-post-publication.json`
conserve cette preuve. Contrat corrigé : `PublicationUnknown` conserve la cause
typée et l'empreinte calculée avant écriture ; le consommateur main émet UNKNOWN
de **publication du rapport**, prochaine opération de vérification, sortie2.
Les tests prépublication ENOSPC vérifient l'absence de nouveau rapport ; le11e test
rejoue uniquement des **octets de recette conservés**, publie réellement les deux
fichiers puis injecte l'erreur et observe UNKNOWN→dossier vérifiable. Ce replay
n'est pas une nouvelle campagne d'effets. Aucune durabilité après coupure électrique
n'est démontrée. `recipe-01` et les deux reproductions originales restent conservés.

La contre-épreuve de contention24 est requalifiée isolément sur l'enregistreur
modifié : `contention-guard-recipe-01` PASS/VERIFIED, manifeste dérivé du seul pas
maintenu `early-release-assertion-guard`, quatre échecs attendus après témoins,
processus avortés/recueillis. Ni répétition du reste de la recette24 ni nouveau
PASS de ses navigateurs. Les quatre inventaires dérivés incluent les nouveaux
modules ; leur contrat final/flux réel stdout-stderr est relu avant lancement.

Les dix sources de protection du dev109 correspondent à son inventaire/archive
effective et au binaire exact `fb1dcac07bd0794a0f0d45c8b1f379c685ac2a1953e830f4bf784e647e1adc6e`.
Profil normal sans fixture, features cœur vides, test=false réutilisés ; aucun
profil courant global déduit. Service observé séparément dev115/PID1379602,
commit079635b29, modifié=true, sans installation/redémarrage de cette tranche.
Le rapport v1 et les assets n'ont pas changé ; navigateur non rejoué. Aucun
compte, pin d'autorité ou contexte d'agent embarqué créé/actualisé implicitement.

## Revue, transmission et limites

Revue de même contexte liée aux19fichiers dans `source-review.json` : politiques,
fonctions et erreurs jusqu'au CLI. Préconditions de chemins/pin/fichiers, grammaire
SHA256, huit cas et formats v1/v2 sont des contrats de représentation/vérification.
Aucun nouveau plafond, délai, retry, réglage opérateur ou politique Admin. FDs
no-follow/nonblocking, fermeture finally et copie de recette privée protègent le
parcours de lecture contre les liens/fichiers spéciaux ; ce n'est pas du confinement.
L'inventaire porte sur les **fichiers réguliers**, pas les dossiers vides, droits,
attributs ou origine des données. Aucune résistance générale à un auteur du même
UID, exhaustivité d'événements avant enregistrement ou vérité d'oracle revendiquée.

Contre-exemple conservé : sortie forgée + index/rapport/inventaire réécrits + nouveau
pin passent comme octets cohérents, avec autorité toujours simulée. L'ancien pin
refuse. Ce test borne la propriété réellement vérifiée, sans prétendre détecter
une falsification dont l'attaquant fournit aussi la nouvelle racine.
La provenance indépendante du pin reste au propriétaire PA-09/autorité externe.

Apprentissage transmis aux documents opérationnels FR/EN et au README : lier chaque
flux original avant de sceller ; vérifier une erreur après la publication, pas
seulement avant, et exposer l'incertitude jusqu'au consommateur. Les consignes
persistantes n'actualisent ni les processus déjà lancés ni les contextes embarqués.
IA3/PA-02 (principal humain vérifié, service/bootstrap/pins indépendants), PA-03
protégé et AC-02 restent leurs dépendances concrètes ; exécution web des campagnes,
E2/E3 et RETEX final du programme ouverts. **0/17 invariants protégés démontrés**.
Aucun commit25/push ou acceptation indépendante. Les preuves historiques14/23/24
conservent leurs octets/portées ; elles ne sont pas rétroactivement scellées.

Vérification de cette campagne avec le pin conservé :

```sh
python3 tools/paea/verify_pilot.py --campaign /home/fpizzi/Documents/ChatGPT/swarm/docs/plans/actions-confiance/paea/evidence/20261026-evidence25-3dw0sevr/campaign-02 --inventory-sha256 d68ea39b6d9a9ec5c62cc9ddff7f1543e0128f56f1fd5ed264be57c6f40110fc
```
