# T7 — Résumé de lancement et diagnostic copiable

## Reprise courante — revue rédactionnelle du 30 septembre 2026

Ce rapport décrit le candidat courant. L’historique intégral est conservé dans
`docs/history/T7-20260930-before-final-review.md` ; ses anciennes conclusions ne
sont pas des observations du candidat actuel. Base Git :
`7110a0b805caf4be1bb09a57b776176b8dc52a68`, avec modifications non commitées.
Production initiale : `a-b7e759aea886550cc74b69fa`. Dernière tentative :
`a-cde55af340e16467351f8d98`, décrite ci-dessous.
La sécurisation de la copie et le test hostile sont des corrections externes
Codex après cette tentative ; ils ne sont pas attribués au worker Claude.
Le contrôle moteur de la dernière tentative est passé le 30 septembre 2026
à 14:48:25 UTC (code 0, environ 21 secondes). Le nouvel examen de ce rapport
exige un reçu renouvelé car son contenu a changé.

### req-22 / critère 3 — textes examinés et jugement motivé

Relecture par Codex des captures réelles issues de la recette navigateur, et du
code des assertions `tests/prelaunch_diagnostic_ui.cjs` (résumé et prévol lignes
50–105, diagnostic lignes 150–175). Cette relecture est une intervention externe
du superviseur, **pas un avis indépendant** et pas une acceptation de la tâche.
Le contenu ci-dessous permet d'examiner la rédaction elle-même, au-delà d'une
simple déclaration « traduction présente ».

| Situation | Texte français observé | Texte anglais observé | Analyse de compréhension |
| --- | --- | --- | --- |
| Récapitulatif | Récapitulatif avant autorisation | Summary before authorization | Le titre situe l'étape avant toute permission de départ. |
| Responsabilités | Responsable ; Exécutants ; Revue et acceptation | Owner ; Workers ; Review and acceptance | Trois fonctions séparées, modèles affichés sous chacune ; la revue n'est pas confondue avec la production. |
| Bornes | Tâches (plan) : 15 max · Appels par rôle (responsable/vérificateur) : 25 max | Tasks (plan): 20 max · Calls per role (owner/reviewer): 40 max | L'unité et la portée sont explicites. Les nombres diffèrent car les captures anglaises reprennent le formulaire initial, avant modification. Ce ne sont pas les plafonds détaillés de tous les exécutants. |
| Avant contrôle | Vérifier avant le lancement | Check before launch | Verbe d'action et moment du contrôle explicites ; ce bouton ne prétend pas lancer un agent. |
| Conditions remplies | Conditions vérifiées. Vous pouvez autoriser cette équipe. | Conditions checked. You can authorize this team. | Deux phrases : résultat du contrôle puis prochaine action permise. Ni « démarré », ni « terminé ». La phrase anglaise est aussi vérifiée exactement par la recette. |
| Aide technique | Voir le détail des vérifications | Pre-launch check details (titre de la modale) | Les états ready/verified et mesures en octets figurent dans une modale, non dans le message principal. |
| Effet de l'aide | Ces informations décrivent les contrôles effectués. Elles ne lancent aucun agent. | This information describes the checks performed. It does not start any agent. | Sépare clairement consultation et exécution. |
| Fermeture | Fermer les détails | Close details | Bouton textuel ; Échap et retour du focus au déclencheur vérifiés. |
| Copie | Copier le diagnostic ; Diagnostic copié. | Copy diagnostic ; Diagnostic copied. | Action et résultat distincts. Le refus de copie est également testé et ne donne pas de faux succès. |
| Échec d'un contrôle | Cause : Un test ou contrôle observable a échoué. Conséquence : Le critère couvert par ce contrôle reste non validé. Action disponible : Examiner la trace, corriger la cause, puis rejouer le même contrôle avec les mêmes options avant toute validation. | Cause: An observable test or check failed. Consequence: The criterion covered by this check remains unvalidated. Available action: Inspect the trace, fix the cause, then rerun the same check with the same options before validation. | Cause, impact et prochaine action sont séparés ; aucune validation n'est annoncée prématurément. |

