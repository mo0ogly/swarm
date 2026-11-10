# PAEA76 — attribution des erreurs de synchronisation pack

Reprise12:47UTC : dernier75, instance231/PID3829385/ELF
ce4750b00da3e8ff7723b51f64b944bf3def76d0f8961eef5415c2e9bee7c387,
HEAD2496c03, aucun Cargo actif,127Go libres. Codex seul sansGLM/délégation/commit/push.
Replan de l'amélioration75 : les deux causes fsync survivent mais leur canal est
perdu. Avant raccordement au transfert complet, corriger ce contrat de diagnostic.
Deux routes : annotation textuelle seule ou enveloppe typée canal+cause dans le
graphe natif ; choisir la seconde, sans chemin privé ni parsing de message.

Reproduction normale : témoin valide pack, puis vrais fichiers Linux proc
réguliers/taille0/position0 mais non inscriptibles et fsync EINVAL. Aucun contenu
proc lu/loggé ; les tentatives d'écriture sont refusées par le noyau. Les deux
causes fsync doivent être distinguées sans perdre le runner primaire et sa garde
de récupération. Cas synthétiques séparés pour fsync après runner réussi ; pas
d'injection syscall réussie revendiquée. Le transfert privé/autorité/ref reste
ouvert, ce lot traite une dépendance explicitement conservée en75.

Revue du consommateur commun : ajouter à la recette les deux régressions
existantes de matrice direct/launch/customIO/Root-cleanup et source IO transparente.
Motif : native_error_links est partagé ; le seul cas pack Box/IO ne suffit pas
à qualifier la non-régression de ses autres enveloppes. Sources applicatives
inchangées, compilation cached ; nouvelle preuve03 avec manifeste final.

## Résultat local et portée

Preuve : `docs/plans/actions-confiance/paea/evidence/20261028-pack-sync76-o45ox7bj`. Red01 FAIL/VERIFIED avant correction,
assertion stdout sync channel missing après témoin valide, double EINVAL réel
conservé et ELF figé. Core02 PASS/VERIFIED :1test unitaire de composition avec
contrôle sain et erreurs stdout/stderr/les deux après statut réussi, puis7tests
d'intégration bibliothèque normale et Rustdoc strict. Core03 PASS/VERIFIED reprend
ce lot avec les2régressions existantes de matrice de wrappers/source transparente,
pour10tests nommés au total sous le manifeste final. Le nouveau cas noyau réel
conserve primaire runner, deux EINVAL et les deux canaux sous Box/IO, puis reprend
avec pack valide. Les entrées synthétiques unitaires ne sont pas des syscalls.
Les deux ELFs unit/cfg(test)=true et intégration/cfg(test)=false sont distincts,
conservés. Régressions75 association/format réutilisées pour leurs sources
inchangées, revue explicite retenue ; aucune conformité globale déduite.

Version **v0.1.1-dev.232**, PID3954299, ELF préparé exact
`7cb94d74a30343d472f20e94f5786f9cfefff4b02415fb2afaf3f8733a0f52d7` installé par le chemin géré. Profil sansfixtures,
682entrées normales/482communes à core02, aucune différence. Graphe normal231→232
inchangé, mêmes3écarts transitifs tests/normal documentés précédemment. Version06 :
56assertions réelles et garde de script PASS/VERIFIED,25assets/notes compilés,
FR clair/EN sombre, vrai commit/dirty, Escape/focus ; deux captures inspectées.
Aucun commit/push ni délégation. Diff global étranger séparé, propres octets revus.

Le build a progressé plus lentement sous pression mémoire/swap observée ; CPU
rustc continuait d'avancer, pas de blocage établi. Observation brute conservée,
pas de kill/retry/watchdog ni intervention sur les processus d'autrui.
L'amélioration75 d'attribution sync est réalisée et vérifiée dans cette portée.
Le cycle de vie privé des fichiers/capacités et le transfert complet vers le service
restent ouverts. Entrée File/Stdio Drop standard inchangée, aucune fermeture
vérifiée inventée. PA02/03partiels,0/17,UID/humain/confinement/indépendance,
E2/E3/PA13 et RETEX final restent ouverts.
