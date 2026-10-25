'use strict';
/** Browser -> explicit fixture HTTP bridge -> frozen existing PAEA CLI.
 * This checks actual v2 correspondence rather than invented successful responses.
 * It does not exercise native HTTP admission, embedded assets, installation or an
 * effect service. No compiler, provider, live Root, automatic retry or watchdog.
 * Required: SWARM_PAEA_TEST_BINARY, SWARM_PAEA_TEST_BINARY_SHA256; fresh output argv.
 */
const fs = require('node:fs'), path = require('node:path'), http = require('node:http');
const crypto = require('node:crypto'), assert = require('node:assert/strict');
const {spawn} = require('node:child_process'), puppeteer = require('puppeteer');
const output = process.argv[2], binary = process.env.SWARM_PAEA_TEST_BINARY;
const expectedHash = process.env.SWARM_PAEA_TEST_BINARY_SHA256;
if (!output || process.argv.length !== 3 || !binary || !/^[a-f0-9]{64}$/.test(expectedHash || '')) {
  throw Error('Supply fresh output, explicit existing binary and SHA256');
}
const sha = raw => crypto.createHash('sha256').update(raw).digest('hex');
assert.equal(sha(fs.readFileSync(binary)), expectedHash); fs.mkdirSync(output);
const checks = [], calls = [], errors = [];
const csrf = 'a'.repeat(64), vectors = JSON.parse(fs.readFileSync('tests/fixtures/paea/contract-v1.json')).vectors;
const assets = new Map(['/paea.html','/paea.js','/paea.css','/i18n-en.js','/i18n.js',
  '/swarm-design.css','/wattson_themes.css','/swarm-logo.png'].map(url => [url, 'web' + url]));
let server, browser;
/** Emit named observable checks while browser/compiler watchdogs stay absent. */
function check(name, value) { assert(value, name); checks.push(name); console.log('PASS ' + name); }
/** Capture actual streamed subprocess bytes and errors without a hidden maxBuffer.
 * An exact fixed operation receives two owned file paths and explicit data fields.
 * No shell, alternate operation, environment dump or live cwd is permitted.
 */
async function invoke(body, language) {
  const directory = fs.mkdtempSync(path.join(path.resolve(output), 'cli-'));
  fs.writeFileSync(path.join(directory,'action.json'),body.document);
  fs.writeFileSync(path.join(directory,'receipt.json'),body.receipt);
  const args = ['protection','receipt-report','action.json','receipt.json',body.expected_sink,body.previous_state,'--json','--lang',language];
  const result = await new Promise((resolve,reject) => {
    const process = spawn(binary,args,{cwd:directory,stdio:['ignore','pipe','pipe']});
    const stdout = [], stderr = [];
    process.stdout.on('data',chunk => stdout.push(chunk)); process.stderr.on('data',chunk => stderr.push(chunk));
    process.on('error',reject); process.on('close',(exit,signal) => resolve({exit,signal,stdout:Buffer.concat(stdout),stderr:Buffer.concat(stderr)}));
  });
  assert.deepEqual(fs.readdirSync(directory).sort(),['action.json','receipt.json']);
  assert.equal(fs.readFileSync(path.join(directory,'action.json'),'utf8'),body.document);
  assert.equal(fs.readFileSync(path.join(directory,'receipt.json'),'utf8'),body.receipt);
  calls.push({argv:[binary,...args],cwd:directory,request:body,exit:result.exit,signal:result.signal,
    stdout_hex:result.stdout.toString('hex'),stderr_hex:result.stderr.toString('hex')});
  assert.equal(result.signal,null); assert([0,2].includes(result.exit));
  if (result.exit === 0) { assert.equal(result.stderr.length,0); return {status:200,raw:result.stdout}; }
  assert.equal(result.stdout.length,0); return {status:400,raw:result.stderr};
}
/** A deliberately separate test transport serves workspace bytes and forwards
 * one closed JSON envelope to the CLI. Its CSRF stub is not native admission.
 * Callback failures remain failed HTTP/test results, never successful empty data.
 */