Captures réellement ouvertes pour cette relecture, dans `test-results/t7-engine/` :
`summary-ready-etat.png`, `summary-ready-sombre.png`, `summary-en-etat.png`,
`summary-en-sombre.png`, `diagnostic-copy-etat.png`, `diagnostic-copy-en-etat.png`,
`preflight-details-fr-sombre.png`, `preflight-details-en-etat.png`.

Résultat local de la relecture : **PASS sur les textes du parcours T7 examinés**.
Motifs : verbes d'action explicites, séparation condition/autorisation/départ,
libellés de rôles distincts, diagnostics cause/conséquence/action, explications
techniques dans la modale ou les traces repliées. Les formulations observables et leur analyse figurent dans la table ci-dessus.

Limites : « Preflight not run » reste un terme technique anglais dans la ligne
synthétique des prérequis, mais le bouton adjacent « Check before launch » et
« Run launch checks before authorizing this team. » donnent l'action concrète.
Les diagnostics détaillés restent longs ; leurs traces sont repliées. Cette revue
ne prétend ni tester la compréhension auprès d'utilisateurs, ni auditer tous les
textes du produit. Les noms de fournisseur et de modèle ne sont pas traduits.
Les fixtures et le presse-papiers simulé ne démontrent pas un fournisseur réel.
L'avis indépendant et la décision finale restent à obtenir.

## Tentative a-cde55af340e16467351f8d98 (départ 3/3, agent auto-5576386e98793d2abaa8, révision 227)

Observation : le reçu `a-b7e759aea886550cc74b69fa.json` (révision 226) indique
`t7-browser-evidence` en échec, code 1, sortie sha256 `bc781fba…`, 73 s
(14:45:01–14:46:14). La sortie brute de cet échec n'est pas conservée dans le
dépôt : **la cause de cet échec n'est pas établie**.

Vérifications de cette tentative (base `7110a0b`, modifications non commitées,
aucun fichier applicatif modifié) :

| Contrôle | Commande / environnement | Résultat |
| --- | --- | --- |
| Entrées identiques au reçu | `sha256sum` des 9 entrées du contrôle | PASS : les 9 empreintes sont identiques à celles du reçu (ex. `web/mission.js` 707c551c…, `tests/prelaunch_diagnostic_ui.cjs` 02ecc54d…). |
| Rejeu 1 | `CHROME_BIN=/usr/bin/google-chrome python3 tools/verification/t7_prelaunch.py all` | PASS, code 0, onze constats, quatorze captures. |
| Rejeu 2 | Même commande avec `env -u CHROME_BIN` (le test retombe sur `/usr/bin/google-chrome`) | PASS, code 0. |
| `git diff --check` | racine du dépôt | PASS, code 0. |

Jugement : mêmes entrées, même commande, deux exécutions réussies ; l'échec du
reçu n'est **pas reproduit**. Hypothèses non vérifiées (aucune reproduction) :
contention de ressources ou de navigateur pendant l'exécution du contrôleur, ou
condition transitoire de l'environnement du contrôleur. Aucune correction de code
n'est justifiée par une défaillance non reproduite ; aucune assertion n'a été
affaiblie. Si le contrôleur échoue de nouveau, conserver sa sortie brute pour
diagnostiquer, puis rejouer avec les mêmes options.

Limites : les deux rejeux sont des exécutions locales du worker, pas le contrôle
du moteur ; le presse-papiers système reste simulé ; fixtures, pas de fournisseur
réel ; ce n'est pas une revue indépendante ni une acceptation. Ce rapport modifié
change l'empreinte d'artefact : un reçu renouvelé est nécessaire.
Prochaine action : le responsable relance le contrôle moteur sur ce candidat.

## REQ-DIAG-01 : copie sans texte libre, contrôle de non-divulgation

Le diagnostic copié est désormais construit exclusivement à partir de formulations
contrôlées. `summary`, `label`, `cause`, `action` et `traces` reçus ne sont jamais
copiés. La catégorie choisit une formulation connue ; une catégorie inconnue
revient à « Cause inconnue ». L'identifiant de tentative et la révision sont
limités à leur format hexadécimal, le compteur à un entier positif. Ce choix
empêche de copier une URL ou un secret arbitraire dans un champ libre, au prix
de laisser les détails précis dans la console locale.

