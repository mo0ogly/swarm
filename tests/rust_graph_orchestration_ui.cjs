'use strict';
const assert = require('node:assert/strict');
const path = require('node:path');
const puppeteer = require('puppeteer');

/** Verify real cockpit graph rendering from controlled public response shapes.
 * The local browser loads source assets, with no Swarm service or provider.
 */
async function run() {
  const web = path.resolve('web');
  const browser = await puppeteer.launch({
    headless: true, executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
    timeout: 0, protocolTimeout: 0, args: ['--no-sandbox']
  });
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    await page.setContent('<main id="graph"></main>');
    for (const source of ['wattson_themes.css', 'pilotage.css', 'graph.css', 'swarm-design.css']) {
      await page.addStyleTag({path: path.join(web, source)});
    }
    for (const source of ['i18n-en.js', 'lib/dagre.min.js', 'graph.js',
      'cockpit-native-graph.js', 'cockpit-native-read.js']) {
      await page.addScriptTag({path: path.join(web, source)});
    }
    for (const [lang, theme, width] of [
      ['fr', 'etat', 1440], ['fr', 'sombre', 320], ['en', 'etat', 390], ['en', 'sombre', 1440]
    ]) {
      await page.setViewport({width, height: 900});
      const reducedMotion = lang === 'en' && theme === 'sombre';
      await page.emulateMediaFeatures([{name: 'prefers-reduced-motion', value: reducedMotion ? 'reduce' : 'no-preference'}]);
      const result = await page.evaluate(({lang, theme}) => {
        document.documentElement.lang = lang;
        document.documentElement.dataset.theme = theme;
        globalThis.SwarmI18n = {t: key => lang === 'en' ? SwarmEnglish[key] ?? key : key};
        const requests = [];
        window.fetch = (url, options) => {
          requests.push({path: new URL(url, 'http://localhost').pathname, method: options?.method ?? 'GET'});
          return Promise.resolve(new Response('{"failure":{"code":"fixture-read-unavailable"}}', {status:404}));
        };
        const tasks = [
          {id: 'wasm', title: 'Wasmtime', status: 'blocked', scope_id: 'validation',
            independent_review: {state: 'changes_requested'},
            attempts: [{status: 'interrupted'}],
            validation_policy: {controls: [{id: 'wasmtime-current', command: ['private argv']}]},
            automatic_validation: {controls: [{id: 'wasmtime-previous', executed: true, passed: false}]}},
          {id: 'rpc', title: 'gRPC', status: 'todo', scope_id: 'transport', depends: ['wasm']},
          ...Array.from({length: 58}, (_, index) => ({
            id: 'independent-' + index, title: 'Independent ' + index,
            status: index < 2 ? 'running' : index === 2 ? 'submitted' : 'todo',
            scope_id: 'documentation'
          }))
        ];
        const work = {
          id: 'owned-orchestration', title: 'Biba fixture', tasks,
          planning: {provider: 'glm', model_route: {model: 'glm-fixture'},
            reviewer: {provider: 'review-fixture'},
            scopes: [
              {id: 'transport', parent: 'root'}, {id: 'root'},
              {id: 'validation', parent: 'root'}, {id: 'documentation', parent: 'transport'}
            ]}
        };
        const mission = {paused: true, supervision: {state: 'active'}, tasks: [{
          id: 'rpc', reason: 'Waiting for independent Wasmtime review'
        }, {
          id: 'wasm', result: {state: 'stopped_early', process_state: 'interrupted', validation_label: 'Current verification'}
        }, {
          id: 'independent-0', attempt: {agent_id:'fixture-worker'}, result: {state: 'in_progress', process_state: 'running'}
        }, {
          id: 'independent-1', result: {state: 'execution_unconfirmed', process_state: 'unknown'}
        }, {
          id: 'independent-2', result: {state: 'review_in_progress', process_state: 'completed'}
        }]};
        SwarmRustCockpitRead.render({work, mission});
        const get = selector => document.querySelector(selector);
        const role = value => get('[data-orchestration-role="' + value + '"]');
        const visible = selector => [...document.querySelectorAll(selector)]
          .filter(node => getComputedStyle(node).display !== 'none');
        const drill = {
          initialRoles:visible('.graph-workflow-role').length,
          initialTasks:visible('.graph-noeud').length,
          initialReview:visible('[data-relation="review"]').length,
          roleStatuses:visible('[data-workflow-scope] .graph-activity-mark').map(node => node.dataset.activity)
        };
        const transport = get('[data-workflow-scope="transport"]');
        transport.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}));
        drill.scopedTasks=visible('.graph-noeud').map(node=>node.dataset.task);
        drill.scopedLinks=visible('[data-assigned-task]').map(node=>node.dataset.assignedTask);
        drill.scopedMinimap=visible('.graph-minimap').length;
        get('#pilot-drill-up').click();
        drill.returned=visible('.graph-noeud').length;
        drill.focusRestored=document.activeElement===transport;
        get('#pilot-drill-all').click();
        drill.worker=get('.graph-noeud[data-task="independent-0"] [data-task-agent]').textContent;
        drill.handle=get('.graph-noeud[data-task="rpc"] .graph-drag title').textContent;

        const activity = id => {
          const mark = get('.graph-noeud[data-task="' + id + '"] .graph-activity-mark');
          return mark ? [mark.dataset.activity, getComputedStyle(mark).fill, mark.nextElementSibling.textContent] : null;
        };
        const currentActivity = ['rpc', 'wasm', 'independent-0', 'independent-1'].map(activity);
        const runningFrames = () => Array.from(document.querySelectorAll('.graph-activity-frame'));
        const frames = runningFrames();
        const frame = frames[0];
        const motion = frame?.getAnimations()[0];
        const timing = motion?.effect.getComputedTiming();
        const initialOffset = frame && getComputedStyle(frame).strokeDashoffset;
        if (motion) motion.currentTime = timing.duration / 2;
        const frameState = {
          tasks: frames.map(frame => frame.closest('.graph-noeud').dataset.task),
          hidden: frame?.getAttribute('aria-hidden'),
          interactive: frame && getComputedStyle(frame).pointerEvents,
          animates: !!motion,
          changesOffset: frame && getComputedStyle(frame).strokeDashoffset !== initialOffset,
          duration: timing?.duration, continuous: timing?.iterations === Infinity
        };

        const workflow = {
          roles: [...document.querySelectorAll('.graph-workflow-role')].map(node=>node.dataset.agentRole),
          delegated: [...document.querySelectorAll('[data-relation="delegation"]')].map(edge=>edge.dataset.fromRole+'>'+edge.dataset.toRole),
          reviewerLinks: document.querySelectorAll('[data-relation="review"]').length,
          assignments: document.querySelectorAll('[data-assigned-task][marker-end]').length,
          permanentEdges: [...document.querySelectorAll('[data-relation="delegation"], [data-relation="review"]')]
            .every(edge => getComputedStyle(edge).display !== 'none')
        };
        function visibleAssignments() {
          return [...document.querySelectorAll('[data-assigned-task]')]
            .filter(edge => getComputedStyle(edge).display !== 'none')
            .map(edge => edge.dataset.assignedTask);
        }
        workflow.defaultAssignments = visibleAssignments().length;
        workflow.organization = get('#graph-studio strong')?.textContent;
        workflow.delegationDash = getComputedStyle(get('[data-relation="delegation"]')).strokeDasharray;
        const assignmentScope = get('#pilot-assignment-scope');
        if (assignmentScope) {
          assignmentScope.value = 'transport';
          assignmentScope.dispatchEvent(new Event('change', {bubbles:true}));
          workflow.focusedAssignments = visibleAssignments();
          assignmentScope.value = '';
          assignmentScope.dispatchEvent(new Event('change', {bubbles:true}));
          workflow.clearedAssignments = visibleAssignments();
        }
        const motionButton=get('#pilot-activity-motion');
        motionButton.click();
        workflow.pause=motionButton.getAttribute('aria-pressed');
        workflow.playState=getComputedStyle(frame).animationPlayState;
        motionButton.click();

        assertRenderLocal();
        function assertRenderLocal() {
          if (requests.length) throw new Error('Graph rendering triggered a request');
        }
        const initialGraph = !get('#graph-studio').hidden && get('#conduite-team-validation').hidden;
        const canvas = get('#pilot-canvas');
        canvas.scrollLeft = 100;
        canvas.scrollTop = 100;
        const scroll = () => [get('#pilot-canvas').scrollLeft, get('#pilot-canvas').scrollTop];
        const visibleScroll = scroll();
        get('#conduite-show-team').click();
        SwarmRustCockpitRead.render({work, mission});
        get('#conduite-show-graph').click();
        const preservedScroll = scroll();
        get('#conduite-show-team').click();
        const root = role('root');
        const roleIcons = ['root', 'subplanner', 'reviewer'].map(value => {
          const icon = role(value).querySelector('.orchestration-role-icon');
          if (!icon) return null;
          const style = getComputedStyle(icon);
          return {
            glyph: getComputedStyle(icon, '::before').content,
            background: style.backgroundColor,
            foreground: style.color,
            labelBackground: getComputedStyle(icon.parentElement).backgroundColor,
            hidden: icon.getAttribute('aria-hidden'),
            visible: icon.getBoundingClientRect().width > 0,
          };
        });
        const branches = Array.from(root.querySelectorAll(':scope > ul > li'), node => node.dataset.scope);
        const connectsCards = (vertical) => {
          const edge = get('.graph-arete[data-from="wasm"][data-to="rpc"]');
          const source = get('.graph-noeud[data-task="wasm"] .graph-cadre');
          const target = get('.graph-noeud[data-task="rpc"] .graph-cadre');
          const start = edge.getPointAtLength(0), end = edge.getPointAtLength(edge.getTotalLength());
          const box = node => ['x', 'y', 'width', 'height'].map(name => Number(node.getAttribute(name)));
          const [sx, sy, sw, sh] = box(source), [tx, ty, tw, th] = box(target);
          const expected = vertical
            ? [sx + sw / 2, sy + sh, tx + tw / 2, ty]
            : [sx + sw, sy + sh / 2, tx, ty + th / 2];
          return [start.x, start.y, end.x, end.y].every((value, i) => Math.abs(value - expected[i]) < 1);
        };
        get('#conduite-show-graph').click();
        const lrConnected = connectsCards(false);
        get('#conduite-show-team').click();
        const snapshot = {
          initialGraph,
          currentActivity,
          frameState,
          roleActivity: ['root', 'subplanner', 'reviewer', 'driver'].map(value => {
            const mark = role(value).querySelector(':scope > .graph-activity-key');
            return [mark?.dataset.activity, mark?.textContent];
          }),
          assignedActivity: get('[data-scope="transport"] details li > .graph-activity-key').dataset.activity,
          visibleScroll,
          preservedScroll,
          teamOnly: get('#graph-studio').hidden && !get('#conduite-team-validation').hidden,
          recordedReview: get('[data-orchestration-review]').textContent,
          currentValidation: get('[data-orchestration-validation]').textContent,
          roleIcons,
          bandCount: document.querySelectorAll('[data-graph-branch]').length,
          lrConnected,
          driver: role('driver').textContent, reviewer: role('reviewer').textContent,
          root: root.firstElementChild.textContent, branches,
          nested: get('[data-scope="transport"] [data-scope="documentation"] details summary').textContent,
          taskCount: document.querySelectorAll('.graph-noeud').length,
          links: Array.from(document.querySelectorAll('.graph-arete'), e => [e.dataset.from, e.dataset.to]),
          reason: get('.graph-task-open[data-task="rpc"]').getAttribute('aria-label'),
          scopeReason: get('[data-scope="transport"] details li').textContent,
          controlReceipt: get('[data-orchestration-controls]').textContent,
          validationScope: get('[data-scope="validation"]').textContent,
          width: get('.native-graph-orchestration').getBoundingClientRect().right,
          viewport: innerWidth
        };
        get('#conduite-show-graph').click();
        const orientation = get('#pilot-orientation');
        orientation.value = 'TB';
        orientation.dispatchEvent(new Event('change', {bubbles: true}));
        snapshot.tbConnected = connectsCards(true);
        snapshot.orientedDriver = role('driver').textContent;
        snapshot.orientedReason = get('.graph-task-open[data-task="rpc"]').getAttribute('aria-label');
        get('#conduite-show-team').click();
        get('[data-orchestration-tree]').open = false;
        SwarmRustCockpitRead.render({work, mission, readMode: 'history', control: {paused: true}});
        snapshot.historyActivity = ['rpc', 'wasm', 'independent-0', 'independent-1'].map(activity);
        snapshot.historyDriver = role('driver').textContent;
        snapshot.historyFrames = runningFrames().length;
        snapshot.historyReason = get('.graph-task-open[data-task="rpc"]').getAttribute('aria-label');
        snapshot.retainedDisclosure = !get('[data-orchestration-tree]').open;
        snapshot.retainedTab = !get('#conduite-team-validation').hidden;
        snapshot.historyHasCurrentResult = !!get('[data-orchestration-validation]');
        const malformed = structuredClone(work);
        malformed.planning.scopes[0].parent = 'missing';
        SwarmRustCockpitRead.render({work: malformed, mission});
        snapshot.partialTree = get('[data-orchestration-tree]').textContent;
        snapshot.orphanHidden = !get('[data-scope="transport"]');
        SwarmRustCockpitRead.render({work: {...work, id: 'ordinary', planning: null}, mission: {
          supervision: {state: 'absent'}
        }});
        snapshot.unattached = get('.native-graph-orchestration').textContent;
        snapshot.ordinary = {
          root: role('root')?.dataset.scope, subplanners: document.querySelectorAll('[data-orchestration-role="subplanner"]').length,
          links: document.querySelectorAll('.graph-arete').length
        };
        SwarmRustCockpitRead.render({work: {id: 'new-work', title: 'Empty work', tasks: []}, mission: {
          organization: {root_scope: 'root', ready: false}, supervision: {state: 'absent'}
        }});
        get('#conduite-show-team').click();
        snapshot.newWork = {
          root: role('root')?.dataset.scope, name: role('root')?.textContent,
          reviewer: role('reviewer')?.textContent, visible: !!role('root')?.getClientRects().length,
          scopeCount: document.querySelectorAll('[data-scope]').length,
          taskCount: document.querySelectorAll('.graph-noeud').length
        };
        snapshot.workflow = workflow;
        snapshot.drill = drill;
        snapshot.requests = requests;
        return snapshot;
      }, {lang, theme});
      assert.equal(result.drill.initialRoles,4);
      assert.equal(result.drill.initialTasks,0);
      assert.equal(result.drill.initialReview,0);
      assert.deepEqual(result.drill.roleStatuses,Array(4).fill('paused'));
      assert.deepEqual(result.drill.scopedTasks,['rpc']);
      assert.deepEqual(result.drill.scopedLinks,['rpc']);
      assert.equal(result.drill.scopedMinimap,0);
      assert.equal(result.drill.returned,0);
      assert.equal(result.drill.focusRestored,true);
      assert.match(result.drill.worker,/fixture-worker/);
      assert.equal(result.drill.handle,lang === 'fr' ? 'Poignée de déplacement' : 'Drag handle');
      assert.deepEqual(result.branches, ['transport', 'validation']);
      assert.deepEqual(result.currentActivity.map(item => item?.[0]), ['paused', 'stopped', 'running', 'unconfirmed']);
      assert.deepEqual(result.frameState.tasks, ['independent-0'], 'only the confirmed task has an activity frame');
      assert.equal(result.frameState.hidden, 'true');
      assert.equal(result.frameState.interactive, 'none');
      assert.equal(result.frameState.animates, !reducedMotion);
      assert.equal(result.frameState.changesOffset, !reducedMotion);
      if (!reducedMotion) assert(result.frameState.duration > 0 && result.frameState.duration < 5000);
      assert.equal(result.historyFrames, 0);
      assert.equal(result.frameState.continuous,!reducedMotion);
      assert.deepEqual(result.workflow.roles,['root','subplanner','subplanner','subplanner','reviewer']);
      assert.deepEqual(result.workflow.delegated,['root>transport','root>validation','transport>documentation']);
      assert.equal(result.workflow.reviewerLinks,1);
      assert.equal(result.workflow.assignments,60);
      assert.equal(result.workflow.permanentEdges,true);
      assert.equal(result.workflow.defaultAssignments,60);
      assert.equal(result.workflow.organization,lang === 'fr' ? 'Organisation agentique · Hiérarchique' : 'Agent organization · Hierarchical');
      assert.equal(result.workflow.delegationDash,'none');
      assert.deepEqual(result.workflow.focusedAssignments,['rpc']);
      assert.deepEqual(result.workflow.clearedAssignments,[]);
      assert.equal(result.workflow.pause,'true');
      if (!reducedMotion) assert.equal(result.workflow.playState,'paused');


      assert.equal(new Set(result.currentActivity.map(item => item[1])).size, 4);
      const activityLabels = lang === 'en'
        ? ['Paused', 'Stopped', 'In action', 'Activity unconfirmed']
        : ['En pause', 'Arrêtée', 'En action', 'Activité à confirmer'];
      assert.deepEqual(result.currentActivity.map(item => item[2]), activityLabels);
      assert.deepEqual(result.roleActivity.map(item => item[0]), ['paused', 'paused', 'running', 'running']);
      assert.match(result.roleActivity[2][1], lang === 'en' ? /Independent review in progress/ : /Revue indépendante en cours/);
      assert.equal(result.assignedActivity, 'paused');
      assert.equal(result.historyActivity[0]?.[0], 'paused');
      assert.equal(result.historyActivity[1]?.[0], 'stopped');
      assert.equal(result.historyActivity[2]?.[0], 'unconfirmed');
      assert(result.initialGraph && result.teamOnly && result.retainedTab);
      assert(result.visibleScroll.some(offset => offset > 0));
      assert.deepEqual(result.preservedScroll, result.visibleScroll);
      assert(!result.historyHasCurrentResult);
      assert.match(result.recordedReview, lang === 'en' ? /Changes or evidence requested/ : /Corrections ou preuves demandées/);
      assert.equal(result.currentValidation, 'Current verification');
      assert.deepEqual(result.roleIcons.map(icon => icon?.glyph), ['"◆"', '"↳"', '"✓"']);
      assert.equal(new Set(result.roleIcons.map(icon => icon.background)).size, 3);
      assert.equal(new Set(result.roleIcons.map(icon => icon.labelBackground)).size, 3);
      for (const icon of result.roleIcons) {
        assert.equal(icon.hidden, 'true');
        assert(icon.visible);
        assert.notEqual(icon.foreground, icon.background);
        assert.equal(icon.labelBackground, icon.foreground);
      }
      assert.equal(result.taskCount, 60);
      assert.equal(result.bandCount, 3);
      assert(result.lrConnected && result.tbConnected, 'dependency endpoints follow the grouped cards');
      assert.deepEqual(result.links, [['wasm', 'rpc']]);
      assert.match(result.root, /glm · glm-fixture/);
      assert.match(result.reviewer, /review-fixture/);
      assert.match(result.nested, /58/);
      assert.match(result.reason, /transport.*Waiting for independent Wasmtime review/);
      assert.equal(result.orientedReason, result.reason);
      assert.equal(result.orientedDriver, result.driver);
      assert.match(result.scopeReason, /gRPC.*Waiting for independent Wasmtime review/);
      assert.match(result.controlReceipt, /wasmtime-previous/);
      assert.match(result.validationScope, /wasmtime-current/);
      assert.doesNotMatch(result.validationScope, /private argv/);
      assert.doesNotMatch(result.historyReason, /Waiting for/);
      assert(result.width <= result.viewport);
      assert(result.retainedDisclosure && result.orphanHidden);
      assert.deepEqual(result.ordinary, {root: 'root', subplanners: 0, links: 1});
      assert.equal(result.newWork.root, 'root');
      assert.equal(result.newWork.scopeCount, 1);
      assert.equal(result.newWork.taskCount, 0);
      assert(result.newWork.visible);
      assert.deepEqual(result.requests, Array.from({length:6}, () => ({
        path:'/api/v1/planning/workflow-architecture', method:'GET'
      })));
      if (lang === 'en') {
        assert.match(result.root, /^Principal orchestrator/);
        assert.match(result.reviewer, /^Independent reviewer/);
        assert.match(result.partialTree, /Incomplete hierarchy/);
        assert.match(result.unattached, /Tasks without a displayed scope.*60/);
        assert.equal(result.newWork.name, 'Principal orchestrator · To configure');
        assert.equal(result.newWork.reviewer, 'Independent reviewer · Not configured');
        assert.equal(result.historyDriver, 'Swarm driver status unavailable.');
        assert.equal(result.driver, 'Swarm driver observed.');
        assert.match(result.controlReceipt, /Recorded failure/);
      } else {
        assert.match(result.root, /^Orchestrateur principal/);
        assert.match(result.reviewer, /^Validateur indépendant/);
        assert.match(result.partialTree, /Hiérarchie incomplète/);
        assert.match(result.unattached, /Tâches sans rattachement affichable.*60/);
        assert.equal(result.newWork.name, 'Orchestrateur principal · À configurer');
        assert.equal(result.newWork.reviewer, 'Validateur indépendant · Non configuré');
        assert.equal(result.historyDriver, 'État du conducteur indisponible.');
        assert.equal(result.driver, 'Conducteur Swarm observé.');
        assert.match(result.controlReceipt, /Échec enregistré/);
      }
      console.log('PASS orchestration graph', lang, theme, width);
      await page.focus('#conduite-show-graph');
      await page.keyboard.press('Enter');
      assert.equal(await page.$eval('#conduite-show-graph', button => button.getAttribute('aria-pressed')), 'true');
      assert.equal(await page.$eval('#conduite-show-team', button => button.getAttribute('aria-pressed')), 'false');
      assert(await page.$eval('#graph-studio', panel => !panel.hidden && panel.getClientRects().length > 0));
      assert(await page.$eval('#conduite-team-validation', panel => panel.hidden));
      await page.click('#pilot-fullscreen');
      assert(await page.$eval('dialog.native-graph-dialog[open]', dialog => {
        const button = dialog.querySelector('#conduite-show-team');
        if (!button) return false;
        const bounds = button.getBoundingClientRect();
        return bounds.width > 0 && bounds.top >= 0 && bounds.bottom <= innerHeight &&
          button.contains(document.elementFromPoint(bounds.x + bounds.width / 2,
            bounds.y + bounds.height / 2));
      }), 'Team switch is visible and interactive in fullscreen');
      if (width === 320) {
        assert(await page.$eval('.native-graph-gesture-hint', hint => !hint.getClientRects().length));
        assert(await page.$eval('#pilot-graph-help-toggle', help => help.getClientRects().length > 0));
      }
      await page.focus('#conduite-show-team');
      await page.keyboard.press('Enter');
      await page.waitForFunction(() => !document.querySelector('dialog.native-graph-dialog').open &&
        !document.querySelector('#conduite-team-validation').hidden &&
        document.activeElement.id === 'conduite-show-team', {timeout: 0});
      await page.focus('#conduite-show-graph');
      await page.keyboard.press('Enter');
      assert(await page.$eval('#graph-studio', panel =>
        !panel.hidden && panel.contains(document.activeElement)));
      assert(await page.$eval('#conduite-team-validation', panel => panel.hidden));
      console.log('PASS fullscreen view switch and keyboard focus', lang, theme, width);
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
}

run().catch(error => { console.error(error); process.exitCode = 1; });
