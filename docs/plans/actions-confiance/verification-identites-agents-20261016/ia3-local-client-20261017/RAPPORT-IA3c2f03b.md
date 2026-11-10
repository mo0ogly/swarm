# IA3c.2f-03b — client privé, identité du retour et reprise

17 octobre 2026. Codex seul dans le checkout canonique ; branche
codex/clear-product-method-names, HEAD 4e58f7cbf8200a58f69f1f4d03d657c3313fb642,
travaux locaux préservés. **Incrément vérifié en fixtures ; aucun service public,
compte système, fournisseur ou mission modifié/activé.** Aucun commit, push,
déploiement ou appel de modèle. Auto-inspection, sans revue indépendante.

## Résultat et contrats

Le client vérifie chaque réponse contre un pidfd serveur détenu indépendamment
par le lanceur et l’UID attendu ; il ne croit ni le créateur de la connexion,
ni un PID déclaré dans le JSON. Le constructeur duplique le pidfd en CLOEXEC,
contrôle pidfs et utilise le couple périphérique/inode sous Linux 64 bits.
Fichier ordinaire, ancien pidfd anonyme ou architecture 32 bits sont refusés sans
repli numérique. Le [code primaire Linux v6.17](https://raw.githubusercontent.com/torvalds/linux/v6.17/fs/pidfs.c)
explicite la comparaison d’objets processus ; kernel-contract-source.json conserve
la référence et ses limites. L’expérience tourne sur Linux 6.17.0-41-generic,
Go 1.24.3 linux/amd64. Ancien noyau et 32 bits : refus dans le source, pas de
machine correspondante testée. Pas d’identité persistante entre démarrages ni
de preuve de l’exécutable ou de l’humain déduite du pidfd.

Transport **version 2** : demande schema_version/challenge/pins/operation/change ;
réponse schema_version/request_digest/operation/view/projection/error.
Challenge aléatoire 32 octets par nouvel échange ; empreinte SHA-256 au domaine
swarm-installation-control-v2 sur les octets canoniques de la demande complète.
Le service compare les pins publics avant l’autorité indépendante ; l’égalité
ne confère pas de droit. Version 1 refusée explicitement, sans downgrade.
Configuration persistée, journal et empreintes restent en version 1 : aucune
migration ou réinitialisation de l’histoire pour cette évolution de transport.

ValidateView/ValidateProjection dans enrollmentconfig partagent le calcul de
tête et la validation d’historique du stockage. Après attribution du processus,
le client vérifie format exact, version, opération, demande, pins, dernier Change,
base/replay et type de résultat. Erreur avec données, résultat absent ou mixte,
code inconnu, tête incohérente et paquet tronqué ne produisent aucun résultat.
Les refus métier authentifiés restent des refus ; unavailable indique seulement
l’indisponibilité du service, sans inventer une cause disque.

Client et canal serveur possèdent chacun un échange unique puis ferment leurs
ressources. Le pidfd original du lanceur reste ouvert ; seul le duplicata client
est fermé. La réutilisation de chaque côté est refusée. Annulation client :
fermeture de sa connexion pour interrompre envoi/réception, sans délai fixe.
Avant envoi annulé : pas d’envoi. Après tentative de mutation sans réponse
attribuée/valide : zéro résultat et ErrIndeterminate, pas de conclusion d’absence
d’effet. Retry métier même EventID/base sur un nouveau canal, challenge frais et
nouvelle autorisation, sans répétition de transport automatique.

Le canal doit venir d’un lanceur de confiance et être remis au serveur attendu
sans fuite de descripteur. Cette bibliothèque vérifie le retour après envoi ;
elle ne démontre pas le destinataire avant envoi ou la confidentialité. Comparer
remise privée vérifiée et établissement authentifié avant contenu : le raccord
local prévoit la première, dont la preuve de déploiement reste obligatoire.
Un simple chemin de socket ne satisfait pas cette précondition ; l’adaptateur
ne fournit pas une authentification mutuelle publique. Aucun secret/session
humaine n’est transmis dans ces fixtures. EP-07 reste ouvert.

## Sources et documentation

Deux ajouts : agent_identity_config_client_linux.go et son test dans internal/engine.
Quatre fichiers existants modifiés : les deux fichiers du service de configuration,
puis config.go/config_test.go dans enrollmentconfig. Les 34 autres entrées de la
référence de 38 sources/modules sont inchangées. Aucun fichier mémoire, lanceur,
méthode ou processus en cours édité. source-scope.json et source.diff détaillent
ce périmètre ; les tests d’avant correction restent archivés.

Contrats FR/EN et schémas, installation FR/EN, cible IA3c.2 et note scientifique
actualisés au même incrément. L’ancienne description opérationnelle transport 1
est corrigée de façon ciblée ; ses preuves historiques demeurent dans 03a.
Les schémas FR/EN sont rendus et inspectés : [FR](diagram-fr.png), [EN](diagram-en.png).
Ce contrôle de schéma ne constitue pas une validation UX ou un service actif.
Les processus déjà démarrés n’ont pas reçu les nouveaux sources.

## Vérifications et portée

| Contrat | Résultat | Preuve / limite |
| --- | --- | --- |
| Client, service et canal IA2 | 77 noms PASS normal/race-02, zéro FAIL/SKIP | Inclut les gardes 03a adaptées à v2 ; pas 77 attaques indépendantes |
| Configuration et témoin | 106 noms PASS normal/race-01 | Inclut les 101 gardes précédentes et la nouvelle validation View/Projection ; douze entrées propres inchangées après la réparation moteur |
| Deux défauts de candidat | Deux tests FAIL avant, PASS dans le lot final | closure-red-01 et before-closure-fix.tar.gz |
| Serveur exact et autre processus même UID | Enfants réels, pidfd indépendant, acceptation/refus du résultat | Même compte de fixture ; aucun confinement réel prouvé |
| Demande et réponse exactes | Version 1/inconnue, challenge invalide/ancien, autre demande/opération/Change/pins, type mixte, erreur avec résultat et head incohérent refusés | Corps canoniques, sans acceptation d’un préfixe |
| Propriété et reprise | Annulation avant/après émission, canal consommé, original conservé, duplicata fermé/CLOEXEC, challenge renouvelé au retry | Pas d’effet nul déduit après perte d’acquittement |
| Taille de réponse réelle | SO_SNDBUF observé 8192 octets ; raison 16384 octets ; vrai EMSGSIZE après écriture, retry conserve révision 1 et octets | Ce réglage réduit uniquement la fixture ; aucune politique produit |
| Go et compilation | vet-02/build-02 PASS, 832 entrées effectives stables | Binaire de fixture dans /tmp, hash enregistré, jamais déployé |
| Contrat des méthodes | Contrôleur réel PASS, cinq tests/sous-tests | Même overlay/832 entrées ; aucune source de méthode modifiée |
| Documentation | Contrôles dans checks-final.json | Pas d’acceptation indépendante ou de parcours Admin |

Compter les noms de tests/sous-tests, pas cumuler normal/race, lots 01/02 ou
processus helpers comme des attaques distinctes. Le helper de sous-processus ne
travaille que lorsqu’il reçoit l’environnement de fixture ; les scénarios parents
vérifient le résultat réel du sous-processus. Le paquet config est rejoué car son
contrat commun change, puis ses résultats sont réutilisés après une réparation
moteur seule, avec hashes pertinents vérifiés. Aucun go test ./... répété.

Les watchdogs globaux Go et enfant sont désactivés (-timeout=0/-test.timeout=0).
Le canal IA2 antérieur conserve ses deadlines de socket de fixture préexistantes
(5 secondes), sans modification ou expiration dans ce lot. Le poll(0) du cas de
fermeture est une observation non bloquante après retour de serve, pas un délai
arbitraire de vérification. Pas de nouveau TTL, budget ou politique de tentatives.

## Deux défauts reproduits et corrigés

Premier candidat : 69 noms moteur et 106 config passent normal/race, vet/build.
La revue du test de taille constate qu’il ferme le socket après serve. Une garde
séparée provoque réellement EMSGSIZE puis observe POLLHUP : elle échoue car le
serveur a laissé le canal ouvert. Aucune attente de timeout nécessaire pour la
preuve. Risque : le client attend sans réponse après une erreur déjà retournée
au côté serveur. Correction générale : canal serveur à usage unique, fermeture
par defer sur toute issue ; le test de taille ne ferme plus à sa place.

Deuxième garde du même lot rouge : unavailable était traduit en ErrStorage.
Le test échoue sur cette cause inventée ; correction par erreur de service générale.
Les deux FAIL sont liés au candidat et non à un service public exploité. Le source
avant correction, la sortie originale, les noms, dates et hashes sont conservés.
Les deux cas sont rejoués dans les 77 noms normaux et race après correction.

## Capitalisation et reprise

| ID | Origine et valeur | État, propriétaire et condition restante |
| --- | --- | --- |
| I2f03b-01 | Test qui ferme à la place de l’adaptateur ; éviter attente et faux sentiment de réparation | Reproduit puis corrigé/vérifié ; Codex ; contrôler la propriété de fin d’échange au raccord public |
| I2f03b-02 | Unavailable ne précise pas la cause ; éviter une mauvaise récupération disque | Reproduit puis corrigé/vérifié ; Codex ; conserver causes exactes côté service et codes limités côté client |
| I2f03b-03 | Retour d’un autre processus/d’une autre demande ; séparer origine noyau et contenu | Client, challenge, v2 et validations partagées vérifiés en fixtures ; Codex ; bootstrap réel et autorité humaine ouverts |
| I2f03b-04 / I2f03a-03 | Limite technique d’envoi réelle et issue après effet | EMSGSIZE et reprise sans doublon maintenant vérifiés ; Codex ; transfert d’historique arbitraire à concevoir avant parcours public, sans plafond métier caché |
| I2f03b-05 | Pins et pidfd indépendants ne prouvent pas à eux seuls une remise privée ou confidentialité | Précondition conservée, pas implémentation publique ; Codex/opérateur ; prouver remise du canal, absence de fuite et compte/confinement ou établir le serveur avant contenu sensible |

Le script de préparation de preuve a échoué SyntaxError (parenthèse excédentaire),
puis le lancement dépendant a échoué faute de fichier ; aucun test exécuté ou
PASS inventé. Correction de méthode : compiler le script avant sauvegarde et
vérifier code de sortie/fichiers avant l’appel dépendant. Ce diagnostic est dans
le plan canonique ; ce n’est pas une panne du client Go. Les URL primaires
kernel.googlesource/freedesktop inaccessibles sont remplacées par le miroir
primaire Linux versionné ; aucune conclusion technique tirée d’un accès échoué.

Commandes et sorties originales : *-run.json, *.jsonl, *.stderr et workflow-check-02.*.
Les entrées effectives sont liées par source-final.json, final-sources.tar.gz et
l’overlay de deux fichiers mémoire explicites ; rien n’est modifié dans ce chantier.
L’overlay est comparé au checkout courant dans checks-final.json. Toute évolution
ultérieure du paquet doit refaire cette comparaison avant réutilisation. La recette
run_checks.py protège les noms d’essai existants et refuse les entrées instables ;
un nouveau suffixe ne dispense ni de diagnostic ni de justification du nouveau lot.

Le prochain raccord demeure le service/lanceur local protégé avec source humaine
indépendante, puis setup/API/CLI/Admin. Les fixtures ne désignent pas l’humain initial
(issuer/subject/mapping), n’installent pas le compte et ne prouvent ni protection
contre un fournisseur réel, ni révocation finale/descendants, ni identité de tous
les nœuds d’un workflow. EP-01/04/05/07, F01–F07, IDA, IA3–IA5 et RETEX restent
ouverts/partiels. Restaurer config et témoin ensemble reste une limite connue ;
distant prévu sans repli permissif. Aucun résultat de nouveauté scientifique ou
de supériorité comparative n’est déduit de cette expérience locale.
