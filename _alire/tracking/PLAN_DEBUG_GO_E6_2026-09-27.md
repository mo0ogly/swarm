# Debug E6 — correction explicite après erreur de revue

Méthode : debug-go de lia-sec-dev, adaptée au build Go de Swarm.
Cause reproduite : recoverResult confond refus IA valide et autorisation de
correction opérateur. Une citation mal localisée laisse une revue error et rend
la correction impossible. Test public Store FAIL avant, PASS après (0.934s).

Décision : conserver recoveredReviewRefused inchangé ; nouveau prédicat limité
à la réparation avec confirm_review_error_repair=true, terminalité, preuves et
plan intacts, zéro réservation active. Gardes d’identité, arbre différent, contrôles
et nouvelle revue préservés. CLI/HTTP partagent PlanningRequest.
Alternative rejetée : accepter ou réécrire la réponse IA invalide, élargir la
notion de refus, modifier la base active, lancer un nouveau producteur.

Validation : ciblés14.582s PASS, reproduction/guards0.934s PASS, race59.130s PASS, vet/contrat/diff PASS ; suite complète357.154s PASS.
Aucun appel modèle consommé pour diagnostic/tests ; fournisseurs simulés.
Le budget de la mission (68/73) et E7/E8 ne sont pas résolus par ce correctif.

Journal : COLLECTOR capture error/citation ligne24 vs26 ; ANALYST confirme
les deux gardes ; EXECUTOR sépare les prédicats ; tests refus sans confirmation,
preuve altérée, revue active, nouvelles preuves/revue, conservation historique.

## Reprise debug du 28 septembre — citations par lots
Cause confirmée localement et par analyste séparé : le lot en erreur est conservé mais son diagnostic ne passe jamais dans le prompt de reprise. Correction : feedback calculé depuis réponse vérifiée et parseur courant, stocké dans le nouveau claim, ajouté avant le contexte original ; plafond vérifié avant réservation. Aucun ancien pass partiel promu. Tests fixture : correction nécessitant diagnostic après réouverture du Store, citation toujours fausse refusée sans boucle, réponse altérée refusée sans appel ; lot précédemment passé conservé.
