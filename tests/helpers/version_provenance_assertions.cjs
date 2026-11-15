'use strict';
const assert=require('node:assert/strict');

/** Compare Version provenance with explicit expected build facts.
 * Both modified fields must be booleans; null/implicit truthiness is refused.
 * The rendered local-change notice count must be one for a modified build and
 * zero for a clean build. A mismatch returns false; invalid facts throw without
 * changing either object. This does not establish process or session identity.
 */
function versionProvenanceMatches(expected,observed){
 assert.equal(typeof expected?.binary?.modified,'boolean','expected-modified-boolean-required');
 assert.equal(typeof observed?.binary?.modified,'boolean','observed-modified-boolean-required');
 assert.equal(typeof expected.commit_url,'string','expected-commit-url-required');
 assert(Array.isArray(observed.hrefs),'observed-links-required');
 assert(Number.isInteger(observed.dirtyNoticeCount)&&observed.dirtyNoticeCount>=0,'observed-notice-count-required');
 return observed.binary.modified===expected.binary.modified&&observed.hrefs.includes(expected.commit_url)&&observed.dirtyNoticeCount===(expected.binary.modified?1:0);
}
module.exports={versionProvenanceMatches};
