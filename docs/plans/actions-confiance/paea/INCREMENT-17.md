# PAEA — incrément 17 : résolution de facture depuis le menu

## Reprise et décision

Le 26 octobre, `go` reprend le programme après dev82 (SHA256
`2a328c56775fad8c9715d1c0bc2ca22b9d60fdbf258a25484f1b3f0cf2154e0d`,
PID 3871902 observé). Le résolveur de données et ses preuves 16 sont conservés.
L’autorité IA3 EP-01/04/05/07/10 et AC-02 restent ouvertes ; aucun rôle humain
n’est attribué par « go ». Codex reste seul propriétaire, sans délégation.

PA-08 peut avancer indépendamment : rendre le résolveur utilisable depuis
**Vérifications PAEA**. Options : deux formulaires à états croisés dans la même
page, ou une page de résolution reliée au menu, réutilisant le contrôleur de
fichiers. Choix : page distincte, mêmes styles et contrôleur partagé ; conserver
le parcours action/reçu et éviter deux implémentations de transport/session.

## Contrat et critères observables

1. Lien depuis Vérifications PAEA, retour au contrôle de reçu et au cockpit.
   Deux fichiers explicites, registre et demande ; aucune saisie d’accord/paiement.
2. `POST /api/v1/protection/resolve-payment`, même admission Host/session/origin/
   CSRF et borne de transport ; enveloppe fermée v1, chaînes UTF-8 exactes.
   Rapport identique au résolveur CLI, sans lecture/mutation Root par opération.
3. Décodage UTF-8 fatal, BOM conservé, nombres des entrées jamais parsés en JS.
   Révision i64 du rapport lue sans perte via le lexème JSON, comme le contrat
   de preuve existant ; navigateur incapable : refus explicite, pas d’arrondi.
4. Afficher objet de dette, version, montant entier/devise/compte et hashes ;
   téléchargement du rapport brut, avec autorité/authenticité/effet non établis.
   Refus et changement de fichier effacent tout résultat antérieur ; requête en
   cours bloque les doubles gestes, sans retry ni politique nouvelle.
5. Tests HTTP nommés, refus/reprises, contrôles du navigateur natif sur profil
   normal ; FR/EN, thèmes, 1440/390/320 px, clavier, contrastes, débordements,
   erreurs réseau, rapports incorrects et nombres exacts. Recettes maintenues,
   revue politiques/fonctions/erreurs liée aux octets, puis installation locale.

## Limites et reprise restante

Ce parcours contrôle des fichiers non authentifiés ; il ne crée ni registre
protégé, ni approbation, ni règlement. Le registre durable, la continuité des
IDs, les rôles humains et l’autorité courante sous ordre d’engagement demeurent
PA-02/03. Les deux UX métier ne deviennent pas deux applications qualifiées.
0/17 invariants de bout en bout ; E2/E3 et RETEX final du programme ouverts.
Pas de commit/push nouveau implicite ; préserver index et travaux étrangers.

État initial : décidé ; preuve avant correction, résultats et RETEX à compléter.

## Replans et garde de qualification

Premier test : mauvais accès au DTO `HttpReply` (`status/body` au lieu de
`response.status()/body()`) ; échec de compilation conservé dans `baseline.*`,
sans conclusion comportementale. Après lecture du producteur et correction du
test : contrôle reçu valide puis route de résolution 404, conservés dans
`baseline-02.*`. Cette deuxième preuve précède l’implémentation du handler.

La recette de composant héritée attendait encore le retour Biba, malgré le
changement cockpit vérifié à l’incrément 15. Échec 01 conservé ; assertion
actualisée vers texte/href cockpit, sans modifier l’application pour le test.
Reprise 02 : 219 assertions et les refus/reprises du composant passent.

La construction propre prépare dev83 ; la métadonnée de livraison partagée
observée ensuite désigne dev84, incluant huit entrées moteur étrangères. Leurs
hashes et l’archive sont conservés ; les 17 entrées modifiées depuis dev82 sont
détaillées dans `previous-delivery-delta.json`. Notre parcours a été qualifié
sur le candidat figé dev84, pas sur une supposition de chemin/version. Aucune
qualification des fonctions étrangères n’est déduite des tests PAEA.

