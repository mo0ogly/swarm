# PAEA — incrément27 : déclaration épinglée et contrôle natif de démarrage

## Reprise et décision

Commit26 `cbd6fecd7e40016d6c259296ac3a21373a4bbd60`,15fichiers,108chemins
préparés d’autrui préservés. Reprise directe Codex, sans GLM. PA-02 reste partiel.
Le noyau attend encore un Startup fourni par son appelant. Le bootstrap v1 ne
contient ni projet, journal ni enrôlement attendu : il ne peut pas le produire.

Deux routes : commencer le transport en inventant son contexte, ou lier d’abord
ce contexte à une déclaration d’installation vérifiée. Choix : déclaration v2
fermée, distincte du v1 conservé, avec checkpoint d’autorité indépendant. La nouvelle
commande `identity control-check SHA256 --input FILE` relit le vrai journal signé
sous réservation. Elle ne crée aucun grant et n’ouvre pas le Coordinator. Pas de
service ni de session humaine implicite ; v1 et ancien parcours inchangés.

| Étape | Contrat observable | Propriétaire / preuve |
| --- | --- | --- |
| 27a | Déclaration v2 canonique, pins/contexte/checkpoint précis, chemins protégés distincts | Codex ; sérialisation stricte, refus avant écriture, reprise |
| 27b | Précontrôle du journal réel lié aux pins et à l’enrôlement courant, causes conservées | Codex ; valide, révoqué/périmé, contexte/temps erronés, contention |
| 27c | CLI native normale, FR/EN, erreurs sortie/flush et options interdites, sans projet ouvert | Codex ; vrais processus et état avant/après |
| 27d | Recette, revue liée aux octets, build/install local pris en charge et dialogue Version | Codex ; profil normal sans fixtures, binaire/assets servis exacts |

Le checkpoint est une attente indépendante de cette observation, pas un grant ;
une évolution du journal le rend périmé et exige un checkpoint autorisé courant.
Le précontrôle ne remplace ni un bootstrap humain initial consommé une fois,
ni une session humaine, un contrôle de requête ou le confinement. PA-03/E2/E3,
0/17invariants protégés et le RETEX global restent ouverts.

## Capacité et originaux

Seulement412Mo libres au départ. Nettoyage borné des caches incrémentaux de notre
cible PAEA, sous verrou Cargo exclusif, inventaire avec hashes conservé avant
suppression. Aucun source, binaire, depfile ou original de preuve supprimé.
4,44Go de caches régénérables retirés ; environ4,62Go libres après nettoyage.
Preuves `evidence/20261026-bootstrap27-bjes7f9p/`. Un premier appel d’orchestration
avait une parenthèse JS excédentaire ; aucun outil ni écriture exécuté, appel corrigé.
Aucun commit ou push de la tranche27 n’est demandé.


## Résultats, livraison et RETEX borné

Recette `paea-control-startup`, `recipe-final-02` : **42 tests nommés, six étapes
PASS** (v2 4 ; v1 5 ; noyau 9 ; CLI 6 ; erreurs secondaires 1 ; régression CLI 17).
Rustdoc bootstrap/autorité passe avec liens cassés refusés. Le rouge initial est
une compilation sans API nouvelle : aucun scénario exécuté. `recipe-01` conserve
le refus de compteur : quatre anciens tests exécutés sur cinq attendus ; la feature
fixture du cinquième a ensuite été explicitement activée, sans réduire le contrat.
Les refus précoces pin/fichier/JSON sont aussi vérifiés sous contention réelle du
journal. Les branches de cleanup synthétiques prouvent leur transport diagnostic,
pas l'apparition d'une panne disque réelle. Le factory test ouvre le vrai noyau,
mais ne prouve ni transport ni authentification humaine.

Installation par `./swarm.sh restart --no-open` : **dev.123**, empreinte
`a5bcad3c3eb9ce60b78ea0b83414715ce918fe221761c33a87d1cb924106826f`.
Une livraison parallèle a ensuite activé **dev.124**, empreinte
`cee2ab96e0d3f09e93d17bfc9aca712ae89333056589a37fa85c1af7c325fe0d` ;
elle a été observée sans interrompre ce travail. Les deux profils normaux retiennent
490 entrées locales effectives, sans feature fixture ; les 12 entrées Rust/manifests
propres à27 sont identiques. Chacun des deux vrais binaires installés a passé les
six tests rejoués avec l'ELF de test figé : cinq parcours de processus natifs et un
test de factory lié au binaire de test (ne pas attribuer ce dernier au CLI livré).

`live-recipe-02` : **56 contrôles navigateur**, FR/EN, thèmes et 25 ensembles
d'assets PAEA exacts ; dialogue Version, numéro, notes, commit réel
`cbd6fecd7e40016d6c259296ac3a21373a4bbd60`, modifications non commises, focus/Escape,
aucune écriture et aucun diagnostic inattendu. Les deux captures Version ont été
inspectées visuellement. Le défaut de script injecté dans le navigateur est refusé
comme attendu et ne modifie pas le serveur. Le premier lancement navigateur
n'avait exécuté aucun scénario : Chrome mis en cache absent ; reprise avec le
Chrome local installé `/usr/bin/google-chrome-stable`.

Apprentissage : les métadonnées du dernier build **préparé** peuvent précéder une
activation parallèle. La preuve de livraison part du processus et du SHA servi,
puis choisit le profil correspondant ; elle ne déduit pas le binaire exécuté de
`deploy/local-build.json`. L'incident de méthode et les observations originales
sont conservés. Une lecture avait aussi supposé `inputs.after.json` dans le profil ;
son inventaire réel est `after.json`, utilisé après inspection, sans relancer le build.
Deux erreurs de syntaxe JS avant appel d'outil n'ont exécuté aucune mutation.

Après les tests, un second nettoyage sous verrou a retiré 1,09 Go de caches
incrémentaux identifiés ; ELF, depfiles, sources et preuves conservés. Aucun commit
ni push27 ; les notes étrangères de `version_history.json` et l'index d'autrui sont
préservés. Revue explicite locale liée aux octets : `source-review.json`, axes
politiques/fonctions/erreurs/documentation. Elle n'est pas une revue indépendante
ni une qualification de toute la migration. `final-checks.json` lie les contrôles
finaux, archives et vérifications d'intégrité des recettes conservées.

PA-02 reste **partiel**. Enrôlement personnel réel avec preuve d'installation
indépendante à usage unique, session/transport authentifiés et service sous compte
séparé restent à réaliser ; PA-03, confinement, E2/E3 et **0/17 invariants globaux**
restent ouverts. Propriétaire technique : Codex ; enrôlement humain : opérateur ;
évaluation E3 : tiers non encore désignés. La demande du26 octobre de poursuivre
en boucle autorise la suite technique immédiate sans nouveaux « next » ; aucun
consentement humain ni résultat indépendant n'est inventé.
