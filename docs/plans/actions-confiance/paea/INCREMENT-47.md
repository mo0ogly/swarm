# PAEA — incrément47 : réponse de service et erreurs après effet

Reprise46 :19tests/4étapes, Rustdoc/revue liée aux octets qualifiés en bibliothèque,
pas de endpoint ni livraison publique déduite. Helpers Linux absents, opération
administrateur encore requise. Codex direct seul sans GLM/délégation/commit/push.

Contrat retenu : lier Reply à opcode et empreinte de la partie PUBLIQUE exacte de
la requête (domaine séparé, header/cible/challenge/opcode/longueur/Change), exclure
proof/password du hash. Aucun gain de fraîcheur ou admission déduit. JSON fermé
canonique, erreurs par couches, view/projection connues conservées même après
échec d'autorité, phases personnelles exactes. Ne jamais transformer Ok de routage
ou d'envoi en succès métier. Anciennes réponses Go distinctes/inchangées.

Encodeur de réponse et write/flush consomment Reply et conservent ce propriétaire
complet dans succès comme refus : toutes les causes typées originales restent
inspectables, y compris Outcome AfterUse avec business AfterStorage interne.
Erreur write = issue d'envoi inconnue, flush après write = écrit puis flush refusé.
Pas de rollback/retry automatique. Aucun secret, digest de credential/head privé,
chemin privé ni cause OS brute dans JSON/Debug. Le client devra vérifier émetteur
et liaison de requête avant résultat ; décodeur public/raccord CLI restent suivants.

Critères : digest public identique si seuls proof/password changent, différent
sur cible/challenge/opération/Change ; vraies réponses init/enroll/recover/control
et refus, conservation des branches imbriquées et des views connues après faute
synthétique explicite, encodeur puis write/flush en erreur avec Reply retenu.
Recette persistante, Rustdoc/FR-EN/revue/inputs exacts. Ni callback synthétique ni
fixture à UID courante ne prouvent un service protégé ou E2/E3/humain/0sur17.

Résultat bibliothèque47 : recette finale03 PASS24cas Rust+3cas Python schéma,5étapes dont Rustdoc privé -Dwarnings, inventaire stable et intégrité VERIFIED. Les recettes01/02 (23puis24cas Rust) restent historiques. Liaison publique comparée sans hash secret, vraies phases/résultats, fautes synthétiques explicitement distinguées, write/flush/encode sans perte du Reply d’origine. Schéma enveloppe aligné au jeu exact de champs produits par Rust ; ses3tests n’affirment aucune sémantique View ni admission. DiagrammesFR/EN rendus et examinés, revue locale liée aux octets. Aucune livraison de service public/OS/humain/E2/E3/global déduite. Suite48 : validation côté consommateur et raccord opérateur natif, profil de production distinct, avant transport protégé réel.
