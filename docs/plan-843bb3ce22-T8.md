# T8 — Installation illustrée : candidat courant

Travail `w-843bb3ce22cff2965c5e77b6`, tâche `plan-843bb3ce22-T8`, tentative
`a-0978abf2bb9d55b0bb8c9fa6`, producteur Claude `auto-a03203a46a39674502a6`.
Base Git `7110a0b805caf4be1bb09a57b776176b8dc52a68`, modifications non commitées.
Les fichiers applicatifs des lots précédents sont préservés. La production du
worker et la revue externe des captures sont attribuées séparément ci-dessous.
Livrable : `docs/T8-install-illustre.md`.

## Complément de vérification externe — 30 septembre 2026

Le rapport du producteur avant ce complément est conservé intégralement dans
`docs/history/plan-843bb3ce22-T8-before-supervisor-review.md`. Ce complément
est une revue par Codex, distincte de la session Claude productrice. Elle
ne vaut pas l’acceptation moteur ni l’avis du vérificateur enregistré.

Résultats courants : req-23 PASS sur les légendes bilingues des trois états ;
la capture de lancement est française, explicitement annoncée comme telle
dans le guide anglais. Aucun nouveau lancement n’a été fabriqué. req-24 PASS :
chaque ligne initiale est conservée dans l’ordre, diff +37/0 dans chaque guide.
req-25 PASS : les six erreurs et résolutions figurent dans le diff ci-dessous.
req-26 PASS sur l’observation des 17 images réellement référencées : toutes ont
été ouvertes et examinées, aucune clé, jeton d’accès ou URL de session visible.
Cette revue visuelle n’est pas une garantie sur des images futures.

Contrôle local : `python3 tools/verification/t8_docs.py`, code 0. Il lie chaque
image à l’attestation visuelle par SHA-256, conserve les lignes initiales et
rejoue l’enregistrement FR/EN dans une racine isolée. Il ne reconnaît pas les
secrets dans les pixels : ce jugement reste la revue visuelle documentée.

### Diff exact proposé aux deux guides

```diff
diff --git a/INSTALL.md b/INSTALL.md
index 5b45366..cbd87f9 100644
--- a/INSTALL.md
+++ b/INSTALL.md
@@ -81,6 +81,10 @@ Dans **IA et connexions**, ajoutez le nom, l’adresse du service, l’identifia
 
 Le protocole attendu est compatible avec `POST /chat/completions`. Une connexion API peut servir à préparer, planifier ou examiner un rapport. Elle ne donne pas automatiquement des outils d’accès aux fichiers.
 
+![Formulaire Ajouter une IA rempli, avant enregistrement](docs/screenshots/connexion.png)
+
+*Capture réelle du formulaire rempli (fournisseur local d’exemple, champ de clé vide) : ni testé ni enregistré, « Connexion non testée dans ce formulaire » est affiché. Aucune clé n’apparaît. Cliquez **Tester la connexion** puis **Enregistrer la connexion** ; l’enregistrement ne prouve pas que le service répond.*
+
 Avec le réseau hôte Linux, une API locale peut être joignable par exemple à `http://127.0.0.1:11434/v1`, si vous avez effectivement lancé un service compatible à cette adresse.
 
 ### Agents capables de modifier le projet
@@ -240,6 +244,14 @@ de certaines migrations ne remplacent pas cette sauvegarde complète.
 
 Capture réelle du parcours français : mission Administration lancée depuis les écrans, le 29 septembre 2026. Le lancement ne prouve pas la réussite des huit tâches ; les résultats restent à examiner.
 
+### Trois états à ne pas confondre
+
+| État | Ce que vous voyez | Ce que cela prouve | Capture |
+| --- | --- | --- | --- |
+| Formulaire rempli | Valeurs saisies, aperçu « valeurs actuelles » avant confirmation | Rien n’est écrit : la révision n’a pas changé | `admin-fr-etat.png`, `admin-fr-sombre.png` |
+| Configuration enregistrée | Révision 1 dans la portée et dans l’historique, auteur et motif | Les valeurs seront utilisées par les **prochains départs** | `admin-saved-fr-etat.png`, `admin-saved-fr-sombre.png` |
+| Lancement effectif | Tâche en cours, tentative active dans le pilotage | Un agent a démarré ; ni résultat, ni acceptation | `mission-lancee-fr.png` |
+
 **Point de vigilance :** le parcours actuel comporte une autorisation de l’équipe, puis un lancement dans le pilotage. Si aucune tâche ne démarre, consultez le motif affiché avant toute nouvelle tentative.
 
 
