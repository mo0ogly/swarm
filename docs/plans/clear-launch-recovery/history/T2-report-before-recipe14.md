# T2 — REQ-QW2 : blocage expliqué (web + CLI)

Mission w-115c11e8f802a4f98c3def32 · tâche plan-115c11e8f8-T2 · agent 537b1d6b-7c30-4e7f-878a-3c832554c719 · tentative a-4fea4cb374a0cf60b5cc2680 · départ 2/2 (dernière tentative autorisée, max_attempts=2) · révision du travail 83.
Tentative précédente `a-1838917a21fb55cd54481c1c` (départ 1/2) : interrupted au plafond de 25 appels, mais avait écrit `T2-handoff.md` et ce rapport avant interruption.
Racine de travail : `/home/fpizzi/workspace/swarm-action-skills` (worktree, branche `codex/clear-launch-recovery`).
Base du candidat : `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, arbre non commité `dirty=true` (même candidat que T1 et que la tentative 1 ; aucun fichier de code modifié par T2, tentatives 1 ou 2).

Reprise bornée : lecture limitée à `docs/plans/clear-launch-recovery/T2-supervisor-recipe.md` et `T2-handoff.md` (aucun nouvel inventaire global, aucune relecture T0/T1). Les deux contrôles de la politique enregistrée (`swarm validation preview`/`apply`, révision 71) ont été rejoués sur le candidat courant.

## Critères — état final (tentative 2)

1. **Message de blocage affiche cause, acteur responsable et action directe réellement autorisée pour chaque cas testé.**
   **PASS.** Rejoué cette tentative : tests ciblés moteur (`go test ./... -run 'TestMissionProviderCooldown|TestProviderCooldown|TestAttemptDiagnostic|TestTaskUnderstanding|TestMissionStatus.*Diagnostic|TestMissionStatus.*Understanding' -v` → 31 tests PASS, code 0, 2,937 s) et relevé CLI réel sur racine isolée (`/tmp/swarm-t2-qw2-v2-*`, travail `w-9812b5ebba819a4e3828d127`) — cause « Organisation autonome non configurée », acteur « Vous », action « Préparer une mission hiérarchique... », identique au texte de la tentative 1, exit 0 sur 4 commandes. Complément du superviseur (hors périmètre worker, évidence attribuée et non revendiquée par ce rapport comme produite par ce worker) : recette navigateur réelle sur le vrai bouton « Diagnostiquer et préparer la reprise » — captures `t2-blockage-{fr,en}-{light,dark}.png` confirmées présentes sous `docs/screenshots/clear-launch-recovery/`, Échap ferme et rend le focus (`focusVisible=true`) ; trois cas de blocage supplémentaires observés par le superviseur dans le produit réel (plafond 25 appels, dépendance T1 périmée, revalidation T1) — non relus en détail par ce worker (hors périmètre de lecture autorisé pour cette tentative). **NOT TESTED, inchangé** : blocage fournisseur en vrai rate-limit et plafond de tentatives via agent fournisseur réel (déclencher ces conditions exigerait un abus de quota réel ou l'épuisement réel d'un agent, explicitement hors périmètre/éthique).
2. **Aucun contournement de limite proposé ou appliqué.** **PASS**, inchangé. Aucun texte lu (tentative 1, tentative 2, ni recette superviseur) ne propose de lever ou augmenter une limite ; plusieurs l'interdisent explicitement. Aucune limite modifiée par cette tentative (relecture bornée + rejeu de deux contrôles déjà enregistrés + une racine `/tmp` jetable supplémentaire).
3. **Recette réalisée sur des cas de blocage réels, sans doublure remplaçant le comportement vérifié.** **PARTIAL, amélioré vs tentative 1.** Le cas « organisation non configurée » est maintenant couvert en CLI réel (worker, tentatives 1 et 2) **et** navigateur réel (superviseur, 4 captures FR/EN clair/sombre). Cooldown fournisseur réel et plafond de tentatives réel restent couverts seulement par la suite de tests comportementale existante (code moteur réel, pas une doublure du rendu) — construire ces préconditions en direct reste hors budget/éthique de cette tâche. Limite de traduction explicitement maintenue : libellés structurels traduits en anglais, certains motifs composés issus des traces (limite d'appels, dépendance) restent en français — ne pas déclarer de traduction intégrale.

## Preuve détaillée

Voir `docs/plans/clear-launch-recovery/T2-handoff.md` (section « Mise à jour — tentative 2 » pour le rejeu des contrôles et l'intégration de la recette superviseur).

## Limites

Cooldown fournisseur réel et épuisement de tentatives via agent réel non rejoués en direct (hors périmètre/éthique) — couverts par tests comportementaux existants uniquement. Les trois cas supplémentaires rapportés par le superviseur (plafond 25 appels, dépendance T1 périmée, revalidation T1) sont cités par attribution ; ce worker n'a pas relu leurs preuves brutes (captures, `T2-observation.json`) car hors périmètre de lecture imposé pour cette tentative — à confirmer indépendamment si requis avant acceptation. Tentative 2/2 : dernière tentative autorisée pour T2 ; toute lacune restante doit être transmise au planificateur, pas relancée à l'identique.

## Prochaine action

Revue indépendante sur le même SHA candidat (`1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, dirty=true). Ce rapport ne vaut ni acceptation ni accusé de réception.

## État courant après le refus 12 — complément du superviseur, 3 octobre 2026

Les paragraphes historiques de cette tentative sont conservés. Leur attribution des captures au diagnostic était incorrecte, et la phrase « mêmes contrôles de politique » confondait deux commandes différentes. Le superviseur a ensuite reproduit et corrigé un vrai défaut du moteur : les refus indépendants des missions ordinaires disparaissaient derrière « rapport à soumettre ». Le code courant présente le refus, son acteur et sa reprise ; aucun nouvel agent producteur n'a été lancé.

Le résultat courant est documenté dans **docs/plans/clear-launch-recovery/T2-handoff.md**, section « État courant corrigé », et **T2-supervisor-recipe.md**, section « Correction vérifiée après le refus 12 ». Les captures t2-blockage-* actuelles montrent la vraie modale Reprise de la tâche avec cause, prochaine étape et acteur. FR clair/sombre : refus réel 12 et plafond 2/2 ; EN clair/sombre : preuves réellement périmées après leur modification. Entrée ouvre, Échap ferme et rend un focus visible au déclencheur. Le CLI public confirme `review_blocked`, rapport soumis, acteur Vous et motif du refus. Le cas réel d'organisation absente reste celui exécuté par le worker. Pour **ces cas effectivement testés**, req-7, req-8 et req-9 sont PASS de recette ; une nouvelle revue indépendante et les contrôles restent requis. Cooldown fournisseur réel reste NOT TESTED, et n'est pas une preuve revendiquée. Les critères n'imposent pas d'épuiser volontairement un quota fournisseur.

Base Git 1d9570bd4ef617130c6be96b7ec88844fdbcd00e + arbre dirty, différent de la première tentative : la nouvelle source result_presentation.go et son test sont identifiés dans les entrées des contrôles, pas par la base seule. Interventions du superviseur explicitement distinctes du worker. Aucune acceptation ni autonomie complète revendiquée.