Extrait exact de `web/mission.js`, implémentation complète de la copie :

```javascript
 diagnosticText(diagnostic){
  // Copy only controlled wording and typed identifiers. Free provider text,
  // summaries, causes, actions and traces never cross the clipboard boundary.
  const version=globalThis.RuntimeHealthPanel?.value?.version;
  const revision=String(version?.revision||'');
  const versionLine=version?.available&&/^[a-f0-9]{12,64}$/i.test(revision)?revision.slice(0,12)+(version.modified?'*':''):tr_web_mission_js('état inconnu');
  const attempt=/^a-[a-f0-9]{16,64}$/i.test(String(diagnostic.attempt_id||''))?diagnostic.attempt_id:tr_web_mission_js('état inconnu');
  const count=Number.isSafeInteger(diagnostic.observed_errors)&&diagnostic.observed_errors>=0?diagnostic.observed_errors:0;
  const summary=count===0?'Aucune erreur d’outil structurée n’a été observée.':count===1?'1 erreur d’outil observée pendant cette tentative.':count+' erreurs d’outil observées pendant cette tentative.';
  const wording={
   configuration:['Configuration','La configuration nécessaire au lancement ou à l’outil est absente, invalide ou indisponible.','Corriger la configuration indiquée, vérifier qu’elle est relue, puis demander explicitement une nouvelle tentative.'],
   environment:['Environnement d’exécution','Les traces signalent un refus d’accès ou une ressource indisponible dans l’environnement.','Faire vérifier les droits et l’accès aux ressources sur la machine qui exécute l’agent ; reprendre après correction vérifiée.'],
   check:['Contrôle en échec','Un test ou contrôle observable a échoué.','Examiner la trace, corriger la cause, puis rejouer le même contrôle avec les mêmes options avant toute validation.'],
   tool:['Outil','Un outil appelé pendant la tentative a signalé un échec.','Examiner la trace et les paramètres de l’outil, corriger la cause, puis demander explicitement la reprise.'],
   limit:['Limite atteinte','Limite d’exécution atteinte.','Examiner les erreurs précédentes et les préconditions ; reprendre explicitement sans relever arbitrairement les protections.'],
   unknown:['Cause inconnue','La cause exacte n’est pas disponible dans les événements structurés de cette tentative.','Ouvrir les traces conservées et établir la cause avant de choisir une reprise.']
  };
  const lines=[tr_web_mission_js('Tentative : ')+attempt,tr_web_mission_js('Version : ')+versionLine,tr_web_mission_js('Cause : ')+missionText(summary),''];
  for(const item of diagnostic.items||[]){const [label,cause,action]=wording[Object.hasOwn(wording,item.category)?item.category:'unknown'];lines.push(missionText(label)+' — '+missionText(cause),tr_web_mission_js('Action disponible : ')+missionText(action),'');}
  return lines.join('\n').trim();
 },
```

La recette navigateur injecte des données hostiles à cette frontière. Extrait
exact de `tests/prelaunch_diagnostic_ui.cjs` :

```javascript
  const hostile=await page.evaluate(()=>Mission.diagnosticText({attempt_id:'token=TOPSECRET',observed_errors:3,summary:'Bearer TOPSECRET https://private/session',items:[{category:'limit',label:'TOPSECRET',cause:'password=TOPSECRET',action:'https://private/session',traces:['TOPSECRET']},{category:'__proto__',cause:'TOPSECRET'}]}));
  assert.doesNotMatch(hostile,/TOPSECRET|https?:|password=|Bearer/);
  assert.match(hostile,/Execution limit reached/);assert.match(hostile,/Unknown cause/);
  checks.push('Copie : champs libres hostiles, secret, URL et catégorie inconnue exclus ; texte issu du catalogue seulement');

```

Les tests de copie FR et EN et de refus du presse-papiers restent conservés.
La frontière presse-papiers système est simulée : aucune permission du navigateur
personnel n'est affirmée. Les assertions testent le texte réellement construit
par Mission.diagnosticText ; elles échoueraient si les champs libres réintégraient
la copie. Le programme de contrôle reste `python3 tools/verification/t7_prelaunch.py all`.

