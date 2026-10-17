'use strict';
// The parent retains the exact API page for export. The frame only presents it.
globalThis.JournalConsole = (() => {
  const tr = source => globalThis.SwarmI18n?.t(source) ?? source;
  let frame, ready = false, page = null;
  const send = () => {
    if (ready && frame?.contentWindow) frame.contentWindow.postMessage({type:'swarm-log-page', ...page, theme:document.documentElement.dataset.theme}, location.origin);
  };
  function display(entries, options = {}) {
    page = {entries, ...options};
    const fallback = document.getElementById('log-lines');
    fallback.textContent = entries.map(e => '#'+(e.seq ?? e.rank)+' '+e.at+' ['+e.kind+'] '+(e.label ? e.label+' · ' : '')+e.message).join('\n') || tr('Aucune ligne correspondante.');
    if (!frame) {
      frame = document.createElement('iframe');
      frame.id = 'journal-console-frame';
      frame.title = tr('Console des journaux, lecture seule');
      frame.src = '/log-console.html?lang='+encodeURIComponent(globalThis.SwarmI18n?.language || 'fr');
      frame.hidden = true;
      document.getElementById('journal-console-wrap').prepend(frame);
    }
    send();
  }
  addEventListener('message', event => {
    if (event.origin !== location.origin || event.source !== frame?.contentWindow || event.data?.type !== 'swarm-log-ready') return;
    ready = !!event.data.monaco;
    document.getElementById('journal-console-mode').dataset.diagnostic=event.data.diagnostic||'';
    frame.hidden = !ready;
    document.getElementById('log-lines').hidden = ready;
    document.getElementById('journal-console-mode').textContent = ready
      ? tr('Lecture seule · Ctrl+F pour rechercher dans la console')
      : tr('Coloration indisponible : affichage texte conservé.');
    send();
  });
  new MutationObserver(send).observe(document.documentElement, {attributes:true, attributeFilter:['data-theme']});
  return {display, supervision(planning, decisions) {
    const rows=(planning?.inbox||[]).map(e=>({rank:e.id,at:e.at,kind:e.kind,origin:e.scope,label:e.decision?tr('Incident acquitté · résolution à vérifier'):tr('Retour reçu · décision attendue'),message:e.message+'\n'+JSON.stringify({id:e.id,scope:e.scope,decision:e.decision||null,result:e.result||null},null,2)}));
    for(const d of decisions||[])rows.push({rank:d.id,at:d.at,kind:'supervisor.decision',origin:d.scope,label:tr('Décision du superviseur'),message:d.reason+'\n'+JSON.stringify({id:d.id,scope:d.scope,operations:d.operations},null,2)});
    return rows.sort((a,b)=>b.at.localeCompare(a.at)||b.rank.localeCompare(a.rank));
  }, clear() {display([], {follow:false});}, search() {if(ready)frame.contentWindow.postMessage({type:'swarm-log-search'},location.origin);else document.getElementById('log-search').focus();}};
})();
