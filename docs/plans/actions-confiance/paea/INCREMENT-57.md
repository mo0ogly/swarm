# PAEA — incrément57 : création initiale sous intention durable

Reprise56 installée/vérifiée dev201,627entrées effectives/62PAEA, preuves56
conservées. Codex seul, sans GLM/agent/commit/push. PA-02 partiel,0/17globaux.

Besoin : pouvoir créer le paquet opérateur dormant depuis une demande privée
épinglée puis le reprendre après publication incertaine, sans régénérer des clés.
La clé métier/proof restent dans le domaine contrôleur ; aucun compte, service,
Root d’installation, autorité courante ou humain créés/admis par cette tranche.

Choix comparés : séquence de précontrôles sans réservation (race entre absence
et génération), versus réservations immédiates Documents retenues pour les deux
stockages. Retenir les réservations existantes : elles sont nonbloquantes,
partagent les mêmes identités FD/verrous, refusent alias physique/busy sans
nouvelle politique de concurrence ni délai. Aucune nouvelle API d’identité de
dossier nécessaire pour cette exclusion. Le point d’ancrage et son intention
sont dans un dossier privé contrôleur ; le paquet est dans son autre dossier.

Une ancre de pin seule ne bloque pas une nouvelle génération si le processus
meurt avant l’écriture de l’ancre. Donc intention immuable AVANT génération,
liée au pin exact des octets de demande. Toute intention/ancre/paquet déjà
présent refuse create avant génération, même vide ou égal. Après intention
Durable : préparer une fois, Seal, ancre exacte Durable puis publication du
paquet sous les DEUX réservations retenues ; libérer/fermer ensuite chaque garde.
Aucune valeur utilisable ne sort après refus tardif. La lecture d’une ancre en
réconciliation reste fermée avant utilisation de son pin ; son écrivain local
créant un record connu n’a pas cette même précondition avant conservation du Seal.
Aucune boucle automatique. La réconciliation ne génère rien, vérifie intention,
ancre, paquet sous pin indépendant et correspondance de la demande canonique.
Une intention orpheline reste refusée explicitement, sans reset ou régénération ;
la remédiation opérateur conservant l’historique reste un critère ouvert si besoin.

Correction de reprise avant implémentation : decode_request impose déjà
encode_request(r)==raw (provisioning.rs174…181). Le JSON non canonique, même
sous son bon pin, est refusé. L’hypothèse précédente contraire est conservée en
preuve/replan, pas appliquée. Le pin lie ces octets canoniques exacts : SHA avant
décodage, puis comparaison du contenu canonique du paquet. La CLI peut réencoder
la Request du lecteur privé existant après sa fermeture, car l’égalité exacte y
est déjà requise ; aucun lecteur bis ou Frame privé supplémentaire nécessaire.

API noyau prévue : initial_creation::create(raw_request,request_pin,operator_dir,
anchor_file,ctx) et reconcile avec mêmes entrées. Métadonnées/Progress séparées :
intent/anchor/package publication de cet appel, lectures et acquittements, tentative
et fin de génération, correspondance de demande. Aucun seed/proof dans résultat.
Helper privé repository/checkpoint sur réservation empruntée pour partager la
publication et lecture réelles sans réentrer un verrou ou dupliquer le protocole.

Séquence : tests publics d’abord ; extraction interne couverte par tests existants ;
noyau/create/reconcile/refus/causes ; raccordement CLI + FR/EN/schémas/recettes ;
profil normal et livraison prise en charge, native et UI ; revue explicite des
politiques/fonctions/erreurs liée aux octets. Propriétaire Codex. Aucun critère
planifié n’est une exécution ou acceptation ; fin E2/E3/PA13/RETEX reste ouverte.

