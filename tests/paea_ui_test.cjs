'use strict';
/** Browser component checks over real workspace assets and an explicit HTTP double.
 * No Rust compilation, native server, live Root or protected effect is exercised.
 * Pass a fresh output directory; retain screenshots, named assertions and requests.
 * Browser watchdogs are disabled; progress is emitted after each meaningful check.
 */
const fs = require('node:fs'), path = require('node:path'), http = require('node:http');
const assert = require('node:assert/strict'), crypto = require('node:crypto');
const puppeteer = require('puppeteer');
const output = process.argv[2];
if (!output || process.argv.length !== 3) throw Error('Usage: node tests/paea_ui_test.cjs FRESH_OUTPUT');
fs.mkdirSync(output);
const checks = [], requests = [], errors = [];
const vector = JSON.parse(fs.readFileSync('tests/fixtures/paea/contract-v1.json')).vectors[0];
const documentRaw = JSON.stringify({proposal: vector.proposal, context: vector.context});
const receiptRaw = JSON.stringify({schema_version: 1, action_digest: vector.action_digest,
  intent_id: vector.intent_id, sink_id: 'sink-1', outcome: 'COMMITTED', observation_digest: 'd'.repeat(64)});
const sha = raw => crypto.createHash('sha256').update(raw).digest('hex');
const csrf = 'a'.repeat(64);
const assets = new Map(['/paea.html', '/paea.js', '/paea.css', '/i18n-en.js', '/i18n.js',
  '/swarm-design.css', '/wattson_themes.css', '/swarm-logo.png'].map(url => [url, 'web' + url]));
let scenario = 'valid', server, browser, releaseResponse, markReceived;
/** Record an observable assertion, allowing diagnosis while no global timer runs. */
function check(name, value) { assert(value, name); checks.push(name); console.log('PASS ' + name); }
/** Compute WCAG contrast for opaque rendered text/background pairs. These are
 * browser-computed colors, not an assertion that a CSS token name is compliant. */
