# M25 — décision complète pour demandeur et destinataire

17 octobre 2026, reprise « go next ». Propriétaire : Codex mémoire, seul,
`replan` et APEX borné. M24 est réutilisé à sources vérifiées. Le chantier
identité a livré les paquets preuve et journal d'enrôlement, sans raccord public.
Ses fichiers restent en lecture seule ; les choix opérateur et l'autorité
avant plan ne sont ni supposés ni contournés.

## Décision

M24 produit les demandes exactes, mais aucun consommateur ne vérifie l'ensemble
ni les droits propres du destinataire. Deux voies : laisser chaque futur appelant
composer une boucle de décisions partielles, ou centraliser ce contrat mémoire.
Retenir un évaluateur privé complet, reconstruit sur les vrais producteurs M19
et le noyau identité existant, sans ajouter d'API, schéma persistant ou autorité.

Le demandeur doit satisfaire tous les contrôles M24. Le destinataire doit aussi
être le principal et l'instance liés à la proposition, avec un appelant frais
et les droits `memory.read` sur les seules sources réellement transmises.
Les contrôles utilisent un même instantané d'autorité et un même instant fourni
par le futur consommateur protégé. Aucune source opérateur/omise n'est transmise.
Une erreur renvoie une décision entièrement vide, sans sous-liste favorable.

Le résultat reste une décision pure : aucune permission créée, aucun texte
diffusé, aucun reçu consommé, aucun engagement ou effet atomique revendiqué.
Le contrôle protégé doit encore attester le destinataire et son affectation
réelle à la tâche, fournir la portée courante et garder les verrous jusqu'à
l'engagement. Ni les structures privées ni les fixtures ne prouvent cela.

## Critères

| ID | Observable |
| --- | --- |
| M25-01 | Tous les contrôles demandeur et lecture destinataire réussissent avant tout résultat complet ; événements des deux rôles distincts. |
| M25-02 | Un droit manquant, y compris le dernier contrôle, donne le refus exact et une décision vide, jamais un préfixe favorable. |
| M25-03 | Principal/instance/canal/portée/projet/temps incompatibles refusés ; aucun destinataire absent ne devient opérateur implicite. |
| M25-04 | Réemploi réévalue les deux acteurs ; contenu/destination/grant/autorité changés, décision éditée, incomplète ou réordonnée refusés. |
| M25-05 | Expiration/révocation et appelants périmés sont pris en compte avant verdict ; décision antérieure et entrées restent inchangées. |
| M25-06 | Tests ciblés nommés, statique, compilation, empreintes et docs FR/EN/RETEX ; aucune conformité publique inférée. |

Plan : tests de contrat avant implémentation, évaluateur privé dans ses nouveaux
fichiers, vérifications ciblées et revue locale. Gardes M24 conservées puisque
le nouvel évaluateur les consomme. Sources testées gelées dans notre périmètre ;
replan si les entrées réellement compilées changent. Aucun fichier identité édité.

## Délais de vérification

L'ancien `-timeout 90s` de M24 est une borne héritée sans contrat démontré pour
ce lot. Ne pas la reprendre. `go test -timeout=0` désactive explicitement le
watchdog global ; compilation et sortie structurée sont suivies, avec diagnostic
si absence de progression. Les délais fonctionnels/produit et refus d'autorité
restent inchangés. Les preuves M24 ne sont pas réécrites. Aucune augmentation
de politique, budget, retry ou appel fournisseur.

## Replan M25-I01 — ressource d'instantané confondue entre tâches

Origine : lecture du tuple de ressource M24 pendant la préparation du contrôle
destinataire. `current-task-snapshot` est un ID fixe ; son tuple v1 contient le
travail mais pas la tâche. Deux instantanés de tâches du même travail obtiennent
donc la même ressource, même si le digest complet de proposition change. Un
octroi de lecture sur cette ressource ne distingue pas ces deux sources.

Propriétaire : mémoire. Valeur : cloisonnement de la source au niveau du droit,
sans confondre ce droit et la fraîcheur d'une décision. Reproduire d'abord avec
deux vraies reconstructions M19 : ressources snapshot différentes attendues,
ressources des dialogues originaux inchangées attendues.

Comparer une règle spéciale sur l'ID courant à une ressource décrivant partout
la tâche source réelle. Retenir le contrat général : tâche du snapshot pour
l'instantané, tâche de `DialogueRef` pour les questions/réponses. Protocole de
ressource v2 et proposition v2 explicites, sans accepter v1 par tolérance. Les
preuves M24 restent historiques ; aucune permission persistée M24 n'est migrée
ou créée puisque ces chemins privés ne sont raccordés à aucun service public.
Mettre à jour le garde M24 et le contrat documentaire dans le même incrément.

## Replan M25-I02 — fixture de changement d'octroi

La première série passe, mais la revue détecte que sa variante « new-grant »
réécrit `Instance.GrantID`, interdit par `validateAgentIdentityTransition`.
Ce résultat pur ne prouve pas une transition applicable. Conserver la fixture
originale et corriger le cas : nouvel octroi au demandeur sans instance, ancien
octroi et instance destinataire intacts, révision suivante et validation réelle
de transition avant évaluation. Rejouer ce garde avec les contrôles concernés.
La correction ne modifie aucun code d'identité ou politique de publication.
