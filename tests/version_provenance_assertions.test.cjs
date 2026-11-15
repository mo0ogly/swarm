'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {versionProvenanceMatches}=require('./helpers/version_provenance_assertions.cjs');
const link='https://github.com/example/project/commit/'+'a'.repeat(40);
const facts=modified=>({binary:{modified},commit_url:link});
const rendered=(modified,count=modified?1:0)=>({binary:{modified},hrefs:[link],dirtyNoticeCount:count});

test('modified control and clean control both match explicit build facts',()=>{
 assert.equal(versionProvenanceMatches(facts(true),rendered(true)),true);
 assert.equal(versionProvenanceMatches(facts(false),rendered(false)),true);
});
test('opposite metadata and missing or spurious dirty notices are refused',()=>{
 assert.equal(versionProvenanceMatches(facts(false),rendered(true)),false);
 assert.equal(versionProvenanceMatches(facts(true),rendered(false)),false);
 assert.equal(versionProvenanceMatches(facts(true),rendered(true,0)),false);
 assert.equal(versionProvenanceMatches(facts(false),rendered(false,1)),false);
 assert.equal(versionProvenanceMatches(facts(true),rendered(true,2)),false);
 assert.equal(versionProvenanceMatches(facts(false),{...rendered(false),hrefs:[]}),false);
});
test('invalid facts preserve inputs and a subsequent valid observation recovers',()=>{
 for(const value of [null,undefined,'false',0]){
  const expected=facts(value),observed=rendered(false),before=JSON.stringify([expected,observed]);
  assert.throws(()=>versionProvenanceMatches(expected,observed),/expected-modified-boolean-required/);
  assert.equal(JSON.stringify([expected,observed]),before);
  assert.throws(()=>versionProvenanceMatches(facts(false),rendered(value)),/observed-modified-boolean-required/);
 }
 assert.equal(versionProvenanceMatches(facts(false),rendered(false)),true);
 assert.equal(versionProvenanceMatches(facts(true),rendered(true)),true);
});
