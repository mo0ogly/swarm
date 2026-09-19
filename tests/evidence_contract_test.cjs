#!/usr/bin/env node
'use strict';

const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const source=fs.readFileSync('web/evidence-contract.js','utf8')+';SwarmEvidenceContract';
const contract=vm.runInNewContext(source);
const evidence={attempt_id:'attempt-7',revision:12,freshness:'fresh',observed_at:'2026-09-19T10:00:00Z',
 report_review:{state:'passed',attempt_id:'attempt-7',reviewer:'reviewer://fixture',at:'2026-09-19T09:59:00Z',limits:['review only']},
 controls:{state:'passed',items:[{id:'npm-test',attempt_id:'attempt-7',execution:'executed',revision:'11',command:['npm','test'],exit_code:0,started_at:'2026-09-19T09:58:00Z',finished_at:'2026-09-19T09:58:02Z',freshness:'fresh',result:'passed',limits:['digest only']}]},
 acceptance:{state:'accepted',revision:'unknown',at:'2026-09-19T10:00:00Z'},limits:['unknown is not success']};
const element={dataset:{},textContent:''};
contract.render(element,evidence,s=>s);
assert.deepEqual({...element.dataset},{evidenceFreshness:'fresh',controlsState:'passed',acceptanceState:'accepted'});
for(const expected of ['attempt-7','Révision lue : 12','Revue du rapport : passed','npm test','code de sortie 0','2026-09-19T09:58:02Z','Acceptation : accepted','Limite : review only'])assert.match(element.textContent,new RegExp(expected));

const claimed=structuredClone(evidence);claimed.controls={state:'unknown',items:[{id:'npm-test',execution:'unknown',revision:'unknown',command:[],exit_code:null,started_at:'unknown',finished_at:'unknown',freshness:'fresh',result:'unknown',limits:['report citation only']}]};claimed.acceptance={state:'pending',revision:'unknown',at:'unknown'};
const unknown={dataset:{},textContent:''};contract.render(unknown,claimed,s=>s);
assert.match(unknown.textContent,/commande unknown.*code de sortie unknown/);
assert.equal(unknown.dataset.controlsState,'unknown');assert.equal(unknown.dataset.acceptanceState,'pending');
assert.throws(()=>contract.text({...evidence,freshness:'VALIDÉ'}),/unknown evidence freshness/);
console.log('PASS evidence DOM: review, executed controls, acceptance, metadata, explicit unknown');
