# PAEA — incrément 15 : compilation, parcours natif et publication

[English](../../../en/plans/actions-confiance/paea/INCREMENT-15.md) ·
[Plan](../PAEA-APEX.md) · [Contrat](../../../architecture/PAEA-CONTRACT.md)

## Autorité et état repris

L’utilisateur demande « fini push compile ». Cette instruction lève la consigne
antérieure sans compilation/redémarrage et autorise le commit/push de PAEA ;
l’installation locale suit l’accord permanent. Elle ne donne pas de mandat de
paiement, d’effet Git externe, de délégation, d’E3 ou d’acceptation globale.
Codex racine travaille seul, avec APEX/replan et vérification du parcours réel.

Reprise : instance Rust dev78, projet et PID identifiés par `swarm.sh status` ;
branche `codex/clear-product-method-names`, base initiale `d566ed35c`. Plus de
1200 changements suivis comprenaient la migration d’une autre session. Un
worktree de qualification est donc créé, sans déplacer le checkout de travail.
Pendant la qualification, cette autre session publie `823479b31` (dépendances
runtime) et `ba581aec6` (distribution Rust seule). Nouvelle précondition observée :
ces dépendances sont désormais versionnées et publiées. Le worktree reprend ce
socle par fast-forward ; aucun de ces changements n’est réattribué à PAEA.

Deux options : publier en masse le checkout partagé ou sélectionner les fichiers
PAEA et les tester sur le socle publié. Seconde option retenue. Aucun changement
étranger, index d’une autre session, binaire, cache ou base Root ne fait partie du
lot. Les petits fichiers de preuve explicitement liés sont conservés à l’identique ;
les répertoires complets non publiés ont un index d’empreintes explicite. Les
sources de guide sont conservées, sans doublon de téléchargement ni rendu de QA.
Les références aux files/plans transversaux locaux restent des chemins nommés,
hors publication PAEA ; leurs propriétaires et leurs prérequis sont conservés.

## Changements et résultats

La page PAEA revient directement au cockpit ; son déploiement ne dépend pas de
Biba. Les notes locales FR/EN décrivent les fonctions réellement présentes et
leurs limites. Schémas, tests, outils CLI et laboratoire, documentation FR/EN et
guide sont préparés pour publication avec les sources déjà versionnées.

La recette `paea-contract.json` couvre désormais les sources Rust/transitives,
ressources embarquées et workflows concernés ; elle ne se limite plus aux
fichiers PAEA directement modifiés. Les entrées réelles du build de production
restent établies séparément par le collecteur de profil et ses depfiles.

| Critère | Preuve observée |
|---|---|
| Contrats et schémas | 5 tests Python + 45 tests Rust nommés, neuf étapes PASS dans le checkout, puis dans le worktree ciblé. Zéro cible vide. |
| CLI | Canonicalisation/comparaison/reçu/v2, deux domaines, conservation v1, refus/reprise et erreurs d’écriture. |
| Corpus public sur dev79 | 18 tests/973 invocations PASS sur le binaire normal figé ; refus attendus et identité du binaire contrôlés. |
| Laboratoire sur dev79 | Sept tests PASS rejoués avec les deux vrais sinks de fixture et leur oracle ; contre-exemple de dette conservé. |
| HTTP natif | Deux tests passants, admission Host/session/origin/CSRF, correspondance/refus/reprise, menu/ressources privées. |
| Build normal | `./swarm.sh build` prépare dev79 avant tout arrêt du serveur ; exécutable figé, profil/dépendances/entrées effectives retenus. |
| Navigateur réel | `paea-native-ux.json`, vrai exécutable dev79 sur Root isolé : **237 assertions**, FR/EN × etat/sombre ×1440/390/320, deux domaines. **24 refus attendus**, reprise et zéro diagnostic inattendu. |
| Ressources | Huit fichiers servis sous authentification comparés octet pour octet aux sources ; aucune réponse de vérification simulée. |
| Version | Dialogue réel FR/EN : version, notes PAEA, vrai lien de commit et restauration du focus. La capture montre la base Git et l’astérisque de modifications non commitées ; aucune version inventée. |
| Visuel | Captures mobiles clair/FR, sombre/EN et dialogue Version examinées ; absence de débordement assertée sur les 12 configurations. |

Le test navigateur n’emploie ni pont HTTP ni double de contrôleur. Il initialise
sa propre Root, vérifie le hash du processus exécuté, lit seulement sa clé privée
de fixture via un descripteur non symlink et ne journalise jamais sa valeur.
Ses stderr restent conservés dans la fixture privée. Il arrête seulement son
processus ; ni fournisseur ni mission vivante n’est sollicité. Chaque refus est
précédé d’un témoin valide et suivi d’une reprise. Aucun watchdog/retry ajouté.

## Revue et reprise

Dossier local : `evidence/20261025-native15-eob8z_9k/` (nom conservé ; dates réelles
des collecteurs, UTC et fuseau local, font foi). Originaux : `current-contract-01`,
`publish-contract-01`, `native-browser-recipe-01` et `native-browser-01`.
Les erreurs d’ancrage/lecture des outils et les publications concurrentes sont
diagnostiquées avant reprise, sans relance inchangée ni suppression des preuves.

La revue finale lie politiques, documentation des fonctions et propagation des
erreurs aux octets retenus. Les domaines, tailles d’écran et nombres de cas sont
des fixtures/critères de couverture ; aucun plafond de politique produit ajouté.
Les erreurs de service/sortie/lecture restent des refus ou issues inconnues avec
cause conservée. Les contrôles Rust ne remplacent ni le profil livré ni le
navigateur ; chaque preuve garde sa portée. Aucun acteur humain ou registre
métier n’est créé par compilation ou publication.

Publication et installation se vérifient par l’observation du commit distant,
du PID/hash et de la version réellement servie ; le candidat dev79 ne constitue
pas à lui seul cette installation. Le registre local `delivery-result.json`
conserve l’identité finale, les contrôles après activation et les limites.
Ce résultat généré reste distinct du commit de sources.

Le `.gitignore` du dossier de preuves prévient l’ajout accidentel des sessions
privées, profils de navigateur et binaires. Seuls les petits résultats/images
explicitement examinés sont sélectionnés pour publication ; les originaux
complets restent locaux, avec leur profil et leurs empreintes.

## Limite du résultat

Il s’agit de livrer le contrôle de correspondance et le laboratoire existants.
PA-02/03 protégés, registre de dette, confinement IA3/Rust/AC-02, campagnes depuis
l’UX, E2/E3 et RETEX du programme restent ouverts : **0/17 invariants protégés
démontrés de bout en bout**. Le contre-exemple d’alias PAEA-GUIDE-03 reste ouvert.
Un formulaire natif compilé n’est pas un service de paiement/promotion protégé.