@@ -259,6 +271,12 @@ Ouvrez **Administration** dans le cockpit (passez en mode expert si cette rubriq
 
 ![Même formulaire en thème sombre](docs/screenshots/installation/admin-fr-sombre.png)
 
+![Administration en français, thème clair : configuration enregistrée en révision 1](docs/screenshots/installation/admin-saved-fr-etat.png)
+
+*Capture réelle, racine temporaire isolée, 30 septembre 2026 : après confirmation, la révision 1 est lue par le CLI (`run-limits show`) et apparaît dans l’historique. C’est une configuration enregistrée, pas un lancement. Le badge de version du serveur est visible dans la colonne de gauche.*
+
+![Configuration enregistrée, thème sombre](docs/screenshots/installation/admin-saved-fr-sombre.png)
+
 Le **mode observation**, lorsqu’il est explicitement autorisé pour une mission, conserve les compteurs mais retire les coupures d’exécution couvertes par ce mode. Il ne supprime ni les quotas du fournisseur, ni la revue, ni les conditions d’acceptation. Il n’est pas un moyen de contourner une erreur fournisseur 429. La rubrique **Budgets et coûts IA** distingue les coûts rapportés des coûts inconnus ; un coût inconnu ne vaut pas zéro.
 
 ### Retrouver les mêmes réglages dans le CLI
@@ -273,6 +291,10 @@ swarm --root /chemin/du/projet run-limits effective ID_MISSION worker ID_TACHE
 
 Le tiret représente une valeur vide. Les commandes `apply` et `rollback` utilisent un fichier JSON, une révision attendue et un identifiant d’événement ; voir le [guide d’utilisation](GUIDE-UTILISATEUR.md) pour le parcours général. Le CLI et le web appliquent les mêmes règles du moteur.
 
+## Version du serveur et liens vers une mission supprimée
+
+Le badge **Version** du rail gauche affiche la révision de build du serveur. Elle est comparée au dépôt Git local, sans requête réseau : si la révision est absente, l’état « inconnu » est affiché ; si la comparaison est impossible, elle est signalée comme dégradée. Un `*` indique un arbre de travail modifié au moment de la compilation. Un lien vers une mission supprimée affiche un message clair et l’action **Choisir une mission**, qui ouvre la gestion des missions.
+
 ## Lire le récapitulatif et partager un diagnostic
 
 Avant autorisation, le récapitulatif montre le responsable, les exécutants, le vérificateur et leurs modèles résolus. Les plafonds du plan et des rôles de planification/revue sont affichés séparément des réglages d’exécution de l’Administration. Les prérequis manquants expliquent ce qu’il faut compléter.
@@ -306,3 +328,18 @@ Les captures d’Administration proviennent de `tests/run_limits_admin_ui.cjs` ;
 ![Diagnostic d’une tentative de test interrompue](docs/screenshots/installation/diagnostic-fr-etat.png)
 
 *Erreurs provoquées par la recette pour démontrer le diagnostic. Le message « Diagnostic copié » provient d’un presse-papiers simulé dans ce test, pas d’un essai de partage externe.*
+
+## Erreurs rencontrées et résolution
+
+Observations réelles du 29 septembre 2026 (mission Administration pilotée depuis les écrans). Détail : [RETEX](docs/RETEX-ADMIN-PREPARATION.md).
+
+| Erreur ou friction | Résolution |
+| --- | --- |
+| L’autorisation de l’équipe ne démarre pas les agents | Cliquez ensuite **Lancer la mission** dans le pilotage ; vérifiez qu’une tentative est active. |
+| La validation automatique refuse le plan | Elle exige une commande de contrôle par critère, y compris documentaire : ajoutez de vraies commandes ou choisissez la revue humaine. Aucun contrôle factice. |
+| Une tâche déclare des exigences « non définies » | Le contexte transmis était incomplet : révisez le plan pour inclure les définitions, puis reprenez. Les tentatives consommées restent comptées. |
+| Revue indépendante interrompue (citation introuvable) | Ce n’est pas une validation. Relancer la revue depuis le cockpit sur le même rapport ; ne pas forcer l’acceptation. |
+| Responsable sans réponse finale après son délai | Consulter l’activité de la session ; la reprise n’augmente pas silencieusement les limites. |
+| Le bouton « Soumettre le rapport » bloquait la revue | Défaut du moteur corrigé ; la réparation passe par le même bouton, sans modifier la base à la main. |
+
+Ne modifiez jamais directement `.swarm/state.db` pour contourner une erreur.
diff --git a/docs/en/INSTALL.md b/docs/en/INSTALL.md
index d440dda..15e0317 100644
--- a/docs/en/INSTALL.md
+++ b/docs/en/INSTALL.md
@@ -64,6 +64,10 @@ For text APIs, open **AI and connections → Add an AI connection**, enter a com
 These connections can prepare, plan and review; they do not automatically gain
 file editing tools.
 
+![Completed Add an AI connection form, before saving](../screenshots/en/connexion.png)
+
+*Actual capture of the completed form (example local provider, empty key field): neither tested nor saved; "Connection not tested in this form" is shown. No key is visible. Select **Test connection** then **Save connection**; saving does not prove the service responds.*
+
 For implementation, install a tool-enabled agent inside the container and
 follow its publisher's authentication instructions:
 
@@ -178,6 +182,14 @@ Automatic backups from some migrations do not replace a complete backup.
 
 Actual French-interface capture, September 29, 2026: the Administration mission was launched through the web screens. This demonstrates launch, not completion of all eight tasks. Results still require review.
 
+### Three states not to confuse
+
+| State | What you see | What it proves | Capture |
+| --- | --- | --- | --- |
+| Completed form | Entered values and a "current values" preview before confirmation | Nothing is written: the revision is unchanged | `admin-en-etat.png`, `admin-en-sombre.png` |
+| Saved configuration | Revision 1 in the scope and in the history, with author and reason | Values apply to **future starts** | `admin-saved-en-etat.png`, `admin-saved-en-sombre.png` |
+| Actual launch | Task in progress, active attempt on the dashboard | An agent started; no result, no acceptance | `mission-lancee-fr.png` (French UI) |
+
 **Current workflow:** team authorization and dashboard launch are separate steps. If nothing starts, read the reported cause before retrying.
 
 
@@ -197,6 +209,12 @@ Open **Administration** in the cockpit (switch to expert mode if it is hidden),
 
 ![The same form in the dark theme](../screenshots/installation/admin-en-sombre.png)
 
+![English Administration, light theme: configuration saved as revision 1](../screenshots/installation/admin-saved-en-etat.png)
+
+*Actual capture, isolated temporary root, September 30, 2026: after confirmation, revision 1 is read back by the CLI (`run-limits show`) and listed in the history. This is a saved configuration, not a launch. The server version badge is visible in the left rail.*
+
+![Saved configuration, dark theme](../screenshots/installation/admin-saved-en-sombre.png)
+
 When explicitly authorized for a mission, **observation mode** retains counters while removing the execution cutoffs covered by that mode. It does not remove provider quotas, review requirements or acceptance conditions. It cannot bypass a provider 429 response. **AI budgets and costs** distinguishes reported costs from unknown costs; unknown does not mean zero.
 
 ### Read the same settings from the CLI
@@ -211,6 +229,10 @@ swarm --root /path/to/project run-limits effective MISSION_ID worker TASK_ID
 
 A dash represents an empty value. `apply` and `rollback` use a JSON file, an expected revision and an event identifier. See the [user guide](USER-GUIDE.md) for the general workflow. The CLI and web use the same engine rules.
 
+## Server version and links to deleted missions
+
+The **Version** badge in the left rail shows the server's build revision. It is compared with the local Git repository without any network request: an absent revision is shown as unknown, and an impossible comparison is reported as degraded. A `*` means the working tree was modified at build time. A link to a deleted mission shows a clear message and the **Choose a mission** action, which opens mission management.
+
 ## Read the summary and share a diagnostic
 
 Before authorization, the summary shows the owner, workers, reviewer and their resolved models. Plan and planning/review-role caps are separate from execution settings in Administration. Missing prerequisites explain what you need to complete.
@@ -244,3 +266,18 @@ Administration screenshots come from `tests/run_limits_admin_ui.cjs`. Summary an
 ![Diagnostic from an interrupted test attempt](../screenshots/installation/diagnostic-en-etat.png)
 
 *Errors are deliberately generated by the test fixture. “Diagnostic copied” uses a mocked OS clipboard in this test; this is not evidence of an external share.*
+
+## Errors encountered and how they were resolved
+
+Actual observations from September 29, 2026 (Administration mission driven from the screens). Details: [RETEX](../RETEX-ADMIN-PREPARATION.md) (French).
+
+| Error or friction | Resolution |
+| --- | --- |
+| Team authorization does not start agents | Then select **Launch mission** on the dashboard and confirm an attempt is active. |
+| Automatic validation refuses the plan | It needs one check command per criterion, documentation criteria included: add real commands or choose human review. No placeholder check. |
+| A task reports requirements as "undefined" | The context supplied was incomplete: revise the plan to include the definitions, then resume. Consumed attempts remain counted. |
+| Independent review interrupted (quote not found) | This is not a validation. Resume the review from the cockpit on the same report; do not force acceptance. |
+| Owner gave no final answer before its deadline | Inspect the session activity; resuming does not silently raise limits. |
+| "Submit report" button blocked the review | Engine defect, fixed; repair goes through the same button, without editing the database by hand. |
+
+Never edit `.swarm/state.db` directly to work around an error.
```

### Images effectivement examinées

- `docs/screenshots/connexion.png` — SHA-256 `8c3c35786d543231d0a3d1954f40e834f898bbfa74be1f935131ebf2c8a20e6f` : Formulaire Ajouter une IA / Add an AI connection ; modèle local d’exemple, http://localhost:11434/v1 ; champ clé vide ; connexion non testée. Aucun secret visible.
- `docs/screenshots/installation/mission-lancee-fr.png` — SHA-256 `cc00614be99096b57ef22f6ec712b995f96636026a5351fc73c7ac3ac322c819` : Mission supervisée ; 1 tâche en cours et 1 tentative active ; prochaine étape attendre les résultats ; capture FR, légende anglaise précise French UI. Aucun secret visible.
- `docs/screenshots/installation/admin-fr-etat.png` — SHA-256 `c85eb915eb5d1f455947c282cdd2243d7d1bb809213c432ae468c3951ea213ef` : Formulaire de limites 30/120/50/3/2 ; motif ui recette ; bouton confirmer encore présent ; aucune clé ni URL. Aucun secret visible.
- `docs/screenshots/installation/admin-fr-sombre.png` — SHA-256 `2dc8dd46ac4d3217f48d248aa2ca77c22e3fd6498642ab041835d4f6fd079e68` : Formulaire de limites 30/120/50/3/2 ; motif ui recette ; bouton confirmer encore présent ; aucune clé ni URL. Aucun secret visible.
- `docs/screenshots/installation/admin-saved-fr-etat.png` — SHA-256 `3b4bac2055c3b2eef0ac75c20ffbc91a8a721ca2a94bce2dad7c644f2a0ee4db` : Historique avec révision 1 ; limites 30/120/50/3/2 ; auteur local fpizzi/uid:1001 et motif install guide ; aucun champ de clé ni URL de session. Aucun secret visible.
- `docs/screenshots/installation/admin-saved-fr-sombre.png` — SHA-256 `ad6c543c90a0c347cfa0c86510fd9ab964f00c3fe17c0179a22d00d0faf2a376` : Historique avec révision 1 ; limites 30/120/50/3/2 ; auteur local fpizzi/uid:1001 et motif install guide ; aucun champ de clé ni URL de session. Aucun secret visible.
- `docs/screenshots/installation/summary-fr-etat.png` — SHA-256 `ea705fa6fbfea0bb2077bf05e7bb99e3914b4dc1adf6ca7ac4fdfb1ddc14616f` : Récapitulatif avant autorisation ; fournisseur recette/sonnet ; limites 15/25 ; conditions vérifiées ; bouton autoriser non activé ici. Aucun secret visible.
- `docs/screenshots/installation/preflight-details-fr-sombre.png` — SHA-256 `b3b141054d142e8463f246384827d544a5c21a5f2bb56e09137f8a09ee28c6a2` : Modale technique ready/verified, exécutable et limites valides, sonde/dossier/écriture temporaire ; quantité d’octets ; ni chemin précis, ni clé, ni URL. Aucun secret visible.
- `docs/screenshots/installation/diagnostic-fr-etat.png` — SHA-256 `379651de02625dda15ea517e449edec4112b9967ce0d2d3b6fe9a6bc1fe8868b` : Diagnostic de 3 erreurs provoquées ; catégories configuration/environnement/contrôle/limite ; traces repliées ; bouton copier et succès simulé ; aucune clé ni URL visible. Aucun secret visible.
- `docs/screenshots/en/connexion.png` — SHA-256 `ab1649d8f983c4f6178d4c5853f7a6d2fa1a718f297327738ce413a4d24b9f5e` : Formulaire Ajouter une IA / Add an AI connection ; modèle local d’exemple, http://localhost:11434/v1 ; champ clé vide ; connexion non testée. Aucun secret visible.
- `docs/screenshots/installation/admin-en-etat.png` — SHA-256 `c1d378f7577c037d3158ee76331988bd405a9afb1b70864242ef982283019797` : Formulaire de limites 30/120/50/3/2 ; motif ui recette ; bouton confirmer encore présent ; aucune clé ni URL. Aucun secret visible.
- `docs/screenshots/installation/admin-en-sombre.png` — SHA-256 `3cc26331f3f1545489df6c95bdaccc62b6776cffdb9605bbc0b38110c4dff23e` : Formulaire de limites 30/120/50/3/2 ; motif ui recette ; bouton confirmer encore présent ; aucune clé ni URL. Aucun secret visible.
- `docs/screenshots/installation/admin-saved-en-etat.png` — SHA-256 `6804f9f4f0f56cd0cecf710700c73440c4c6491de44b046687ab974bdd26d99e` : Historique avec révision 1 ; limites 30/120/50/3/2 ; auteur local fpizzi/uid:1001 et motif install guide ; aucun champ de clé ni URL de session. Aucun secret visible.
- `docs/screenshots/installation/admin-saved-en-sombre.png` — SHA-256 `10e19be2449a6aa46275a50e974808aa5f8c3adae47b25bfe8700396d513d5bd` : Historique avec révision 1 ; limites 30/120/50/3/2 ; auteur local fpizzi/uid:1001 et motif install guide ; aucun champ de clé ni URL de session. Aucun secret visible.
- `docs/screenshots/installation/summary-en-etat.png` — SHA-256 `30a54b451fc15dbaeff5de1c4e98f3dfc43433479ef172517e62779f5d78f042` : Summary before authorization ; provider Select ; limits 20/40 ; AI to choose, Validation to choose, Preflight not run ; aucun agent lancé. Aucun secret visible.
- `docs/screenshots/installation/preflight-details-en-sombre.png` — SHA-256 `42443f5a84284717665826710b4548f7b24bf59d45764744241cebe92feb027b` : Modale technique ready/verified, exécutable et limites valides, sonde/dossier/écriture temporaire ; quantité d’octets ; ni chemin précis, ni clé, ni URL. Aucun secret visible.
- `docs/screenshots/installation/diagnostic-en-etat.png` — SHA-256 `a3958e9f31a8c4ea5ff6fa78659b6b0309e63ba735de7529d08ba5e5aaa111ca` : Diagnostic de 3 erreurs provoquées ; catégories configuration/environnement/contrôle/limite ; traces repliées ; bouton copier et succès simulé ; aucune clé ni URL visible. Aucun secret visible.

### Contrôle documentaire, code exact

```python
#!/usr/bin/env python3
"""Read-only installation evidence checks; browser replay uses a temporary root."""
import hashlib
import json
import pathlib
import re
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]