Clippy du module HTTP entier échoue d’abord sur deux défauts hérités : conditions
accolées dans le classificateur d’erreur et tuple de query préparation complexe.
Options comparées : masquer les règles (rejeté), ou correction minimale de
qualification (retenue). Un seul retour à la ligne conserve tous les tokens et
leur ordre ; un alias documenté conserve exactement le type et le corps query.
Les substitutions inverses reproduisent les sources antérieures octet pour
octet (`clippy-fix-equivalence.json`). Aucun comportement de préparation ne
change et aucun test miroir n’est ajouté pour cet alias. Clippy `--no-deps
--lib -D warnings` et Rustdoc avec membres privés passent ; avertissements
moteur étrangers conservés, sans verdict global de migration. Requalification
HTTP et profil/runtime finaux exigés après ces deux changements.

La garde d’attente ouverte dans le RETEX 16 est réalisée sur la recette native :
les échecs navigateur ou la sortie inattendue du serveur de fixture rejettent
les attentes en cours. Un témoin normal précède le blocage volontaire du script,
sans délai arbitraire ; réception de la cause et échec attendu sont vérifiés.
Les recettes et ce suivi persistent ; aucun contexte d’agent natif démarré n’est
modifié par ces fichiers. Un appel d’outil a aussi été refusé pour parenthèse JS
surnuméraire avant toute action ; correction de syntaxe, sans mutation partielle.

## Résultat livré et RETEX final de cet incrément

**Installé : v0.1.1-dev.87**, PID 70398 observé, SHA256
`d98fbaa4aa6f65c0a80bfbb06bc3515f602ed84b774c516a9579c31c07d05446`.
Base Git réellement affichée : `4749f36eeca6b1bfc769d4ec7f01e55b67837dcb`,
commit moteur d’une évolution concurrente ; nos changements PAEA restent dans
le working tree et leur présence n’est pas attribuée à ce commit. Modifications
non commitées affichées dans Version. Aucun commit/push réalisé par cet incrément.

| Critère | Résultat | Preuves sous `evidence/20261026-ui17-xzzkd1qc/` |
|---|---|---|
| Contrats et admission HTTP | 4 tests de schéma + 4 tests HTTP PASS ; témoins, refus, reprise, i64 max et snapshot Root inchangé | `http-recipe-04` |
| Composant reçu conservé | 219 assertions PASS, transport simulé explicitement ; navigation héritée corrigée | `component-recipe-02` |
| Profil natif final | VERIFIED_DEFAULT_NATIVE_PROFILE, features core vides/test=false, aucune feature de fixture | `profile-87/profile/result.json` et archives effectives |
| Navigateur sur candidat figé dev86 | 452 assertions PASS, 12 configurations reçu + 12 résolution, FR/EN × clair/sombre × 1440/390/320 ; 36 vrais refus attendus, zéro diagnostic inattendu | `native-recipe-03`, `native-browser-03` |
| Nombres et export | Révision/montant i64 max exacts, fichiers originaux hashés, vrai téléchargement terminé et identique au rapport natif | fichiers `*-resolution/native-report.json` et export de `native-browser-03` |
| Refus client et récupération | Sept réponses altérées de navigateur refusées après témoins natifs ; perte de lexème numérique, UTF-8 invalide et double geste exercés, reprises valides | assertions `payment-client-*` / `payment-*` ; ces substitutions ne prouvent pas le serveur |
| Garde de supervision des tests | Témoin valide puis script bloqué dans le seul navigateur : sortie 1 attendue, cause `/paea.js` reçue et attente interrompue ; serveur détenu fermé proprement | étape 2 de `native-recipe-03` |
| Installation réellement servie dev87 | 30 contrôles PASS en lecture seule ; 10 assets uniques comparés, menus/fichiers vides, limites, Version/notes FR/EN, thèmes, lien Git, dirty et focus | `live-recipe-04`, `live-browser-04` |
| Garde sur l’instance | Témoin lecture seule puis script bloqué côté navigateur : échec attendu, aucune interruption du service | étape 2 de `live-recipe-04` |
| Statique et documentation | Clippy web `--no-deps --lib -D warnings` et Rustdoc privé PASS ; revue sémantique de notre delta, sans revue indépendante | `clippy-web-03.*`, `rustdoc-web-02.*`, `source-review.json` |

