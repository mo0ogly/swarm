/** Native read hook for the existing Version and news dialog. Protocol shape
 * validation only; backend model/actor/source authenticity is not established. */
'use strict';
(() => {
  const HEX40 = /^[0-9a-f]{40}$/;
  const isoDate = (value) => typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value);
  const langText = (value) =>
    !!value && typeof value === 'object' &&
    typeof value.fr === 'string' && value.fr.length > 0 &&
    typeof value.en === 'string' && value.en.length > 0;
  const safeUrl = (value) =>
    typeof value === 'string' && (() => { try { const u = new URL(value); return u.protocol === 'https:' && u.hostname === 'github.com' && !u.username && !u.password; } catch { return false; } })();

  const fail = (code) => { const e = new Error(code); e.code = code; return e; };

  /** Require the real native envelope and explicit nullable compiled metadata.
   * Representation checks cannot attest build authenticity or local Git state. */
  function validateBinary(raw) {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) throw fail('version-contract');
    if (raw.schema_version !== 1) throw fail('version-schema-unsupported');
    const b = raw.binary;
    if (!b || typeof b !== 'object' || Array.isArray(b)) throw fail('version-contract');
    if (typeof b.version !== 'string' || b.version.length === 0) throw fail('version-contract');
    if (b.commit !== null && !(typeof b.commit === 'string' && HEX40.test(b.commit))) throw fail('version-contract');
    if (b.modified !== null && typeof b.modified !== 'boolean') throw fail('version-contract');
    if (b.build_date !== null && typeof b.build_date !== 'string') throw fail('version-contract');
    if (typeof b.provenance !== 'string' || b.provenance.length === 0) throw fail('version-contract');
    return { version: b.version, commit: b.commit, modified: b.modified, build_date: b.build_date, provenance: b.provenance };
  }

  /** Validate embedded FR/EN notes and retained releases before the shared
   * renderer consumes them. Missing notes never become an invented empty list. */
  function validateHistory(raw) {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) throw fail('version-history-contract');
    if (raw.schema_version !== 1) throw fail('version-schema-unsupported');
    const local = raw.local_changes;
    if (!local || typeof local !== 'object' || Array.isArray(local) ||
        !langText(local.summary) || !Array.isArray(local.items) ||
        !local.items.every((item) => item && typeof item === 'object' && langText(item))) {
      throw fail('version-history-contract');
    }
    const releases = raw.releases;
    if (!Array.isArray(releases)) throw fail('version-history-contract');
    for (const release of releases) {
      if (!release || typeof release !== 'object' || Array.isArray(release) ||
          typeof release.version !== 'string' || release.version.length === 0 ||
          !isoDate(release.date) || !langText(release.summary) ||
          typeof release.release_url !== 'string' || !safeUrl(release.release_url) ||
          !Array.isArray(release.commits)) {
        throw fail('version-history-contract');
      }
      for (const commit of release.commits) {
        if (!commit || typeof commit !== 'object' || Array.isArray(commit) ||
            !(typeof commit.sha === 'string' && HEX40.test(commit.sha)) ||
            !safeUrl(commit.url)) {
          throw fail('version-history-contract');
        }
      }
    }
    return {
      available: true,
      local_changes: { summary: local.summary, items: local.items },
      releases: releases.map((release) => ({
        version: release.version,
        date: release.date,
        summary: { fr: release.summary.fr, en: release.summary.en },
        release_url: release.release_url,
        commits: release.commits.map((commit) => ({ sha: commit.sha, url: commit.url })),
      })),
    };
  }

  /** Read only the protected compiled identity/notes. IO/codec/HTTP causes are
   * retained with safe family codes, never raw credential/body text. No retry,
   * timeout, CLI probe, source comparison or runtime-health inference occurs. */
  async function read() {
    let response,payload;
    try { response=await fetch('/api/v1/native-version',{method:'GET',credentials:'same-origin',redirect:'error',headers:{Accept:'application/json'},cache:'no-store'}); }
    catch(cause) { throw new Error('version-network',{cause}); }
    try { payload=await response.json(); }
    catch(cause) { const error=new Error('version-response-invalid',{cause});error.httpStatus=response.status;throw error; }
    if(!response.ok){const error=fail('version-http-'+response.status);error.httpStatus=response.status;error.response=payload;throw error;}
    const binary=validateBinary(payload),history=validateHistory(payload.history);
    return {binary,source:{available:false,observed:false},history};
  }

  globalThis.SwarmNativeVersionHooks = Object.freeze({ read });
})();
