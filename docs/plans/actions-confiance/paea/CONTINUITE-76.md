# Reprise après76 — erreurs de sync attribuées, transfert protégé restant

Codex direct sansGLM/délégation/commit/push. Revalider heure/dernier lot/source,
changements concurrents et vrai ELF ; réveils bornés28oct19h20Paris. Ne pas utiliser
PID/version hérités comme état courant. Aucun nouveau goal/automation.

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

## Prochaine action PA03

Relire les producteurs réels de stockage géré et la capacité privée72. Introduire
une garde source détenue et le cycle de vie privé des fichiers puis le transfert
natif avec vérification du candidat/format et refs inchangées. Comparer callback
with_storage actuel (ferme la lane en sortie) et garde possédée de durée explicite :
si enfant non récolté, ne pas libérer ses dépendances de custody/lock en retour
d'erreur. Préférer une garde/job conservée avec la cause plutôt qu'un booléen
précheck ou un argv /proc/PARENT/fd dont le parent serait fermé trop tôt. Ce choix
est à implémenter/vérifier, pas déjà une capacité du moteur.

Réutiliser run_git_pack75/76 et vraie politique Git sauvegardée, pas de Vec de
pack ni plafond caché. Les objets partiels après refus ne sont pas un rollback.
Ensuite admission65/66 + writer73 + intention70 + transaction69 avant COMMIT/ref.
Existing/Uncertain ne donne jamais droit à retry ; réconciliation terminale encore
à faire. Le service doit retenir autorité/Trust/plan/rôles au point d'engagement.
Ne pas abaisser séparation/incompatibilité ni fabriquer enrôlement ou UID.

Voir CONTINUITE73/74/75 pour fondations et vraies fixtures CLI inertes, jamais
mission utilisateur/SQL positif. UID/humain/acceptations déjà pendants : aucune
question répétée. Préserver données, preuves, processus et caches partagés. Éditer
et formatter sous build lock nonbloquant, le libérer avant launcher. Inspecter
profil et véritable ELF/assets/Version FR/EN après toute livraison qualifiée.
Les diagnostics de pression mémoire76 sont un constat ponctuel, pas un timeout
ou une autorisation d'arrêter d'autres travaux. Continuer sans relance.