Après la qualification dev85, une nouvelle évolution des préparations a rendu
une construction intermédiaire dev86 invalide : `PreparationDocument` au lieu
de `Box<PreparationDocument>` dans `preparation_write.rs`. Refus du collecteur
conservé (`failed-profile-86`, `failed-profile-errors.json`) ; le lanceur n’a pas
arrêté dev84. La nouvelle version corrigée par son évolution fournit ensuite un
profil dev86 complet, courant et stable ; nous ne modifions pas cette logique
métier. Nos handlers/assets sont identiques à dev85 ; qualification native 452
rejouée sur le binaire dev86 figé. Aucun retry identique sans nouvelle preuve.

Le commit moteur concurrent change ensuite l’identité de build et produit dev87.
**Les 448 entrées de livraison dev86/dev87 sont identiques**, même profil et mêmes
overrides ambiants ; seuls version/commit/date compilés diffèrent. La preuve 452
porte sur dev86, les 30 contrôles live sur dev87. Cette réutilisation bornée est
conservée dans `metadata-only-delta.json` ; aucun faux replay 452 sur dev87.
Les attentes de métadonnées antérieures ayant rencontré dev84/dev87 sont
conservées dans leurs fichiers, sans déclaration d’installation du candidat attendu.

Le navigateur n’utilise ni base de mission réelle comme fixture ni POST sur
l’instance pour la recette live. Les POST de qualification sont effectués sur
un Root isolé ; les rapports altérés sont uniquement des injections client.
Captures résolution FR clair 390 / EN sombre 320 inspectées : aide repliée,
contrôles lisibles, chiffres intacts, textes/hashes sans débordement. La preuve
visuelle ne remplace pas les assertions et réceptions du navigateur.

Le contrôle de diff de notre périmètre et l’intégrité retenue sont suivis avec
la revue finale ; le contrôle global conserve les anomalies étrangères lorsqu’il
en rencontre. Aucune conformité globale de migration ou indépendance de revue.
Les règles réutilisables passent par les sources/recettes maintenues, le README,
le plan et les deux langues ; pas par une promesse de contexte natif mis à jour.

Usage : **Vérifications PAEA → Résoudre une référence de facture**, sélectionner
registre et demande puis **Résoudre la référence** ; lire/télécharger le résultat.
Les deux exemples du dépôt restent les entrées de démonstration. Le registre
reste non authentifié, sans accord ni règlement. PA-08 partiel, PA-02/03 protégés,
rôles humains, continuité/fraîcheur, campagnes UX, E2/E3 et RETEX final du programme
restent ouverts. **0/17 invariants de bout en bout**. Reprise protégée conditionnée
aux remises IA3/AC-02 et aux autorités réelles, sans les inventer depuis ces tests.

## Commit demandé — 26 octobre 2026

La demande explicite « commit » couvre les incréments 16 et 17. Le candidat Git
isole leurs sources, tests, schémas, recettes, documentation FR/EN et deux notes
PAEA de version. L’index préexistant est conservé ; les notes et passages de
vérification étrangers sont exclus. Les files locales APEX et les preuves brutes
privées restent locales ; les chemins de preuve ci-dessus ne garantissent donc
pas leur présence dans un clone.

Les octets des fonctions et parcours PAEA restent ceux des qualifications
conservées. Le README décrit maintenant les recettes courantes : 38 tests de
contrat (quatrième cas de schéma ajouté au lot 17), 30 contrôles live et dix assets ;
les résultats historiques 37/26 du lot 16 ne sont pas réécrits. La revue de
publication conserve les empreintes et l’isolation du candidat dans
`evidence/20261026-ui17-xzzkd1qc/commit-9um2zctn/`. Cette opération Git ne
constitue ni une nouvelle installation ni un push ; les identités des livraisons
ci-dessus restent leurs observations historiques. Les critères protégés ouverts
et le RETEX final du programme ne changent pas.