def run(args):
    print('COMMAND', args, flush=True)
    return subprocess.run(args, cwd=ROOT, check=True, timeout=180,
                          text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT).stdout


review = json.loads((ROOT / 'docs/plan-843bb3ce22-T8-visual-review.json').read_text())
images = {item['path']: item for item in review['images']}
referenced = set()
for name in ('INSTALL.md', 'docs/en/INSTALL.md'):
    text = (ROOT / name).read_text()
    before = run(['git', 'show', '7110a0b805caf4be1bb09a57b776176b8dc52a68:' + name])
    # Every original line must remain, in the original order.
    remaining = iter(text.splitlines())
    for line in before.splitlines():
        assert any(current == line for current in remaining), 'original content removed: ' + name
    for target in re.findall(r'!\[[^\]]*\]\(([^)]+)\)', text):
        image = (ROOT / name).parent.joinpath(target).resolve()
        relative = str(image.relative_to(ROOT))
        referenced.add(relative)
        assert relative in images, 'image without visual review: ' + relative
        assert hashlib.sha256(image.read_bytes()).hexdigest() == images[relative]['sha256'], 'image changed after visual review'
        assert images[relative]['secret_visible'] is False and images[relative]['observation']
    print('PASS original content preserved in', name, flush=True)
assert referenced == set(images) and len(images) == 17
assert 'French UI' in (ROOT / 'docs/en/INSTALL.md').read_text()
run(['git', 'diff', '--check'])
print('PASS 17 referenced images match the visual review; this is hash binding, not automatic image analysis', flush=True)
with tempfile.TemporaryDirectory(prefix='swarm-t8-control-') as directory:
    binary = str(pathlib.Path(directory) / 'swarm')
    run(['go', 'build', '-trimpath', '-o', binary, '.'])
    output = run(['node', 'tools/verification/t8_saved_config_shots.cjs', binary, str(pathlib.Path(directory) / 'shots')])
    print(output, flush=True)
    assert 'PASS fr saved revision 1' in output and 'PASS en saved revision 1' in output
