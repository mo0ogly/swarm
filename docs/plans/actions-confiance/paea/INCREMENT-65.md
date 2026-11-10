# PAEA65 — permission exacte distincte pour les services d’effet

Reprise : le tour documentaire précédent a localisé les vrais contrôles mais
n’a établi aucun effet nouveau. Ce tour implémente la frontière de permission
nécessaire au raccord PA-03 : service, opération, stockage et action exacts dans
un namespace distinct. Propriétaire Codex direct, aucun GLM/délégation/commit/push.
Le choix ne crée ni droit signé ni service protégé activé.

Plan borné : (1) partager le contrôle de droit courant sans modifier l’API Trust ;
(2) introduire Request/Identity PAEA distincts ; (3) vérifier refus, reprise et
résultats tardifs, puis les trois régressions Trust ; (4) documenter et qualifier
les sources/profil, construire et installer l’avancée par le chemin pris en charge.

Code : `control/held_action.rs`, `control/paea_action.rs`, wrapper authentifié,
réexports et adaptation interne Trust. Le constructeur PAEA valide seulement les
représentations : le futur consommateur doit lier son stockage ouvert, son action
canonique et le contexte courant. Aucun effet PA-03 n’est encore exécuté.

Recette permanente : `tools/verification/recipes/paea-exact-effect-authority.json`.
Huit tests PAEA ciblés, trois régressions Trust, trois refus de compilation et Rustdoc strict PASS dans core02.
Preuves fraîches `evidence/20261027-paea-action65-ivbw3kuu/`, originaux avant édition
conservés. Pas de watchdog global, pas de données d’une mission active en fixture.

Contrat FR/EN : `docs/architecture/PAEA-EFFECT-AUTHORITY.md` et
`docs/en/PAEA-EFFECT-AUTHORITY.md`. Politiques : constantes de protocole uniquement,
aucun délai/retry/budget opérateur nouveau. Fonctions : nouveaux contrats Rustdoc,
capacité non désérialisable et durée empruntée ; revue finale à lier aux octets.
Erreurs : action/autorité/after-use personnelle indépendantes, cause et résultat
conservés ; vérifiés dans le périmètre local déclaré.

Disque observé 2,4Go libres, aucun cache partagé supprimé ; sources de moteur
étrangères préservées. PA-02partiel, deux UID/humain/daemon/confinement/PA-03complet,
0sur17 globaux, E2/E3/PA13/RETEX restent ouverts.

## Résultat vérifié et limites

Bibliothèque compilée dans la livraison locale **v0.1.1-dev.218**, ELF
`e4b1b5adeb3a83073367656de85cc8c9c677096f83d1615ef2dd2d42fa89eb41`, PID2497672
à observation05. `build03` puis `activation04` pris en charge, profil normal671
entrées, features de cœur vides/test=false/sans fixture, 6producteurs revus exacts.
Aucun parcours public nouveau n’invoque encore `with_paea_action` : sa preuve
comportementale est celle des tests de bibliothèque, pas un effet natif installé.

`core01` PASS10tests initiaux conservé, puis `core02` PASS8PAEA+3Trust+3refus de
compilation et Rustdoc strict. Pas de cumul trompeur : les anciens tests ne sont
pas additionnés aux mêmes régressions actuelles. Trois diagnostics Rust observés :
non-désérialisation, types Trust/PAEA distincts et capacité ne pouvant quitter son
callback. Cinq pages Rustdoc générées et empreintes conservées.

`version06` PASS56assertions de parcours existants et dialogue Version FR/EN,
notes exactes, lien vers vrai commit et indicateur modified, Escape/focus,
25ressources protégées comparées aux octets compilés, aucune requête métier.
Témoin valide puis script manquant refusé. Deux captures FRclair/ENsombre inspectées.
Les 3intégrités de recettes sont vérifiées. Sources de contrôles et archive de
revue liées aux octets ; revue Codex locale, aucune acceptation indépendante.

Pas de timeout, budget, retry ou plafond caché ajouté. Équivalence textuelle du
corps commun vérifiée après les deux seules substitutions explicites : permission
immuable passée en argument et callback document au lieu de construction Trust.
Les warnings résiduels du helper de fixture partagé ne concernent pas une erreur
de bibliothèque ; aucune suppression globale de warning introduite.

Suite : consommateur PA-03 dérivant la requête du stockage ouvert et de l’action
canonique, contexte courant et services séparés durables. Ne pas réutiliser un
Trust commit comme droit PAEA. UID/helpers/humain/daemon/confinement, deux effets
protégés/E2/E3/PA13/RETEX restent ouverts ; aucune clôture globale.