État courant : reserved04(25anciens Rust) stable VERIFIED ; core05(8publics) PASS avant gardes de faits ; intent-progress-red06 reproduit décodage perdu après refus de liaison ; anchor-close-red07 reproduit matière non conservée malgré pin durable. Ordre corrigé : écrire paquet sous les deux réservations après pin Durable, puis terminer/fermer en gardant tout échec ; private09(7cas panne/compteurs) PASS. core08 conserve8publics PASS puis échec de compilation du seul test privé Box corrigé sans modification produit. cli-red10(6routes absentes) puis cli11(6publics) stable PASS. helper12 initial_request figé, source distincte du helper54 dont l’UID courant est côté service ; qualification12permutations et natif/binaire/UI/livraison en cours. Rien de ce57 n’est encore installé ou accepté globalement.


## Vérification locale terminée — portée exacte

Installé par build/restart pris en charge : **v0.1.1-dev.205**, PID1254617,
ELF `a66fe751bf8844df8be954c552c8bfd6c4fac382e1c210423162a4f6528510e1`.
Profil normal631entrées effectives/64entrées PAEA, sans test-fixtures ; même
ELF observé par le noyau et toutes les recettes natives. Les64entrées restent
identiques à la clôture. Les dialogues Version FR/EN ont été vus, avec numéro,
vrai commit modifié et notes ;56contrôles UI et refus de script passent.

| Contrat | Preuve retenue | Résultat local |
| --- | --- | --- |
| Helpers empruntés, comportements antérieurs | reserved04,25Rust | PASS |
| Création/relecture, phases, pannes et fermetures | final-core27,8publics+7privés Rust,3méthodes schéma intention | PASS |
| CLI privée, erreurs de sortie, rapport fermé | final-cli28,6Rust+3méthodes schéma rapport | PASS |
| Documents exacts | Rustdoc strict,6sources HTML égales aux octets+6contrats de fonctions ; archive HTML | PASS |
| Fixture sans matière, rôles distincts | helper12,roles13,12permutations avec témoins/reprises | PASS |
| Nouveau parcours sur ELF installé | active-initial18,6parcours/113commandes | PASS |
| Ancres/export/demande/installation/appel | active-anchor19 5/67 ; operator20 6/154 ; provisioning21 5/84 ; install22 9/66 ; call23 5/56 | PASS |
| Parcours personnels/canal/UI | personal24 7parcours+3gardes ; peer26 3paires/6commandes ; ui25 56contrôles+garde | PASS |

Intégrité des15dossiers de recette vérifiée APRÈS leurs résultats finaux.
Preuves : `evidence/20261027-initial57-idg95f_s/`, notamment
`closure-integrity.json`, `source-review.json`, `reviewed57-sources.tar.gz`
et `closed-local-result.json`. Revue politiques/fonctions/erreurs liée aux octets,
pas revue indépendante. Les46tests Rust ciblés désignent25antérieurs+15noyau+6CLI,
pas un dénominateur global. Les résultats plus anciens sont conservés avec leur
portée, sans prétendre que leurs inventaires entiers sont les sources finales.

Échecs originaux conservés : API absente ; hypothèse noncanonique contredite par
le producteur ; fait de décodage perdu ; matière perdue après fermeture tardive ;
constructeur de test Box ; cache Rustdoc faussement refusé ; collision du backup
FR/EN (original FR de planning exact non prouvé). Aucun n’est écrasé par le PASS.
Un appel status avec option supposée a refusé sans mutation ; l’aide établit
que status produit déjà JSON et la commande prise en charge vérifie PID/ELF.
Une recherche documentaire trop large a été réduite aux chemins sans preuves ;
ces incidents ne prouvent pas une correction générale de notre méthode.

Suite : conserver le refus d’intention orpheline, qualifier une remédiation
opérateur qui préserve l’historique, puis poursuivre remise au vrai domaine
service/admission et effets PA-03. Pas de compte/service/installation humaine,
autorité courante, UID séparé ou confinement démontrés. PA-02partiel,0sur17,
helpers UID administrateur, humain, E2/E3/PA-13 et RETEX final restent ouverts.
Deux lignes EOF étrangères et les liens historiques absents restent distincts
des contrôles propres à cette tranche. Aucun commit/push/agent/GLM exécuté.
