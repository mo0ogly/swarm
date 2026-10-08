# T4 — reprise ciblée : recette du superviseur

Base Git `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, candidat non commité.
Le premier agent `auto-f6aa5085c5f63a1e7fad`, tentative `a-4060d356d9f554c29087fad8`, a commencé le prototype moteur puis a été interrompu après 25 outils (7 lectures, 3 écritures, 15 non classés). Le superviseur a terminé le moteur, CLI, interface, traductions et tests. Les captures sont celles du superviseur ; aucune attribution à l’exécutant de vérification.

## Contrat

Le CLI `mission recovery TRAVAIL TACHE` et le bouton « Avant une relance » affichent les changements durables depuis le refus de cette tâche, les empreintes des seules preuves liées à son avis et les critères restant à vérifier. Maximum 64 fichiers, 8 Mio par fichier et 32 Mio au total ; hors limites, absence ou chemin non autorisé = inconnu. Aucun inventaire global. Une preuve inchangée est réutilisable comme entrée, jamais comme acceptation. Une preuve modifiée périme l’ancien contrôle. L’aperçu reste en lecture seule et n’augmente aucune limite.

## Vérifications exécutées

- Tests Go ciblés récupération/politique de validation/concurrence : PASS 2,535 s.
- Même périmètre avec détecteur de courses : PASS 15,991 s.
- `npm test` : PASS, y compris rendus FR/EN et parité des traductions.
- `go vet ./...` et `git diff --check` : PASS.
- Test isolé de réservation SQLite : échec reproduit avant correctif (écriture concurrente entrée pendant contrôle), PASS après ajout de `validation-policy.change` aux opérations réservant le rédacteur avant leur garde. Aucun changement de base de mission par accès SQLite direct.
- Le premier agent masquait l’échec de compilation par `go build ... | head`. Le superviseur a reproduit puis corrigé l’argument Task/*Task ; compilation directe actuelle réussie.
- Suite Go complète antérieure : PASS 1483,907 s. Suite candidat T4 : première commande malformée `-timeout40m`, échec immédiat ; corrigée en `-timeout 40m`, en cours, aucun PASS prétendu.

## Parcours réel

Observation CLI publique jointe `T4-recovery-observation.json` : refus T1 ancré r131, puis contrôles r135, avis r137 et acceptation r139. Les modifications courantes de `locales/en.json` et `web/i18n-en.js` sont signalées comme périmées, avec les trois critères restant à vérifier. Les autres preuves inchangées sont regroupées et repliées. Ce n’est pas une fixture ni une approbation.

Navigateur réel sur port 18792 construit depuis ce candidat : FR et EN, thèmes État et sombre. Captures `t4-recovery-{fr,en}-{light,dark}.png` ; ouverture par Entrée, fermeture Échap, retour du focus au déclencheur confirmés dans les quatre variantes ; aucune erreur console relevée. Les contenus rédigés par l’utilisateur restent français en interface anglaise. Le texte et les composants de l’application sont traduits. Capture complémentaire `t4-recovery-changed-fr-light.png`, relevé DOM des preuves modifiées. Les décisions de revalidation se font ensuite par les actions normales, sur les mêmes producteurs terminés ; aucun nouveau producteur T1/T2/T3.

## Limites

L’historique consulté est borné à 500 événements ; une absence au-delà de cette fenêtre est explicitement inconnue. La comparaison d’empreinte ne juge pas la qualité d’un fichier. Les résultats de revue indépendants et les contrôles restent obligatoires. La fin de mission reste à obtenir après T4 et T5.

## Confirmation sur la mission active

Deux applications publiques de politique de validation T4 réussies pendant que le serveur et son suivi tournaient : pas d’arrêt requis après le correctif SQLite. Sur T4 interrompue sans revue, le CLI/web indiquent explicitement « Aucune preuve liée à un avis disponible ; réutilisation non démontrée », conservent les trois critères et montrent la consigne corrigée r155 depuis son arrêt. Capture complémentaire `t4-recovery-no-review-fr-light.png` ; Entrée ouvre, Échap ferme, focus retrouvé sur recovery-preview-plan-115c11e8f8-T4. Le contenu de l’aperçu ne lance aucun agent.

## Reprise effectivement obtenue après le complément T4

Après le refus historique T1 et les modifications bornées, bouton web « Reprendre la vérification » sur le même producteur, contrôles moteur réellement exécutés, revue indépendante favorable, nouvelle gate de six contrôles, puis « Confirmer la validation » dans le cockpit, sans dérogation. T1 acceptée r165, T2 r170 et T3 r175. La lecture CLI publique `work show` confirme à r180 quatre tâches actuellement validées, avec les preuves courantes suivantes :

```json
[
  {
    "task": "plan-115c11e8f8-T1",
    "status": "accepted",
    "fresh": true,
    "review": "review-b97c5870b8ade635d4df372d",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T1/a-62b5c700c364e7236430a9ca-receipt-7b0dcd5b01e19fb7b0407205.json"
  },
  {
    "task": "plan-115c11e8f8-T2",
    "status": "accepted",
    "fresh": true,
    "review": "review-44c35bed9cfee9044da62bd0",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T2/a-4fea4cb374a0cf60b5cc2680-receipt-50e742fbecb2419db9115501.json"
  },
  {
    "task": "plan-115c11e8f8-T3",
    "status": "accepted",
    "fresh": true,
    "review": "review-70477a6401487b971a1116bd",
    "receipt": ".swarm/validation/w-115c11e8f802a4f98c3def32/plan-115c11e8f8-T3/a-9c26df876eb8691a342f2e00-receipt-f9de2da8be77d4d4aee83139.json"
  }
]
```

Puis bouton « Relancer cette tâche » T4 : consigne de vérification bornée contrôlée dans le formulaire, fournisseur et workspace conservés, deuxième et dernière tentative lancée `3500cb7c-3823-48cf-a454-295a032b309e` / `a-9b4502ef490c0a9b2227676b`. État running confirmé par CLI public et cockpit. Tests ciblés PASS, rapport en rédaction. Cette nouvelle tentative ne change pas l’application ; sa propre revue et acceptation restent à obtenir. La recette réelle du superviseur prouve ce parcours web/CLI, elle n’est pas attribuée à l’exécutant.

## Complément exigé par le refus indépendant — séquence, pas écran statique

L’avis T4 a reconnu les critères 1/2 et demandé la preuve du parcours réel pour le critère 3. Événement de refus r185, observé dans le cockpit à r187 (la consigne abrège « refus r187 » : il s’agit de la révision d’observation, pas de l’ancre du moteur). Aucun défaut de code établi par cet avis.

Séquence de captures réelles, prises pendant les actions du superviseur :
1. `t4-recovery-no-review-fr-light.png` : première tentative interrompue sans revue, nouvelle consigne bornée r155 affichée, preuves inconnues conservées.
2. `t4-verification-running.png` : après clic réel « Relancer cette tâche » puis « Confirmer », quatre acceptations courantes et un agent T4 démarré. CLI agent show confirme 3500cb7c-3823-48cf-a454-295a032b309e, tentative a-9b4502ef490c0a9b2227676b, terminée ensuite en 12 outils.
3. `t4-refusal-current.png` : avis indépendant réel, tests réellement exécutés code 0, deux critères avec preuve présente et troisième preuve insuffisante, pas d’acceptation.
4. `t4-correction-after-refusal.png` : après task update public r188, bouton « Avant une relance » ouvert par Entrée. L’interface montre r188 depuis le refus, les preuves inchangées repliées et UN critère restant : le parcours réel. Échap rend le focus au déclencheur. Aucune action de production dans l’aperçu.

JSON CLI publics AVANT/APRÈS : `T4-recovery-before-correction.json`, `T4-recovery-after-correction.json`. Dans le second : since_refusal.from_revision=185, to_revision=188, événement decision r188 de cette seule tâche, un seul critère restant et preuves inchangées. Cette observation n’est pas un JSON inventé ni une fixture.

Correction bornée de la preuve, sans toucher au code, sans nouvelle production et sans relever les budgets : le présent complément, les étapes capturées et un troisième contrôle moteur qui appelle réellement les commandes CLI publiques mission recovery et work show sur l’état persistant. Ce contrôle examine l’ancre du refus, la correction r188, l’état du producteur terminé et les deux tentatives déjà consommées ; il ne substitue pas les JSON archivés à une lecture live. Les catégories du bilan final seront relues séparément. La nouvelle revue apprécie ces preuves supplémentaires sans demander une troisième tentative de production ni effacer le refus.
