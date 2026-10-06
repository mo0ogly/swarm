# T4 — dossier actuel de reprise ciblée

## Résultat actuel et attribution

La production terminée est celle de l’agent 3500cb7c-3823-48cf-a454-295a032b309e, tentative a-9b4502ef490c0a9b2227676b. Le producteur a exécuté ses contrôles ciblés et ne disposait pas du navigateur. Sa limite initiale est conservée dans T4-handoff.worker-original.md et T4-report.worker-original.md ; ce bilan initial ne décrit pas les observations effectuées ensuite par la supervision.

La recette web complémentaire est réalisée par Codex avec CUA, comme un utilisateur. Elle n’est pas attribuée au producteur ni au vérificateur sans outils. Elle vérifie un parcours utilisateur supervisé, sans prétendre démontrer une autonomie sans pilote externe.

## Critère 1 — changements, preuves et exigences

L’aperçu distingue les preuves modifiées, inchangées et inconnues et recalcule les critères restant à vérifier.

Les tests réellement exécutés couvrent fichiers modifiés/identiques, fichier manquant ou trop grand, chemin hors projet, 64 preuves et borne totale de 32 Mio. Une preuve inchangée est un document réutilisable, pas une validation. L’aperçu a réellement affiché le changement de consigne, le fichier corrigé et les critères restants.

## Critère 2 — périmètre et limites

Le contrôle CLI des événements et de la tentative confirme deux tentatives de production et un plafond d’outils inchangé.

La comparaison porte uniquement sur les fichiers liés au dernier avis : aucun inventaire de dépôt. Aucune nouvelle production et aucun plafond augmenté. Les refus, appels de revue consommés et rapports initiaux restent conservés. Reprise explicite du même résultat après changement des preuves liées, puis décision de validation séparée.

## Critère 3 — recette complète réellement exécutée

La recette complète Codex/CUA a constaté la fiche Bloquée, affiché la correction r282 et enregistré la reprise web r283 ; la fiche affiche ensuite À vérifier.

Cette recette réellement exécutée termine avec un code de processus 0. Les trois observations ont lieu dans les 105 secondes suivant le défi nouveau 84a714566ebedd5184fcffcbd2d2ac40, prévu pour 300 secondes ; aucune observation antérieure au défi n’est réutilisée. Les captures de fiche refus/correction/reprise sont t4-inspector-84a714566ebedd5184fcffcbd2d2ac40-*.png. Le processus compare aussi les événements durables CLI et leurs ordres. Le reçu complet et son log sont T4-full-fresh-recipe.json et T4-full-fresh-recipe.log.

La revue r277 demeurait non favorable. Après un essai de présentation expiré, la tâche a été explicitement remise en Bloquée via CLI public à r281, sans modifier ses tentatives ni ses budgets ; cette remise est conservée comme une action du superviseur, pas attribuée à un nouvel avis IA. La correction Next r282 et la reprise r283 ont ensuite été réalisées. L’essai expiré de 120 secondes n’est pas compté comme contrôle réussi. Les anciennes captures d’avis restent archivées ; elles ne sont pas attachées comme preuve visuelle de cette nouvelle recette.

## Contrôles et limites de preuve

Les contrôles moteur Go et frontend passent sur les sources liées ; les commandes, codes de sortie et captures explicitement autorisées sont dans les reçus. Les contrôles CLI lisent les vrais événements de la mission. Le reçu CUA est produit durant une interaction réelle demandée par un défi ; il reste une preuve externe attribuée, pas une certification cryptographique de l’origine d’une image. Le moteur contrôle ce reçu en lecture seule avant la revue : modifier la tâche pendant une validation à révision figée serait invalide.

Les tests de protocole sont des fixtures lorsqu’ils le disent ; le fournisseur de la tentative rapporte claude-sonnet-5. Les requêtes internes et le coût total non rapporté ne sont pas inventés. Le code est non commité sur base 1d9570bd4ef617130c6be96b7ec88844fdbcd00e ; candidat exact dans T5-candidate.json.

Aucun avis favorable ni statut terminé n’est anticipé ici : revue indépendante et acceptation restent requis.

## Incident de format de la revue r288

La revue a été enregistrée en erreur : son extrait JSON supprimait les retours à la ligne du contrôle CLI. Le garde de citation exacte a refusé cet extrait ; cet incident n’est ni un échec du contrôle exécuté ni une validation du critère. La sortie du contrôle CLI est maintenant sérialisée sur une ligne, sans modifier ses assertions, événements, tentatives ou plafonds. Un nouvel avis reste nécessaire.