print('PASS FR/EN saved state replay and read-back; launch screenshot is historical, captioned as French UI', flush=True)
```

### Recette d’enregistrement, code exact

```javascript
'use strict';
// T8: real browser capture of a SAVED run-limits configuration (FR/EN, 2 themes)
// in an isolated temporary root. Usage: node t8_saved_config_shots.cjs BINARY OUTDIR
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-t8-'));fs.mkdirSync(out,{recursive:true});
function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let t='',e='';p.stdout.on('data',x=>t+=x);p.stderr.on('data',x=>e+=x);p.on('exit',c=>c?reject(Error(e||t)):resolve(t?JSON.parse(t):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);
 const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Install guide capture',objective:'Capture saved limits',scope:'isolated',criteria:['No secret shown'],next:'Configure'})).work;
 const app=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 const url=await new Promise((res,rej)=>{let t='';app.stdout.on('data',x=>{t+=x;const m=t.match(/http:\/\/[^\s]+/);if(m)res(m[0])});app.on('exit',c=>rej(Error('server '+c)))});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const errors=[];
 try{
  for(const lang of ['fr','en']){
   const key='qa-'+lang;
   const p=await browser.newPage();p.setDefaultTimeout(30000);
   p.on('pageerror',e=>errors.push(lang+' pageerror: '+e.message));
   p.on('console',m=>{if(m.type()==='error')errors.push(lang+' console: '+m.text())});
   p.on('requestfailed',r=>errors.push(lang+' requestfailed: '+r.url()));
   await p.setViewport({width:1400,height:1000});
   const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);
   await p.waitForFunction(()=>typeof snapshot!=='undefined'&&snapshot?.work);
   if(await p.$eval('[data-view="admin"]',e=>e.getClientRects().length===0))await p.click('#mode');
   await p.click('[data-view="admin"]');
   await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);
   await p.select('#admin-scope','role');
   await p.$eval('#admin-mission',(e,v)=>{e.value=v},w.id);
   await p.$eval('#admin-key',(e,v)=>{e.value=v},key);
   await p.click('#admin-load');
   await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);
   await p.click('#admin-current button');
   await p.waitForSelector('#field-silence_seconds');
   for(const [k,v] of Object.entries({silence_seconds:'30',tool_seconds:'120',max_tool_calls:'50',max_repeated_calls:'3',max_consecutive_errors:'2',reason:'install guide'}))
    await p.$eval('#field-'+k,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},v);
   await p.click('#confirm');
   await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
   await p.click('#confirm');
   await p.waitForFunction(()=>!document.querySelector('#modal').open);
   const saved=await cli(['run-limits','show','role',w.id,key]);
   assert.equal(saved.revision,1);assert.equal(saved.values.silence_seconds,30);
   await p.waitForFunction(()=>/(Révision|Revision) 1/.test(document.querySelector('#admin-current').textContent));
   const txt=await p.$eval('#admin',e=>e.textContent.replace(/Aucun secret n.est affiché sur cet écran\.?|No secret is shown on this screen\.?/gi,''));
   assert(!/api[_-]?key|bearer |secret|password|mot de passe/i.test(txt),'secret-like text');
   await p.$eval('#admin-history',e=>e.scrollIntoView({block:'center'}));
   for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'admin-saved-'+lang+'-'+theme+'.png')})}
   console.log('PASS',lang,'saved revision',saved.revision);
  }
  assert.deepEqual(errors,[]);
 }finally{await browser.close();app.kill()}
})().catch(e=>{console.error('FAIL',e.message);process.exit(1)});
```

## Revue visuelle par le moteur

Les 17 captures sont déclarées explicitement dans les entrées du contrôle
`t8-install-evidence`. Le moteur joint leurs octets au vérificateur Claude
sans lui donner d’outils. Les mêmes captures restent liées par empreinte au
reçu ; changer une image invalide le dossier de revue. Cette transmission
permet une véritable observation visuelle dans le contexte indépendant.
Aucune capture n’est suivie automatiquement depuis un lien du rapport.
Les adaptateurs sans prise en charge visuelle refusent ce parcours.
L’acceptation finale reste à enregistrer après le verdict indépendant.
