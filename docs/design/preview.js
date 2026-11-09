/* Local preview adapter for these trusted Claude Design files. No network,
 * storage, provider or Swarm API calls. Expressions execute local template code.
 * Unsupported actions report a simulation instead of pretending to persist. */
(() => {
  'use strict';
  document.addEventListener('DOMContentLoaded', () => {
    const host = document.querySelector('x-dc');
    const script = document.querySelector('[data-dc-script]');
    if (!host || !script) return;
    const template = host.cloneNode(true);
    const evaluate = (expression, scope) => Function('scope', 'with(scope){return (' + expression + ')}')(scope);
    let render;
    class DCLogic {
      constructor(props) { this.props = props; this.state = {}; }
      /** Merge demo state and redraw; no persistence or external side effects. */
      setState(next) { this.state = Object.assign({}, this.state, next); render(); }
    }
    const Component = Function('DCLogic', script.textContent + '\nreturn Component;')(DCLogic);
    const component = new Component({ theme: 'sombre' });
    const exact = /^\{\{([^{}]*?)\}\}$/;
    const value = (input, scope) => exact.test(input) ? evaluate(input.match(exact)[1], scope) : input.replace(/\{\{([\s\S]*?)\}\}/g, (_, expr) => String(evaluate(expr, scope) ?? ''));
    /** Expand the loops, conditions and bound events used by the four boards.
     * Fails visibly on unsupported expressions; never treats errors as success. */
    function expand(node, scope) {
      if (node.nodeType === Node.TEXT_NODE) { node.textContent = value(node.textContent, scope); return; }
      if (node.nodeType !== Node.ELEMENT_NODE) return;
      if (node.localName === 'sc-for') {
        const fragment = document.createDocumentFragment();
        for (const item of value(node.getAttribute('list'), scope)) {
          const childScope = Object.assign({}, scope, { [node.getAttribute('as')]: item });
          for (const child of [...node.childNodes]) { const clone = child.cloneNode(true); fragment.append(clone); expand(clone, childScope); }
        }
        node.replaceWith(fragment); return;
      }
      if (node.localName === 'sc-if') {
        if (!value(node.getAttribute('value'), scope)) { node.remove(); return; }
        const children = [...node.childNodes]; node.replaceWith(...children); children.forEach(child => expand(child, scope)); return;
      }
      for (const attr of [...node.attributes]) {
        if (!attr.value.includes('{{')) continue;
        const result = value(attr.value, scope);
        if (attr.name.startsWith('on')) {
          node.removeAttribute(attr.name);
          if (typeof result !== 'function') throw new Error('Missing demo handler: ' + attr.name);
          node.addEventListener(attr.name.slice(2).toLowerCase(), event => { event.preventDefault(); result(event); });
        } else if (['disabled', 'checked', 'required'].includes(attr.name)) {
          node.toggleAttribute(attr.name, !!result); node[attr.name] = !!result;
        } else {
          node.setAttribute(attr.name, String(result ?? ''));
          if (attr.name === 'value') node.value = result ?? '';
        }
      }
      [...node.childNodes].forEach(child => expand(child, scope));
    }
    const notice = document.createElement('p'); notice.className = 'preview-notice'; notice.setAttribute('role', 'status'); notice.hidden = true;
    const controls = document.createElement('nav'); controls.className = 'preview-bar'; controls.setAttribute('aria-label', 'Prévisualisation des maquettes');
    controls.innerHTML = '<strong>Swarm <span>· atelier UX</span></strong><a href="../swarm/Main.dc.html">Cockpit</a><a href="../swarm/Couloirs.dc.html">Couloirs</a><a href="../swarm/Preparation.dc.html">Préparation</a><a href="../swarm/Securite.dc.html">Sécurité</a><a href="../swarm/Audit.dc.html">Audit</a><a href="../admin/Main.dc.html">Admin · référence</a><a href="../admin/Proposition.dc.html">Admin · proposition</a>';
    const theme = document.createElement('button'); theme.type = 'button'; controls.append(theme);
    const announce = text => { notice.textContent = text; notice.hidden = false; };
    theme.addEventListener('click', () => { component.props.theme = component.props.theme === 'sombre' ? 'clair' : 'sombre'; render(); });
    document.body.prepend(controls); document.body.append(notice);
    /** Preserve active editable control/caret, scroll and disclosures on redraw. */
    render = () => {
      const active = document.activeElement;
      const inputs = [...host.querySelectorAll('input,textarea,select')];
      const focusedIndex = inputs.indexOf(active);
      const selection = active && /^(textarea|input)$/.test(active.localName) && active.type !== 'number' ? [active.selectionStart, active.selectionEnd] : null;
      const open = [...host.querySelectorAll('details')].map(detail => detail.open);
      const scrolls = [...host.querySelectorAll('*')].map((element, index) => [index, element.scrollLeft, element.scrollTop]).filter(entry => entry[1] || entry[2]);
      const clone = template.cloneNode(true);
      try { expand(clone, component.renderVals()); }
      catch (error) { announce('Erreur de prévisualisation : ' + error.message); throw error; }
      host.replaceChildren(...clone.childNodes);
      host.querySelectorAll('details').forEach((detail, i) => { if (i < open.length) detail.open = open[i]; });
      const elements = host.querySelectorAll('*'); scrolls.forEach(([i,x,y]) => { if (elements[i]) elements[i].scrollTo(x,y); });
      if (focusedIndex >= 0) { const next = host.querySelectorAll('input,textarea,select')[focusedIndex]; if (next) { next.focus({ preventScroll: true }); if (selection) next.setSelectionRange(...selection); } }
      host.querySelectorAll('form').forEach(form => form.addEventListener('submit', event => { event.preventDefault(); announce('Démonstration : votre texte reste local. Aucun message envoyé.'); }));
      host.querySelectorAll('button:not([onclick])').forEach(button => {
        // Bound template handlers are already registered; generic actions only
        // explain the preview and never change the simulated application state.
        if (!button.outerHTML.includes('{{')) button.addEventListener('click', () => {
          const label = button.textContent.trim();
          if (/^(Accepter|Renvoyer|Ouvrir la session|Voir les preuves|Suspendre|Rechercher|Exporter|RETEX|Joindre|Examiner le brief|Restaurer|Illustrer|Comprendre)/.test(label)) announce('Démonstration — « ' + label + ' ». Aucune action dans Swarm.');
        });
      });
      theme.textContent = component.props.theme === 'sombre' ? 'Passer en clair' : 'Passer en sombre';
      document.documentElement.dataset.theme = component.props.theme;
      controls.querySelectorAll('a').forEach(link => { if (link.href === location.href) link.setAttribute('aria-current', 'page'); });
    };
    render();
  });
})();