## État de livraison

REQ-SUM-01 : recette du récapitulatif ; REQ-DIAG-01 : copie contrôlée et test
hostile explicite ; req-22 : formulations FR/EN fournies avec observations de
rendu et séparation des détails. Candidat soumis à décision indépendante,
pas d’acceptation déclarée. Aucun appel producteur supplémentaire.

L'incident historique fournisseur (429, 127 appels) reste documenté dans l'archive.

## Contrôles du candidat corrigé

`npm test` : code 0. `python3 tools/verification/t7_prelaunch.py all` : code 0, onze constats, quatorze captures, aucune erreur JavaScript. Le refus de copie est activé au clavier après les captures pour éviter le déplacement du bouton lors du rafraîchissement. Le test hostile et son assertion sont inclus dans cette exécution. `git diff --check` : code 0.

Empreintes du candidat actuel :

```json
{
  "locales/en.json": "82a2b31b29ed26157b092919690919af29ea3660b08f8083996e5d40126e2a92",
  "tests/prelaunch_diagnostic_ui.cjs": "02ecc54dcc28c689578d97a8eb074825fdb1119f74b019bb538a8c726b01c4f2",
  "web/i18n-en.js": "bd15d1fc3f47e31bf45f968fad0cd17e11a3e2cc4967e3f744d2afe7fbe08b97",
  "web/mission.js": "707c551caf558a26ddc2fce7d824569fed57a87529c3aff7239bbf06a354865a",
  "web/prepare.html": "301246c624aa5994f9c042e1bffe21821b629263efe3a3b44875139fc876c593",
  "web/prephase-conversion.js": "c009b494ffb4d1e594e54cddade342a10068d20e0cb2d87c41079fa22ac801e6",
  "web/prephase-editor.js": "859de925d60b31720a09d47f9d0ccf05b1b572a8f334d47812f54d4123c942d2",
  "web/prephase.js": "29c4c3428c975162ea1dd2cfed6e2ff532036f0a50cbe290871671d00f62825d"
}
```

## Assertions exécutées du résumé et des textes FR/EN

Extrait exact de `tests/prelaunch_diagnostic_ui.cjs`, entrée liée par empreinte
au contrôle moteur. La navigation lors du changement de langue recrée le
formulaire : les valeurs par défaut 20/40 sont donc testées en anglais, après
les valeurs saisies 15/25 en français. La configuration n’est pas enregistrée
et aucun agent réel n’est lancé dans ces fixtures. Les assertions vérifient
les nombres, les rôles, les conditions manquantes, les textes exacts et le
placement des détails dans la modale.

