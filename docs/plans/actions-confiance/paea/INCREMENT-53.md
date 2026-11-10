# PAEA — incrément 53 : représentation des UID à la définition

Reprise52 livrée/vérifiée dev185,41Rust+3schéma/2fixtures/5opérateur/3gates/56UI+garde. PA-02 et0/17globaux restent ouverts. Codex seul, sans GLM/délégation/commit/push.

Le suivi52 a relevé une hypothèse de cohérence : le canal refuse le sentinel u32MAX mais bootstrap/provisioning n’en vérifient que la distinction service/contrôleur. Source Rust et Go confirment l’absence de ce contrôle. Le [contrat Linux primaire](https://github.com/torvalds/linux/blob/v6.17/include/linux/uidgid.h) et l’en-tête6.17 local définissent uid_valid par exclusion de UID-1. Cela ne signifie ni compte existant ni autorité ; UID0 reste représentable.

Plan borné : témoins de données valides0/1 et MAX-1/0, puis tests nommés des deux rôles réservés en Go/Rust et provisioning avant entropie. Conserver chaque refus actuel en échec avant correction. Si confirmé, prédicat de représentation commun à chaque langage, utilisé aux deux frontières canoniques et avant génération de matière ; pas de plafond opérateur ou de ban root. Schémas UID à inventorier avant modification, aucun schéma détaché inventé.

Revoir les fixtures qui utilisent !euid : sous euid0 elles produisent la sentinelle. Corriger la génération des seules données de fixture, sans exception de validation en production. Préserver les anciens drivers/helpers figés et leurs preuves ; si une source maintenue change, reconstruire un nouveau driver lié aux octets et adapter la recette courante, jamais réécrire son ancien SHA. Ne pas prétendre qu’un test de données UID0 constitue une exécution OS root ni deux comptes réels.

Comparer l’option refus local tardif et celle de validation cohérente à l’entrée : retenir la seconde si le reproducer confirme, pour éviter matière/configuration inexploitable. Aucun état privé existant réécrit : les déclarations invalides doivent refuser avant ouverture/effet, avec origine conservée et reprise explicite par une configuration valide. Pas de migration, réinitialisation ou nouvelle identité automatiquement adoptée.

Vérification : tests ciblés de représentation/encode/décode/refus sans entropie et reprise, consommateurs des fixtures modifiées, profil livré sans fixture, nouvelles preuves natives/installation locale/Version FR-EN ; docs et fonctions dans le même lot. Contrôler les erreurs et valeurs littérales contre leurs octets. Puis retour au raccordement admission→dispatch→réponse ; helpers UID, humain, E2/E3/PA13 et0/17 restent ouverts. Les177liens historiques indisponibles gardent leur suivi51.

## Résultats intermédiaires et reprise

Les originaux `go-red-01`, `bootstrap-red-02` et `entropy-red-03` échouent
avec sources stables : les deux rôles invalides étaient acceptés. Les correctifs
communs passent dans `contract-07`, intégrité VERIFIED : 29 tests Rust et
8 tests Go de premier niveau, avec sous-cas nommés, plus Rustdoc privée stricte.
La validation de préparation refuse avant l’appel d’entropie. Aucun schéma UID
existant trouvé dans le périmètre inspecté ; aucun schéma déconnecté ajouté.

Les sources de 13 fixtures Rust et du helper Go ont été adaptées au cas UID0 ;
les binaires historiques 37/49 restent intacts. Un nouveau wrapper natif vérifie
le SHA du driver et celui de sa source maintenue avant tout lancement, puis
exige les sept tests personnels nommés. Le parcours d’installation maintenu
contient désormais neuf cas, dont refus des deux UID réservées après témoin
valide, absence de cible/stage et restauration suivie d’installation. Son
exécution et la nouvelle livraison restent à vérifier, sans présumer du résultat.

Les compilations driver08/09 ont abouti mais leur inventaire a changé pendant
la compilation : fichiers étrangers pour08, notes de version seules pour09.
Ces preuves restent FAIL et ne qualifient aucun driver. Reprise10 après inventaire
courant, avec compilation du nouveau helper séparé ; aucun input exclu pour
masquer la dérive. Une première recette native utilisait des chemins absolus
interdits par le lanceur : refus avant exécution, corrigé en chemins relatifs
dans une nouvelle preuve12. La concurrence du checkout et les délais de notre
propre diagnostic restent des limites ouvertes, sans déclaration de réparation
par simple consigne. Puis reprise admission→dispatch→réponse.

## Vérification de l’incrément 53, sans clôture globale

Livraison locale **v0.1.1-dev.189**, PID259071, ELF
`8648d95a65cf524c7ee6cb315152a69c911b8fb9cabc3db1694f0f7bbf7e6703`.
Build et restart pris en charge ont réutilisé le candidat189 qualifié :
595 entrées effectives archivées et vérifiées, 57 entrées PAEA de production
égales aux sources, canal sans feature et sans fixture. Le préflight187 refusé
est conservé ; il n’avait exécuté aucun parcours. Ni commit ni push.

Preuves dans `evidence/20261027-uid-contract53-5xundpsl/` :

| Contrat | Preuve réelle | Limite |
| --- | --- | --- |
| UID Go/Rust, bootstrap canonique, avant entropie | contract07 : 29Rust, 8Go parents avec sous-cas, Rustdoc stricte | Données0/MAX-1, pas comptes réels |
| Noyau personnel et startup | core14 : 32Rust +3schéma | Fixture isolée |
| Entrées CLI, publication, cadre service | affected-cli16 : 26Rust | Erreurs réelles/fixtures de panne séparées |
| Autres fixtures CLI affectées | other-fixtures22 : 14Rust | Aucune suite globale déduite |
| Installation normale | active-install17 : 9cas/66commandes | Deux rôles réservés refusés avant cible, données restaurées et installation vérifiée |
| Opérateur normal | active-call18 : 5cas/56commandes | Pas daemon/canal |
| Personnel, driver actuel | active-personal24 : 7tests +3refus de liaison | ELF driver séparé, aucune identité humaine prouvée |
| Gates pair Linux | active-peer21 : 3paires/6commandes | Credentials connexion, pas admission complète |
| Interface réellement servie | active-ui20 : 56contrôles +garde script | Version, notes actuelles FR/EN, lien Git réel, état modifié, assets protégés/catalogue |

Les résultats sont stables et leur intégrité VERIFIED. 101exécutions Rust
ciblées au total entre quatre recettes, pas un pourcentage de conformité.
Les helpers53/driver53 nouveaux sont figés avec métadonnées de source ; les
originaux37/49 restent intacts. Les trois gardes refusent normal ELF, driver
ou source mal liés avant création du dossier de sortie/lancement. La recette
maintenue complète à deux étapes est rejouée24 après son évolution.

`git diff --check` PAEA passe. Le contrôle global retourne2 pour deux lignes
blanches étrangères (DELIVERY-PREPARATIONS-20261025.md et managed_receipt.rs),
conservées sans modification d’autrui. Ce contrôle global reste ouvert, de même
que les177liens historiques absents suivis51. Les limites de transmission et de
diagnostic lent de notre méthode restent ouvertes ; les consignes seules ne
les réparent pas. Une observation des fermetures Go du registre existant est
conservée dans fixture-followups.json pour son propriétaire/sa reprise.

Aucune migration privée automatique, Root UID réel, séparation physique,
admission, listener, daemon, humain, E2/E3 ou PA13 n’est qualifié. PA-02 reste
partiel et les17invariants globaux restent0/17. Reprise suivante : exposer de
manière protégée la préparation initiale encore réservée à la bibliothèque/
aux fixtures, puis reprendre le canal ; helpers UID et décisions humaines
restent des dépendances explicites.
