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
    for (const source of ['wattson_themes.css', 'graph.css', 'swarm-design.css']) {
      await page.addStyleTag({path: path.join(web, source)});
    }
    for (const source of ['i18n-en.js', 'lib/dagre.min.js', 'graph.js',
      'cockpit-native-graph.js', 'cockpit-native-read.js']) {
      await page.addScriptTag({path: path.join(web, source)});
    }
    for (const [lang, theme, width] of [
      ['fr', 'etat', 1440], ['fr', 'sombre', 390], ['en', 'etat', 390], ['en', 'sombre', 1440]
    ]) {
      await page.setViewport({width, height: 900});
      const result = await page.evaluate(({lang, theme}) => {
        document.documentElement.lang = lang;
        document.documentElement.dataset.theme = theme;
        globalThis.SwarmI18n = {t: key => lang === 'en' ? SwarmEnglish[key] ?? key : key};
        let requests = 0;
        window.fetch = () => { requests++; throw new Error('A render must make no requests'); };
        const tasks = [
          {id: 'wasm', title: 'Wasmtime', status: 'blocked', scope_id: 'validation',
            validation_policy: {controls: [{id: 'wasmtime-current', command: ['private argv']}]},
            automatic_validation: {controls: [{id: 'wasmtime-previous', executed: true, passed: false}]}},
          {id: 'rpc', title: 'gRPC', status: 'todo', scope_id: 'transport', depends: ['wasm']},
          ...Array.from({length: 58}, (_, index) => ({
            id: 'independent-' + index, title: 'Independent ' + index,
            status: 'todo', scope_id: 'documentation'
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
        const mission = {supervision: {state: 'active'}, tasks: [{
          id: 'rpc', reason: 'Waiting for independent Wasmtime review'
        }]};
        SwarmRustCockpitRead.render({work, mission});
        const get = selector => document.querySelector(selector);
        const role = value => get('[data-orchestration-role="' + value + '"]');
        const root = role('root');
        const branches = Array.from(root.querySelectorAll(':scope > ul > li'), node => node.dataset.scope);
        const snapshot = {
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
        const orientation = get('#pilot-orientation');
        orientation.value = 'TB';
        orientation.dispatchEvent(new Event('change', {bubbles: true}));
        snapshot.orientedDriver = role('driver').textContent;
        snapshot.orientedReason = get('.graph-task-open[data-task="rpc"]').getAttribute('aria-label');
        get('[data-orchestration-tree]').open = false;
        SwarmRustCockpitRead.render({work, mission, readMode: 'history'});
        snapshot.historyDriver = role('driver').textContent;
        snapshot.historyReason = get('.graph-task-open[data-task="rpc"]').getAttribute('aria-label');
        snapshot.retainedDisclosure = !get('[data-orchestration-tree]').open;
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
        snapshot.newWork = {
          root: role('root')?.dataset.scope, name: role('root')?.textContent,
          reviewer: role('reviewer')?.textContent, visible: !!role('root')?.getClientRects().length,
          scopeCount: document.querySelectorAll('[data-scope]').length,
          taskCount: document.querySelectorAll('.graph-noeud').length
        };
        snapshot.requests = requests;
        return snapshot;
      }, {lang, theme});
      assert.deepEqual(result.branches, ['transport', 'validation']);
      assert.equal(result.taskCount, 60);
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
      assert.equal(result.requests, 0);
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
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
}

run().catch(error => { console.error(error); process.exitCode = 1; });