```javascript
  // REQ-SUM-01 — before any choice: provider, validation and preflight missing.
  let limits=await page.$eval('#summary-limits',e=>e.textContent);
  assert.match(limits,/Tâches \(plan\) : 20 max/);assert.match(limits,/Appels par rôle \(responsable\/vérificateur\) : 40 max/);
  let prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);
  assert.match(prereq,/IA à choisir/);assert.match(prereq,/Validation à choisir/);assert.match(prereq,/Prévol non exécuté/);
  assert.doesNotMatch(prereq,/Dossier de travail à indiquer/,'default workspace "." must not be reported as missing');
  checks.push('Résumé initial : limites par défaut et 3 prérequis manquants affichés (REQ-SUM-01)');

  // Effective limits reflect the live caps before any launch.
  await fill(page,'#organization-tasks','15');await fill(page,'#organization-calls','25');
  await page.waitForFunction(()=>document.getElementById('summary-limits').textContent.includes('15 max'));
  limits=await page.$eval('#summary-limits',e=>e.textContent);assert.match(limits,/Tâches \(plan\) : 15 max · Appels par rôle \(responsable\/vérificateur\) : 25 max/);
  checks.push('Limites effectives suivent les plafonds saisis sans relance manuelle (REQ-SUM-01)');

  await page.select('#organization-provider','recette');await page.select('#organization-validation','human');await page.waitForFunction(()=>document.getElementById('organization-model').textContent.includes('Modèles résolus'));
  assert.match(await page.$eval('#summary-owner',e=>e.textContent),/recette/);assert.match(await page.$eval('#summary-workers',e=>e.textContent),/1/);assert.match(await page.$eval('#summary-review',e=>e.textContent),/recette/);
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.doesNotMatch(prereq,/IA à choisir/);assert.doesNotMatch(prereq,/Validation à choisir/);assert.match(prereq,/Prévol non exécuté/);
  checks.push('Rôles et modèles effectifs résolus ; prérequis restant = prévol uniquement (REQ-SUM-01)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-missing-'+theme+'.png','#organization-summary');

  await page.click('#organization-preflight');await page.waitForFunction(()=>document.getElementById('organization-preflight-status').classList.contains('success'));
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.equal(prereq,'Aucun prérequis manquant — prêt à autoriser.');
  checks.push('Prérequis manquants vides après prévol exploitable (REQ-SUM-01)');
  assert.equal(await page.$eval('#organization-preflight-status',e=>e.textContent),'Conditions vérifiées. Vous pouvez autoriser cette équipe.');
  assert.doesNotMatch(await page.$eval('#organization-preflight-status',e=>e.textContent),/ready|verified|octets/);
  await page.click('#organization-preflight-details');
  await page.waitForSelector('#preflight-details-dialog[open]');
  assert.match(await page.$eval('#preflight-details-content',e=>e.textContent),/ready|compatible/);
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'preflight-details-fr-'+theme+'.png','#preflight-details-dialog');
  // Native Escape returns to the form and restores focus to the trigger.
  await page.keyboard.press('Escape');
  await page.waitForFunction(()=>!document.getElementById('preflight-details-dialog').open&&document.activeElement.id==='organization-preflight-details');
  await page.select('#organization-validation','automatic');
  assert.match(await page.$eval('#summary-prerequisites',e=>e.textContent),/Précisez les arguments/);
  await page.select('#organization-validation','human');
  checks.push('Détails techniques dans une modale, Échap/focus corrects ; contrôles manquants signalés avant autorisation');

  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-ready-'+theme+'.png','#organization-summary');

  // FR/EN — labels translated, no rebuild of the screen.
  await page.keyboard.press('Escape');
  await Promise.all([page.waitForNavigation(),page.select('#swarm-language','en')]);
  await page.waitForFunction(()=>document.documentElement.lang==='en');
  await waitAction('convert');await page.click('#advance');await page.waitForSelector('#conversion-dialog[open]');
  limits=await page.$eval('#summary-limits',e=>e.textContent);assert.match(limits,/Tasks \(plan\): 20 max/);assert.match(limits,/Calls per role \(owner\/reviewer\): 40 max/);
  prereq=await page.$eval('#summary-prerequisites',e=>e.textContent);assert.match(prereq,/AI to choose|Validation to choose|Preflight not run/);
  checks.push('Textes du résumé traduits en anglais sans changer la structure de l’écran (req-22)');
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'summary-en-'+theme+'.png','#organization-summary');
  await page.select('#organization-provider','recette');await page.select('#organization-validation','human');
  await page.waitForFunction(()=>document.getElementById('organization-model').textContent.includes('Models resolved'));
  await page.click('#organization-preflight');
  await page.waitForFunction(()=>document.getElementById('organization-preflight-status').classList.contains('success'));
  assert.equal(await page.$eval('#organization-preflight-status',e=>e.textContent),'Conditions checked. You can authorize this team.');
  await page.click('#organization-preflight-details');
  assert.equal(await page.$eval('#preflight-details-title',e=>e.textContent),'Pre-launch check details');
  assert.match(await page.$eval('#preflight-details-content',e=>e.textContent),/executable and limits valid/);
  assert.doesNotMatch(await page.$eval('#preflight-details-content',e=>e.textContent),/octets disponibles|dossier lisible|sonde configurée/);
  for(const theme of ['etat','sombre'])await captureTheme(page,theme,'preflight-details-en-'+theme+'.png','#preflight-details-dialog');
  await page.click('[data-close="preflight-details-dialog"]');
  await page.waitForFunction(()=>document.activeElement.id==='organization-preflight-details');
  checks.push('English preflight: short explanation, translated details dialog and focus restored');

```