function serve(request,response) {
  const url = new URL(request.url,'http://localhost');
  if (url.pathname === '/api/v1/native-session') {
    response.setHeader('Content-Type','application/json');
    response.end(JSON.stringify({schema_version:1,csrf_token:csrf})); return;
  }
  if (url.pathname === '/api/v1/protection/receipt-report') {
    const chunks = []; request.on('data',chunk => chunks.push(chunk));
    request.on('end',async () => {
      try {
        assert.equal(request.method,'POST'); assert.equal(request.headers['x-swarm-csrf'],csrf);
        const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        assert.deepEqual(Object.keys(body).sort(),['document','expected_sink','previous_state','receipt','schema_version']);
        assert.equal(body.schema_version,1);
        for (const key of ['document','receipt','expected_sink','previous_state']) assert.equal(typeof body[key],'string');
        const result = await invoke(body,url.searchParams.get('lang') || 'en');
        response.statusCode = result.status; response.setHeader('Content-Type','application/json'); response.end(result.raw);
      } catch (error) { errors.push(error.stack); response.statusCode = 500; response.end('{"failure":{"code":"fixture-bridge-failed"}}'); }
    }); return;
  }
  if (assets.has(url.pathname)) {
    response.setHeader('Content-Type',{'.html':'text/html; charset=utf-8','.js':'application/javascript','.css':'text/css','.png':'image/png'}[path.extname(url.pathname)]);
    response.end(fs.readFileSync(assets.get(url.pathname))); return;
  }
  response.statusCode = 404; response.end('fixture-route-unknown');
}
/** Fill a real control and dispatch input so stale reports are invalidated. */
async function field(page,id,value) {
  await page.$eval('#'+id,(element,text) => { element.value=text; element.dispatchEvent(new Event('input',{bubbles:true})); },value);
}
/** Observe both HTTP completion and visible UI state after an actual button click. */
async function submit(page,status,code) {
  const pending = page.waitForResponse(response => response.url().includes('/api/v1/protection/receipt-report') && response.request().method() === 'POST');
  await page.click('#paea-check'); const response = await pending;
  const raw = await response.text(), value = JSON.parse(raw);
  await page.waitForFunction(() => !document.querySelector('#paea-check').disabled);
  check('HTTP-'+status,response.status()===status);
  if (status===200) {
    check('actual-checker-visible-success',await page.$eval('#paea-status',element => element.dataset.state==='success'));
    check('actual-checker-no-authority',value.authority==='not-established' && value.effect==='not-established' && value.receipt_authenticity==='not-established');
  } else {
    check('actual-checker-'+code,value.failure.code===code);
    check('actual-checker-no-stale-report',await page.$eval('#paea-download',element => element.disabled));
    check('actual-checker-visible-refusal',await page.$eval('#paea-status',element => element.dataset.state==='error'));
  }
  return value;
}
/** Assert all report fields against selected bytes and the independent frozen vector. */
function expectedReport(value,document,receipt,vector,previous,outcome) {
  assert.deepEqual(value,{schema_version:2,scope:'offline-data-only',document:{document_sha256:sha(document),
    action_digest:vector.action_digest,intent_id:vector.intent_id},receipt_sha256:sha(receipt),previous_state:previous,
    proposed_state:outcome,authority:'not-established',receipt_authenticity:'not-established',effect:'not-established'});
}
(async () => {
  try {
    server=http.createServer(serve); await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const base='http://127.0.0.1:'+server.address().port;
    browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(path.resolve(output),'browser')});
    const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
    page.on('pageerror',error=>errors.push(error.message)); page.on('requestfailed',request=>errors.push(request.url()+' '+request.failure()?.errorText));
    for (const language of ['fr','en']) for (const index of [0,8]) {
      await page.goto(base+'/paea.html?lang='+language,{waitUntil:'networkidle0'});
      await page.waitForFunction(()=>!document.querySelector('#paea-check').disabled);
      const vector=vectors[index],document=JSON.stringify({proposal:vector.proposal,context:vector.context});
      const receipt={schema_version:1,action_digest:vector.action_digest,intent_id:vector.intent_id,sink_id:'sink-1',outcome:'COMMITTED',observation_digest:'d'.repeat(64)};
      const documentPath=path.join(path.resolve(output),language+'-'+index+'-action.json'),receiptPath=path.join(path.resolve(output),language+'-'+index+'-receipt.json');
      fs.writeFileSync(documentPath,document);fs.writeFileSync(receiptPath,JSON.stringify(receipt));
      await (await page.$('#paea-document')).uploadFile(documentPath);await (await page.$('#paea-receipt')).uploadFile(receiptPath);
      await field(page,'paea-sink','sink-1');await page.select('#paea-state','COMMIT_REQUESTED');
      const baseline=await submit(page,200);expectedReport(baseline,document,JSON.stringify(receipt),vector,'COMMIT_REQUESTED','COMMITTED');
      check(language+'-'+index+'-exact-checker-report',true);
      await field(page,'paea-sink','wrong-sink');await submit(page,400,'protection-receipt-binding');
      await field(page,'paea-sink','sink-1');await submit(page,200);
      receipt.observation_digest='e'.repeat(64);fs.writeFileSync(receiptPath,JSON.stringify(receipt));await (await page.$('#paea-receipt')).uploadFile(receiptPath);
      const observation=await submit(page,200);expectedReport(observation,document,JSON.stringify(receipt),vector,'COMMIT_REQUESTED','COMMITTED');
      check(language+'-'+index+'-observation-hash-distinct',baseline.receipt_sha256!==observation.receipt_sha256);
      receipt.outcome='UNKNOWN';fs.writeFileSync(receiptPath,JSON.stringify(receipt));await (await page.$('#paea-receipt')).uploadFile(receiptPath);
      const unknown=await submit(page,200);expectedReport(unknown,document,JSON.stringify(receipt),vector,'COMMIT_REQUESTED','UNKNOWN');
      check(language+'-'+index+'-actual-unknown-visible',await page.$eval('#paea-summary',element=>element.textContent.includes('UNKNOWN')));
      await page.screenshot({path:path.join(output,language+'-'+index+'-unknown.png'),fullPage:true});
      receipt.outcome='RECONCILED_COMMITTED';fs.writeFileSync(receiptPath,JSON.stringify(receipt));await (await page.$('#paea-receipt')).uploadFile(receiptPath);await page.select('#paea-state','UNKNOWN');
      expectedReport(await submit(page,200),document,JSON.stringify(receipt),vector,'UNKNOWN','RECONCILED_COMMITTED');
      receipt.outcome='COMMITTED';fs.writeFileSync(receiptPath,JSON.stringify(receipt));await (await page.$('#paea-receipt')).uploadFile(receiptPath);await page.select('#paea-state','COMMIT_REQUESTED');await submit(page,200);
      await page.select('#paea-state','UNKNOWN');await submit(page,400,'protection-transition-invalid');
      await page.select('#paea-state','COMMIT_REQUESTED');await submit(page,200);
      fs.writeFileSync(receiptPath,'{"PRIVATE-CANARY-NOT-A-PERMISSION":');await (await page.$('#paea-receipt')).uploadFile(receiptPath);await submit(page,400,'protection-document-invalid');
      fs.writeFileSync(receiptPath,JSON.stringify(receipt));await (await page.$('#paea-receipt')).uploadFile(receiptPath);await submit(page,200);
      const formatted=JSON.stringify({proposal:vector.proposal,context:vector.context},null,2);fs.writeFileSync(documentPath,formatted);await (await page.$('#paea-document')).uploadFile(documentPath);
      const whitespace=await submit(page,200);expectedReport(whitespace,formatted,JSON.stringify(receipt),vector,'COMMIT_REQUESTED','COMMITTED');
      check(language+'-'+index+'-actual-input-identity-distinct',baseline.document.document_sha256!==whitespace.document.document_sha256 && baseline.document.action_digest===whitespace.document.action_digest);
    }
    check('existing-binary-stable',sha(fs.readFileSync(binary))===expectedHash);
    check('no-bridge-browser-errors',errors.length===0);
    console.log('PAEA_REAL_CLI_UI_PASS');
  } finally {
    if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));
    fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({scope:'workspace-browser -> fixture HTTP bridge -> existing CLI; no native API or effect assurance',binary,binary_sha256:expectedHash,checks,calls,errors},null,2));
  }
})().catch(error=>{console.error(error.stack);process.exitCode=1});
