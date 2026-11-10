# FPA-03 — consignes attribuées, livraison locale du 25 octobre 2026

Statut : **livré localement et vérifié**, v0.1.1-dev.60. APEX, contributions et
revues GLM via HTTP SKYNET ; intégration et vérification par Codex. Aucun commit,
push ou publication. Cette tranche ne clôt pas la migration complète.

Le planificateur peut ajouter un apprentissage à une tâche future de son périmètre.
L’origine, la base, la décision et les empreintes sont conservées. L’utilisateur
peut rejeter cet ajout avant transmission, depuis Conduite, la CLI ou l’API. Le
rejet reste historique et est remis au planificateur suivant. Réservation active,
révision périmée et tentative ayant reçu l’ajout empêchent sa réécriture. Critères,
validation, budgets et droits sont conservés. Le plafond d’instructions Admin
s’applique aux nouveaux ajouts.

Preuves : 5 tests Rust nommés ; 67 assertions publiques CLI/API/parité/contexte ;
35 assertions navigateur FR/EN, deux thèmes, clavier, focus, mobile et reprise
après réponse perdue. Trois planificateurs locaux factices ont reçu leurs vrais
prompts puis acquitté le rejet. L’oracle Go compare tâches, notifications et huit
refus ; 2280 sources/outils/embeds effectifs sont conservés. Aucun modèle payant,
raisonnement autonome ou identité humaine n’est démontré par ces fixtures.

Installation : empreinte du processus vérifiée, 45 ressources statiques comparées
au catalogue et au profil livré ; fenêtre Version FR/EN, notes, commit réel,
modifications non commitées et Escape/focus : 14 assertions. Deux missions
existantes restent suspendues. Empreinte : `81eafe6f9925cfb77d7dc23497b10ae027e239d3e469147a643bdf0a41b93587`.

La revue explicite couvre les 15 entrées effectives modifiées. Les revues GLM sont
statiques et bornées ; aucune acceptation indépendante par le moteur n’est
fabriquée. La première déclaration de revue a refusé des références absolues ;
l’enchaînement shell a laissé le redémarrage précéder ce contrôle formel. Refus
conservé, déclaration v2 corrigée et validée avant conclusion de livraison.

RETEX : examiner tous les aiguillages et le prompt réellement remis pour chaque
opération portée ; relire les producteurs des fixtures, puis vérifier un témoin
valide avant les refus. La recette permanente couvre maintenant le feedback
persistant après actualisation et le plafond Admin omis du premier candidat.
Apprentissage transmis par le plan et par le préfixe Rust reconstruit, sans relever
les limites des corps de rôle ni réécrire les contextes déjà figés.

La mémoire permanente FPA-01/02 et les autres écarts restent ouverts. AC-02 et les
droits mémoire futurs ne sont pas démarrés par cette tranche. Les commandes
opérateur modifient des consignes autorisées ; elles ne rendent pas toutes les
consignes immuables et ne prouvent pas l’immunité aux injections.

- [Revue et diagnostic détaillés](evidence/20261025-instruction-learning-371v38up/REVIEW-RACE.md)
- [Contrôle de couverture de la revue](evidence/20261025-instruction-learning-371v38up/source-review-race-result.json)
- [Identité et ressources installées](evidence/20261025-instruction-learning-371v38up/installed-summary-race.json)
- [Recette permanente](../../../tools/verification/recipes/instruction-learning-rust.json)

Reprise : la course d’actualisation du même Work est reproduite sur dev59 et corrigée en dev60. La recette finale passe sans intervention ; le collecteur continu est vérifié au-delà de la capacité mesurée du flux. Les exécutions interrompues ou aidées manuellement restent historiques. Le mode observateur synchrone conserve son propre témoin public PASS.
