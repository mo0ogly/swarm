# T2 — correction et recette par le superviseur Codex, 4 octobre 2026

Mission w-01567e073c1ed2f3d4c71c9e, HEAD cc3069dc7bb61b90168d21f945cb2eb5e27578ed, candidat non commité : T2-candidate.json (SHA256 par fichier) et T2-candidate.diff. Premier producteur auto-ed82e20e57f3200af263, tentative a-b45d8905fcf816f645fa0ec2 interrompue après 60/60 outils ; aucun achèvement prétendu. Original conservé dans T2-worker-original.md. Les contrôles et compléments ci-dessous ont été réalisés par Codex, pas par ce producteur.

## Correction bornée

- Budget USD : fonction openBudgetEdit commune à l’onglet et aux cartes. « Régler cette limite » visible dans le pilotage principal dès une décision budget ouverte ; conserve la mission sélectionnée. La première version avait placé le bouton dans une section fermée ; la recette l’a détecté et le superviseur a corrigé ce défaut. La clé de rafraîchissement de Mission inclut maintenant les décisions afin de faire apparaître/disparaître ce bouton après une escalade.
- Planification/revue : même bouton lorsque activations, décisions ou appels reviewer sont épuisés. Quotas.open affiche consommé/plafond et restant séparés. Aperçu puis autorisation motivée via les endpoints existants ; aucun compteur remis à zéro.
- Outils : « Voir les limites de cette tâche » depuis un diagnostic de limite ouvre Administration à scope=task, mission et identifiant exacts, avec focus sur Charger cette portée. La dernière tentative affiche ses appels/plafond figés/restant lorsque rapportés, sinon Non rapporté. Le plafond du plan est affiché séparément avec avertissement : changer une limite d’exécution ne change ni le plan ni une tentative lancée. Ce bouton ne promet pas une reprise automatique et n’augmente aucun plafond.
- Aucune modification moteur Go ; garde de révision, historique et CLI budget/quotas/run-limits réutilisés. FR/EN, aucun nouveau style/couleur. Une reprise reste une décision explicite avec précondition corrigée.

## Contrôles réellement exécutés

| Commande exacte | Sortie/code | Effet vérifié |
| --- | --- | --- |
| go build -o /tmp/swarm-qw5-candidate . | 0 | binaire isolé embarquant ce candidat |
| go vet ./... | 0 | analyse Go ; aucun fichier Go modifié |
| npm test | 0, PASS | contrats frontend existants et catalogue i18n |
| git diff --check | 0 | différences sans erreurs d’espaces |
| node tests/qw5_resolve_blocking_limit_ui.cjs /tmp/swarm-qw5-candidate /tmp/swarm-qw5-budget-entry | 0, PASS budget UI | entrée budget visible au clavier, bonne mission, aperçu sans écriture, application publique, conflit de révision rejeté, Échap restitue le focus |
| node tests/qw5_quota_entry_ui.cjs /tmp/swarm-qw5-candidate /tmp/swarm-qw5-quota-entry | 0, PASS quotas UI | entrée quota au clavier, restant zéro visible, aperçu sans mutation, consommations conservées et vérification CLI, conflit rejeté ; ouverture de la portée tâche exacte |

Les deux recettes exécutent les parcours FR et EN, thèmes etat et sombre, capturent les modales et enregistrent aucune exception de page. Receipts T2-budget-result.json et T2-quotas-result.json ; captures T2-budget-fr-etat.png, T2-budget-fr-sombre.png, T2-budget-en-etat.png, T2-budget-en-sombre.png et équivalents T2-quotas-quotas-*.png.

Les budgets/quotas sont réellement modifiés puis relus par CLI dans des racines temporaires isolées. Les cartes de blocage et compteur épuisé sont des fixtures DOM explicites : elles fournissent l’état, elles ne remplacent pas le rendu ni la mutation sous contrôle. Ce ne sont pas des preuves de lancement autonome d’un fournisseur. Le routage outils fournit un diagnostic de limite fictif puis utilise le vrai chargement public de portée ; pas de mutation de limites outils ni de relance dans cette recette.

## Critères et limites

req-4 : cibles budget mission, quotas planning/revue et portée tâche distinctes ; les unités et le plafond du plan ne sont pas fusionnés. req-5 : confirmation existante réutilisée, comptes conservés et prévisualisation contrôlée avant écriture, conflits rejetés. req-6 : FR/EN, deux thèmes, clavier/focus et lecture CLI vérifiés par recettes isolées. Tests Go complets non relancés : aucun Go modifié. Les recettes enregistrent les exceptions JavaScript, pas un inventaire complet de console/réseau ; cette limite reste à compléter dans la recette globale T6. Contrôle visuel des captures par superviseur distinct de l’agent. Pas de push/merge ni de budget augmenté.

La seconde tentative prévue sert uniquement à examiner ces preuves et remettre un rapport complet et attribué. Elle ne doit pas explorer de nouveau le dépôt ni changer le code. Revue indépendante puis gate et acceptation restent à obtenir ; aucun PASS reviewer ni acceptation prétendu dans ce document.

## Reçus fournis au contexte de revue (contenu, pas liens seuls)

### T2-budget-result.json
```json
{
  "status": "PASS",
  "checks": [
    "budget escalation keyboard entry targets selected mission",
    "preview is read-only",
    "web and CLI parity",
    "stale modal rejected",
    "Escape restores focus",
    "French and English",
    "both themes"
  ],
  "root": "/tmp/swarm-budget-ui-C48VY2"
}```

### T2-quotas-result.json
```json
{
  "status": "PASS",
  "checks": [
    "quota preview read-only",
    "web CLI parity",
    "counts preserved",
    "missing reviewer not invented",
    "stale form refused",
    "keyboard focus",
    "FR EN both themes"
  ],
  "root": "/tmp/swarm-budget-ui-E4NGQS"
}```

### T2-candidate.json
```json
{
  "head": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed",
  "files": {
    "web/cockpit.js": "db7437d5b85fca4b554f9782c9f380ede2ed575a6de3e4142caa32997c6fc18d",
    "web/conduite.js": "f70db504c2c1c4cc215767048b084ea81e7a8b6acf426f20f2c8b654d04ab484",
    "web/mission.js": "488512223cc9de8f407092d91b95b33fb50a4e51ba9536329a4307189bfbdead",
    "web/planning.js": "1908692110c74d592d91828d1398b6821ebe8e386dadacd9f3a65bb0a70b9c32",
    "web/quotas.js": "b24ab62489de35504930ed54bb9ebd790b67c946d41d27dd497eaf2692cfd950",
    "web/admin.js": "c1f4137f24ecd292335aaf7da76b83d3c3b0c9dcc9f3fa05a0224cc7d70c3e33",
    "web/i18n-en.js": "8c7c012058476fcfb8e2b8f0f84481d3dd623da059ab39b8583d29f008b6f6e9",
    "locales/en.json": "3001b57c4ec286d4d69dfb7d170cfff3c5287b5a911eaf0f35a494bf59bbaac7",
    "tests/qw5_resolve_blocking_limit_ui.cjs": "7d6a2595541da5866fbd508d59e72a332c8ecbbb916021fd824bb2fd853906d5",
    "tests/qw5_quota_entry_ui.cjs": "fbd5e3f855613ce5a23653fbf5ba24c02105f9cac6f72b2548a8a6245c20659f"
  },
  "checked_at": "2026-10-04T08:05:24.851846+00:00"
}
```
