# PAEA75 — flux pack natif, primitive sans autorité

Reprise12:17UTC, dernier74 et preuve réelle relus. Instance230/PID3682612/ELF
0b8cf6947596da380ed2f9ee862ed80597f949b2d3607d2147a787fd0d051287, HEAD2496c03,
aucun Cargo actif,127Go libres. Codex seul, sansGLM/délégation/commit/push.

Replan PA03 :74 qualifie les commandes mais pas leur supervision native.
Comparer implémentation directe complète service/stockage et primitive native
fermée réutilisant le superviseur existant : choisir cette dernière pour vérifier
séparément streaming fichiers, non-aliasing, erreurs/effets partiels et reprise.
Aucune primitive ne reçoit un mandat ni n'accorde droit d'engager une ref.
Le constructeur Git durci/politique existante est partagé, pas dupliqué.

Critères : six tests natifs nommés sur vrais Git/fichiers (valide/refs inchangées,
corruption/reprise, aliasage refusé avant spawn, sortie non vide refusée, export
OID absent et erreur typée/branches). Rustdoc strict, recette et profil normaux,
puis build/install exact/Version FR/EN si qualifié. Garde source/destination et
intention/autorité service restent séparément ouvertes. Échéance19h20Paris.

Replan méthode : archive73 des seuls fichiers modifiés ne contenait pas le runner
Git inchangé en73. Assertion préalable conservée, build non lancé. Lire le vrai
profil normal73/archive complète (fichier présent) puis comparer le bloc extrait
octet pour octet, sans changer les sources ni répéter une erreur comportementale.

## Livraison et limites réelles

Preuve : `docs/plans/actions-confiance/paea/evidence/20261028-native-pack75-_w86cpzq`.
Core01 :5tests PASS/VERIFIED, puis core02 :6tests PASS/VERIFIED avec garde réelle
d'erreur IO de sortie et reprise, graphes d'erreurs enveloppées compris. ELFs
conservés avant réutilisation du nom Cargo. Rustdoc strict PASS. Intégration03 :
6tests natifs association/format PASS/VERIFIED sur le constructeur Git partagé.
Le bloc de construction/délai est identique octet pour octet à l'archive normale73.
Une contradiction de compte dans le README permanent hérité de72 a été corrigée
(6=1normal+5SDK explicites, pas5), sans altérer les preuves historiques.

Version **v0.1.1-dev.231**, PID3829385, ELF
`ce4750b00da3e8ff7723b51f64b944bf3def76d0f8961eef5415c2e9bee7c387` préparé, installé par restart --prepared-build exact.
Profil sansfixtures/cœur features vides/test=false :682entrées normales,
482communes avec core02, aucune différence. Graphe normal230→231 identique ;
trois écarts transitifs tests/normal inchangés (once_cell/smallvec/time), revue
historique réutilisée dans sa portée. Pas de test unitaire cfg(test) présenté
comme production. Version06 :56assertions réelles + garde de script PASS/VERIFIED,
25assets/notes protégés compilés, FR clair/EN sombre, version/commit réel et dirty,
Escape/focus ; deux captures inspectées. L'identifiant de recette dérivée contient
encore73 comme provenance, pas comme version testée : status/notes/ELF prouvent231.
Aucun commit/push/délégation effectué. Diff global étranger reste séparé.

Le transfert complet service n'est PAS implémenté par cette primitive. L'appelant
doit encore retenir source/destination, fichiers privés, vérification/fermeture
et identité exacte, puis autorité courante/intention/COMMIT/ref/reconciliation.
Entrée File/Stdio fermée par Drop standard, aucun reçu de fermeture vérifiée.
Fsync des deux sorties tente chaque branche, sans injection dédiée ni attribution
stdout/stderr dans le type actuel : amélioration découverte en auto-revue75,
propriétaire Codex, à traiter avec le contrat de cycle de vie des fichiers avant
raccordement du service ; critère : fautes de sync distinguées par canal, causes
indépendantes conservées. Aucun échec fsync ni enfant non récolté nouveau injecté.
PA02/03 partiels,0/17,human/UID/confinement/indépendance,PA13/RETEX restent ouverts.
