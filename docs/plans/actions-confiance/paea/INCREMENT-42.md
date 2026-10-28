# PAEA — incrément42 : artefact destiné au service

41vérifié67tests/Rustdoc, normal159/160mêmes sources et7CLI chacun. Livraison au
catalogue actuel encore ouverte : UI160compare correctement les octets embarqués,
workspace i18n/notes modifiés ; rebuild162refusé pour sources concurrentes. Ne pas
affaiblir la recette ni activer un candidat non qualifié ; travail indépendant.

Avant publication43 : le paquet40contient clé métier et preuve initiale, il ne doit
jamais entrer dans les fichiers du service personnel. Options : faire ouvrir ce
vault au service, ou produire un artefact de remise limité. Choix deuxième.
Propriétaire Codex direct, sans GLM/délégation/commit/push.

Contrat : ServiceArtifact non Clone/non Serialize, Debug expurgé, seed personnelle
Zeroizing, déclaration v3 canonique et deux propositions initiales exactes déjà
signées. Aucun nouveau tirage/KDF/compte/grant. En-tête versionné, seed32 brute,
puis trois longueurs u64BE et octets (déclaration/événement métier/personnel), EOF
exact ; additions/conversions vérifiées sans plafond opérateur. Une allocation
privée réservée avant copie de seed. Pas de businessseed/proof dans ce format.
Decode exige un SHA256 extérieur canonique avant parsing ; donnée à protéger par
l'appelant, jamais apprise du message. Vérifier clé personnelle/pins et layout
initial fixe sous une racine, heads séquence1/digests exacts, versions/générations.

Le codec ne vérifie ni signatures d'événements opaques, ni autorité actuelle ni
présence humaine/UID réel.43utilisera les vrais journaux existants dans une zone
privée de préparation, puis leurs vérifications d'autorité, AVANT publication
atomique du nom final. Pas de seconde implémentation de cryptographie. Un test
doit distinguer décodage de données et refus cryptographique du journal réel.

Critères : encodage/restauration exacts, clé personnelle liée à la déclaration,
absence structurelle des secrets opérateur, refus pin/header/longueur/troncature/
trailing/binding/layout, diagnostics expurgés et erreur de sink avec phase source
conservée. Signature réelle depuis le producteur vérifiée par application sur
journaux isolés, jamais sur base vivante. DocFR/EN/recette/revue/Rustdoc puis
livraison supportée quand source globale qualifiable.43publication/récupération
des fichiers/journaux avec UIDservice réel, objets vault/service séparés, intention
et pin externes avant effets, aucun head appris/reset.0/17/E2/E3/PA13toujours ouverts.


Résultat bibliothèque : 18 tests nommés dans 3 étapes PASS, entrées stables et
intégrité vérifiée ; Rustdoc privé avec avertissements refusés PASS. Les six tests
nouveaux couvrent le format, les liaisons, les secrets exclus, les vraies signatures
et la reprise sur le même journal, plus la phase d'erreur de sortie. La revue a
supprimé une copie gardée inutile de seed et renforcé le témoin de reprise.
Schémas FR/EN rendus et examinés. Le codec reste sans effet fichier ; publication43
et installation au profil normal en cours. Aucun critère global clôturé.


Livraison locale vérifiée : **v0.1.1-dev.165**, SHA256 `0fd085c7885b6275cbb54c4d3cca9460fb506a99bdd9247fae4872ca5610997b`. 7 parcours CLI natifs et 56 contrôles navigateur plus refus de script en erreur PASS ; ressources HTTP protégées, Version et notes FR/EN liées au profil normal. Cela ferme la livraison locale du codec42 et des noyaux40/41, pas la publication des fichiers, le service personnel, les humains ou les invariants globaux. [Preuves42](evidence/20261027-service-artifact42-3n0wricj/closed-local-result.json).
