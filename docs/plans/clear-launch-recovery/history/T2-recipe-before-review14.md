# Complément de recette réel du superviseur — T2

Intervention distincte du worker, sans nouvelle tentative ni validation inventée.

## Cas réellement observés dans le produit

- Tentative T2 arrêtée au plafond de 25 appels : processus interrompu, résultat non validé, rapport peut-être partiel. Le détail montre le motif et demande d'examiner avant reprise ; aucune action d'acceptation n'est disponible tant qu'aucun résultat complet n'est soumis.
- Dépendance T1 périmée/non validée : détail T2 indique le prérequis manquant et désactive Lancer/Relancer avec ce motif. Pendant revalidation T1, il ne prétend pas qu'une relance T2 est autorisée.
- Revalidation T1 : ancien résultat accepted avec fichiers liés modifiés, bouton principal maintenant ouvre une vraie confirmation de nouvelle revue. Même tentative conservée ; statut submitted et ancien avis archivé après confirmation ; appels 7→8 puis 8→9, aucune tentative producteur ajoutée. Nouveaux contrôles exécutés avant décision.
- CLI réel : `bin/swarm --lang fr mission recovery w-115c11e8f802a4f98c3def32 plan-115c11e8f8-T2` et équivalent en anglais montrent critères inchangés, ce qui est gardé/refait, et aucune hausse de budget.

## Navigateur

Captures t2-blockage-fr-light.png, t2-blockage-fr-dark.png, t2-blockage-en-dark.png et t2-blockage-en-light.png sous docs/screenshots/clear-launch-recovery/. Ouverture via le bouton réel « Diagnostiquer et préparer la reprise ». Échap ferme le détail, rend le focus au déclencheur, focusVisible=true. Aucune API directe ni donnée SQLite de mission modifiée.

Limite constatée : les libellés structurels anglais sont traduits ; certains motifs composés issus des traces restent en français (limite d'appels et dépendance). Ne pas affirmer une traduction intégrale de ces traces. Les cas d'organisation absente restent ceux de la recette CLI isolée du worker. Les cas de cooldown restent des tests comportementaux isolés, jamais déclarés quota réel provoqué.

## Consigne de reprise ciblée proposée au worker

Lire uniquement ce complément et T2-handoff.md, puis vérifier les tests de priorité du bandeau / reprise et finaliser le bilan par critère et le manifeste de livraison exigé par le moteur. Réutiliser les preuves de recette superviseur avec attribution. Ne pas refaire l'inventaire du dépôt ni consommer les 25 appels en relecture générale. Pas de changement de critères, budget ou source pendant une revue T1 en cours.

## Contrôles préparés et exécutés par le superviseur

Politique human enregistrée par `swarm validation preview` puis `apply` à la révision 71 ; aucun plafond ni critère modifié. Deux contrôles enregistrés : tests ciblés moteur et cohérence du relevé CLI du cas réel. Exécution directe des mêmes commandes avant reprise : Go PASS 0,207 s, code 0 ; relevé CLI PASS. Ces contrôles personnels ne remplacent pas le reçu du contrôleur sur une nouvelle tentative ni la revue indépendante du résultat.

Pour la seconde tentative autorisée : lire uniquement ce document et T2-handoff.md, vérifier les deux contrôles ciblés, vérifier explicitement les limites de traduction conservées, rédiger le rapport et le livrable avec attribution du superviseur, puis terminer. Ne pas refaire un inventaire global ou provoquer une limite fournisseur ; aucune suppression d’historique.

## Correction vérifiée après le refus 12 — 3 octobre 2026, 16:50

Les captures précédentes ouvraient le détail générique de l'agent. Leur attribution au diagnostic était incorrecte ; le refus 12 a identifié ce manque. La présente recette les remplace par la vraie modale **Reprise de la tâche**, ouverte au clavier depuis **Examiner les tentatives et les refus**. Aucun agent, quota ou état de mission n'a été fabriqué pour cette recette.

Un défaut moteur a été reproduit sur cette même mission : `resultPresentation` ne présentait les refus que pour les copies Git gérées. Dans une mission ordinaire, un producteur terminé devenait « rapport détecté, à soumettre » malgré un avis `changes_requested` actuel. `currentReviewPresentation` présente maintenant ce refus avec son motif, le rapport effectivement soumis, le responsable et la reprise permise. Les avis d'une autre tentative ou d'un autre producteur ne sont pas attribués à la tentative courante. Un rapport modifié est signalé périmé ; aucun avis favorable n'est une acceptation.

| Cas réel | Cause | Qui agit | Action réellement essayée | Résultat |
| --- | --- | --- | --- | --- |
| T2, refus indépendant 12 et plafond 2/2 consommé | Preuves navigateur incorrectes et critères non démontrés, détail du refus conservé | Vous | Entrée sur « Examiner les tentatives et les refus » ouvre la modale et les preuves ; aucun agent ne démarre | PASS réel, FR clair/sombre |
| T2, preuves liées modifiées après ce refus | Avis périmé ; le contrat ou ses entrées ont changé depuis la revue | You | « Inspect attempts and rejections » ouvre la modale avec cause, Next step et Who acts ; aucun budget n'est relevé | PASS réel, EN clair/sombre |
| Organisation absente dans la racine CLI isolée du worker | Organisation autonome non configurée | Vous | Préparer l'organisation ; aucune exécution prétendue | PASS CLI réel, tentative 2, preuve distincte |

Les quatre captures t2-blockage-* montrent désormais ces deux cas réels de refus/péremption, et non un quota fournisseur. Elles montrent le triplet cause / acteur / prochaine étape. Entrée ouvre la modale ; Échap la ferme ; focus rendu au bouton et `:focus-visible=true` observés dans les combinaisons contrôlées. Les surfaces clair/sombre et leur hiérarchie ont été inspectées visuellement. Les avis rédigés en français restent en français dans l'interface anglaise ; les libellés et consignes du moteur sont traduits. Aucun rate-limit fournisseur réel n'a été déclenché et il n'est pas déclaré testé.

Les critères de T2 demandent des cas de blocage réels et le triplet pour chaque cas testé ; ils ne demandent pas de provoquer toutes les catégories possibles de panne. La matrice actuelle couvre le refus indépendant réel, la péremption réelle et l'absence réelle d'organisation. Les anciennes lignes PARTIAL sont conservées comme historique des deux tentatives, remplacées pour la couverture actuelle par cette recette datée. La recette des quotas demeure comportementale et séparée.