function contrast(foreground, background) {
  const luminance = color => color.match(/[\d.]+/g).slice(0,3).map(Number)
    .map(channel => channel / 255).map(channel => channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4)
    .reduce((sum, channel, index) => sum + channel * [.2126,.7152,.0722][index],0);
  const a = luminance(foreground), b = luminance(background);
  return (Math.max(a,b) + .05) / (Math.min(a,b) + .05);
}
/** An explicit fake response tests UI validation only, never library correctness. */
function report(body) {
  return {schema_version: 2, scope: 'offline-data-only', document: {
    document_sha256: sha(body.document), action_digest: vector.action_digest, intent_id: vector.intent_id},
    receipt_sha256: sha(body.receipt), previous_state: body.previous_state,
    proposed_state: scenario === 'unknown' ? 'UNKNOWN' : 'COMMITTED',
    authority: 'not-established', receipt_authenticity: 'not-established', effect: 'not-established'};
}
/** Serve only named canonical assets and one HTTP double; no disk path comes from inputs. */
function serve(request, response) {
  const url = new URL(request.url, 'http://localhost');
  if (url.pathname === '/api/v1/native-session') {
    response.setHeader('Content-Type', 'application/json');
    if (scenario === 'session-unavailable') { response.statusCode = 403; response.end('{}'); return; }
    response.end(JSON.stringify({schema_version: 1, csrf_token: csrf})); return;
  }
  if (url.pathname === '/api/v1/protection/receipt-report') {
    let raw = ''; request.setEncoding('utf8'); request.on('data', chunk => { raw += chunk; });
    request.on('end', () => {
      requests.push({method: request.method, csrf: request.headers['x-swarm-csrf'], raw, scenario});
      const body = JSON.parse(raw);
      response.setHeader('Content-Type', 'application/json');
      if (scenario === 'refused') { response.statusCode = 400; response.end(JSON.stringify({failure: {code: 'protection-receipt-binding'}})); return; }
      if (scenario === 'malformed') { response.end('{'); return; }
      const value = report(body);
      if (scenario === 'wrong-hash') value.receipt_sha256 = '0'.repeat(64);
      if (scenario === 'authority') value.authority = 'established';
      if (scenario === 'delayed') { releaseResponse = () => response.end(JSON.stringify(value)); markReceived(); return; }
      response.end(JSON.stringify(value));
    }); return;
  }
  if (assets.has(url.pathname)) {
    const extension = path.extname(url.pathname);
    response.setHeader('Content-Type', {'.html':'text/html; charset=utf-8', '.js':'application/javascript', '.css':'text/css', '.png':'image/png'}[extension]);
    response.end(fs.readFileSync(assets.get(url.pathname))); return;
  }
  response.statusCode = 404; response.end('fixture-route-unknown');
}
/** Fill normal file controls with retained fixture bytes, without interpreting JSON numbers. */
async function selectInputs(page, documentPath, receiptPath) {
  await (await page.$('#paea-document')).uploadFile(documentPath);
  await (await page.$('#paea-receipt')).uploadFile(receiptPath);
  await page.$eval('#paea-sink', element => { element.value = 'sink-1'; element.dispatchEvent(new Event('input', {bubbles:true})); });
  await page.select('#paea-state', 'COMMIT_REQUESTED');
}
/** Submit through the real button, awaiting visible successful or refused completion. */
async function submit(page, expected) {
  await page.click('#paea-check');
  await page.waitForFunction(() => !document.querySelector('#paea-check').disabled);
  const status = await page.$eval('#paea-status', element => element.textContent);
  check(expected, await page.$eval('#paea-status', (element, text) => element.dataset.state === text, expected));
  return status;
}
(async () => {
  try {
    // A missing view is the original public-entry regression before implementation.
    for (const file of assets.values()) fs.accessSync(file);
    fs.writeFileSync(path.join(output, 'action.json'), documentRaw);
    fs.writeFileSync(path.join(output, 'receipt.json'), receiptRaw);
    fs.writeFileSync(path.join(output, 'invalid-utf8.json'), Buffer.from([0xff, 0x7b, 0x7d]));
    server = http.createServer(serve);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const base = 'http://127.0.0.1:' + server.address().port;
    browser = await puppeteer.launch({headless: true, executablePath: process.env.CHROME_BIN || '/opt/google/chrome/chrome',
      timeout: 0, protocolTimeout: 0, args: ['--no-sandbox'], userDataDir: path.join(output, 'browser')});
    const page = await browser.newPage(); page.setDefaultTimeout(0); page.setDefaultNavigationTimeout(0);
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error' && !['refused','session-unavailable'].includes(scenario)) errors.push(message.text()); });
    page.on('requestfailed', request => errors.push(request.url() + ' ' + request.failure()?.errorText));
    const documentPath = path.resolve(output, 'action.json'), receiptPath = path.resolve(output, 'receipt.json');
    for (const lang of ['fr', 'en']) for (const theme of ['etat', 'sombre']) for (const width of [1440, 1024, 760, 390, 320]) {
      await page.setViewport({width, height: 1050});
      await page.goto(base + '/paea.html?lang=' + lang + '&theme=' + theme, {waitUntil:'networkidle0'});
      await page.waitForFunction(() => !document.querySelector('#paea-check').disabled);
      check(`${lang}-${theme}-${width}-empty`, await page.$eval('#paea-status', element => element.dataset.state === 'empty'));
      check(`${lang}-${theme}-${width}-heading`, (await page.$eval('h1', element => element.textContent)) ===
        (lang === 'fr' ? 'Vérifications PAEA' : 'PAEA checks'));
      check(`${lang}-${theme}-${width}-theme`, await page.$eval('html', (element, value) => element.dataset.theme === value, theme));
      check(`${lang}-${theme}-${width}-independent-cockpit-navigation`, await page.$eval('nav a', element =>
        element.textContent === 'Cockpit' && element.getAttribute('href') === '/'));
      await selectInputs(page, documentPath, receiptPath);
      scenario = 'valid'; await submit(page, 'success');
      const colors = await page.$eval('#paea-check', element => ({text:getComputedStyle(element).color,background:getComputedStyle(element).backgroundColor}));
      check(`${lang}-${theme}-${width}-primary-contrast-AA`, contrast(colors.text, colors.background) >= 4.5);
      check(`${lang}-${theme}-${width}-exact-request`, JSON.parse(requests.at(-1).raw).document === documentRaw &&
        JSON.parse(requests.at(-1).raw).receipt === receiptRaw && requests.at(-1).csrf === csrf);
      check(`${lang}-${theme}-${width}-no-overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      check(`${lang}-${theme}-${width}-visible-limits`, (await page.$eval('#paea-limits', element => element.textContent)).includes(lang === 'fr' ? 'non établis' : 'not established'));
      await page.screenshot({path: path.join(output, `${lang}-${theme}-${width}.png`), fullPage:true});
    }
    // Valid control precedes each malformed/refused response, then real button recovery.
    for (const fault of ['refused', 'malformed', 'wrong-hash', 'authority']) {
      scenario = 'valid'; await submit(page, 'success'); scenario = fault;
      await submit(page, 'error');
      check(fault + '-no-report-download', await page.$eval('#paea-download', element => element.disabled));
      check(fault + '-retains-inputs', await page.$eval('#paea-document', element => element.files.length === 1));
      scenario = 'valid'; await submit(page, 'success');
    }
    scenario = 'unknown'; await submit(page, 'success');
    check('unknown-visible-with-limits', await page.$eval('#paea-summary', element => element.textContent.includes('UNKNOWN')));
    scenario = 'valid'; await submit(page, 'success');
    const count = requests.length;
    await (await page.$('#paea-document')).uploadFile(path.resolve(output, 'invalid-utf8.json'));
    check('edit-clears-report', await page.$eval('#paea-download', element => element.disabled));
    await submit(page, 'error'); check('invalid-utf8-never-sent', requests.length === count);
    await selectInputs(page, documentPath, receiptPath); await submit(page, 'success');
    // UTF-8 BOM/non-ASCII and an integer beyond JS precision remain original bytes.
    const exactRaw = '\ufeff' + documentRaw.replace('"420000"','"9007199254740993"') + '\n é';
    fs.writeFileSync(path.join(output,'exact-utf8.json'),exactRaw);
    await (await page.$('#paea-document')).uploadFile(path.resolve(output,'exact-utf8.json'));
    await submit(page,'success');
    check('raw-utf8-bom-and-large-integer-retained', JSON.parse(requests.at(-1).raw).document === exactRaw);
    await selectInputs(page,documentPath,receiptPath); await submit(page,'success');
    // Hold an actual HTTP reply at a barrier: one request, disabled inputs,
    // no stale downloadable success and no duplicate request from another submit.
    scenario = 'delayed'; const received = new Promise(resolve => { markReceived = resolve; });
    const initialCount = requests.length; await page.click('#paea-check'); await received;
    check('loading-disables-inputs-and-old-report', await page.evaluate(() =>
      document.querySelector('#paea-inputs').disabled && document.querySelector('#paea-download').disabled &&
      document.querySelector('#paea-status').dataset.state === 'loading'));
    await page.$eval('#paea-form', form => form.requestSubmit());
    check('duplicate-gesture-no-second-request', requests.length === initialCount + 1);
    releaseResponse(); await page.waitForFunction(() => !document.querySelector('#paea-check').disabled);
    scenario = 'valid'; await submit(page,'success');
    await page.focus('#paea-check'); await page.keyboard.press('Tab');
    check('keyboard-reaches-download', await page.evaluate(() => document.activeElement.id === 'paea-download'));
    const session = await page.createCDPSession();
    // Wait on the download completion event rather than an arbitrary sleep.
    await session.send('Browser.setDownloadBehavior', {behavior:'allow', downloadPath:path.resolve(output), eventsEnabled:true});
    const completed = new Promise(resolve => session.on('Browser.downloadProgress', event => { if (event.state === 'completed') resolve(); }));
    await page.click('#paea-download'); await completed;
    check('download-input-bound-report', JSON.parse(fs.readFileSync(path.join(output, 'paea-receipt-report.json'))).receipt_sha256 === sha(receiptRaw));
    scenario = 'session-unavailable';
    await page.reload({waitUntil:'networkidle0'});
    check('session-failure-blocks-checks', await page.$eval('#paea-check', element => element.disabled));
    check('session-failure-visible', await page.$eval('#paea-status', element => element.dataset.state === 'error'));
    scenario = 'valid'; await page.reload({waitUntil:'networkidle0'});
    await page.waitForFunction(() => !document.querySelector('#paea-check').disabled);
    await selectInputs(page,documentPath,receiptPath); await submit(page,'success');
    check('no-browser-exceptions', errors.length === 0);
  } finally {
    if (browser) await browser.close();
    if (server) await new Promise(resolve => server.close(resolve));
    fs.writeFileSync(path.join(output, 'result.json'), JSON.stringify({scope:'browser-component-http-double', checks, requests, errors}, null, 2));
  }
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
