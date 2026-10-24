(function () {
  'use strict';

  /**
   * t(key) — Translate a French source label via SwarmI18n.
   * Input: key string. Side effects: none. Falls back to raw key if SwarmI18n missing.
   */
  function t(s) {
    return (globalThis.SwarmI18n && typeof SwarmI18n.t === 'function') ? SwarmI18n.t(s) : s;
  }

  /**
   * engine(s) — Translate a recorded engine state string via SwarmI18n.
   * Input: recorded state string. Side effects: none. Falls back to raw string.
   */
  function engine(s) {
    return (globalThis.SwarmI18n && typeof SwarmI18n.engine === 'function') ? SwarmI18n.engine(s) : s;
  }

  /**
   * num(v) — Check finite nonnegative number.
   */
  function num(v) {
    return typeof v === 'number' && Number.isFinite(v) && v >= 0;
  }

  /**
   * str(v) — Check non-empty string.
   */
  function str(v) {
    return typeof v === 'string' && v.length > 0;
  }

  /**
   * validate(data) — Validate the complete observation payload.
   * Input: {schema_version:1, work_id, budget:null|WorkBudgetState, agents}. Throws Error('web-native-observation-invalid') if malformed.
   * Side effects: none.
   */
  function validate(data) {
    if (!data || typeof data !== 'object') throw new Error('web-native-observation-invalid');
    if (data.schema_version !== 1 || !str(data.work_id)) throw new Error('web-native-observation-invalid');
    const b = data.budget;
    if (b !== null && (typeof b !== 'object' || Array.isArray(b))) throw new Error('web-native-observation-invalid');
    if (b !== null && (!num(b.limit_usd) || !num(b.reserve_per_launch_usd) || !num(b.committed_usd) || !num(b.reported_usd))) {
      throw new Error('web-native-observation-invalid');
    }
    if (b !== null && (b.work_id !== data.work_id || !Number.isSafeInteger(b.attempts_without_cost) || b.attempts_without_cost < 0)) {
      throw new Error('web-native-observation-invalid');
    }
    if (!Array.isArray(data.agents)) throw new Error('web-native-observation-invalid');
    for (const a of data.agents) {
      if (!a || typeof a !== 'object' || a.work_id !== data.work_id || !str(a.id)) throw new Error('web-native-observation-invalid');
      const fields = ['id', 'work_id', 'task_id', 'attempt_id', 'status', 'provider', 'role', 'parent', 'previous', 'started', 'ended'];
      for (const f of fields) {
        if (typeof a[f] !== 'string') throw new Error('web-native-observation-invalid');
      }
    }
  }

  /**
   * el(tag, className, text) — Create element with optional class and textContent.
   * Side effects: none (returns detached node). Never uses innerHTML.
   */
  function el(tag, cls, text) {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text !== undefined && text !== null) n.textContent = text;
    return n;
  }

  /**
   * box(container) — Empty the container and return it.
   * Side effects: removes all children of container.
   */
  function box(id) {
    const c = document.getElementById(id);
    if (!c) throw new Error('web-native-observation-invalid');
    while (c.firstChild) c.removeChild(c.firstChild);
    return c;
  }

  /**
   * renderBudget(data) — Fill #budget-info with recorded budget values as text.
   * committed labeled as estimate; reported labeled provider-declared unauthenticated,
   * not invoice; missing cost noted as partial. limit_usd=0 and missing policy shown as unlimited; null figures stay unknown.
   * No remaining arithmetic, no invented currency. Throws if container missing.
   */
  function renderBudget(data) {
    const c = box('budget-info');
    const b = data.budget;
    if (b === null) { c.appendChild(el('p','notice info',t('Aucun plafond enregistré : budget illimité. Les montants engagés et déclarés ne sont pas disponibles dans cette observation.'))); return; }
    c.appendChild(el('p', 'notice info', t("Plafond (USD)") + ': ' + (b.limit_usd === 0 ? t("Illimité") : String(b.limit_usd))));
    c.appendChild(el('p', 'notice info', t("Réserve par lancement (USD)") + ': ' + String(b.reserve_per_launch_usd)));
    c.appendChild(el('p', 'notice info', t("Estimations engagées (USD, non facturées)") + ': ' + String(b.committed_usd)));
    c.appendChild(el('p', 'notice info', t("Coûts déclarés par le fournisseur (USD, non authentifiés, hors facture)") + ': ' + String(b.reported_usd)));
    if (b.attempts_without_cost > 0) {
      c.appendChild(el('p', 'notice', t("Tentatives sans coût déclaré") + ': ' + String(b.attempts_without_cost) + ' — ' + t("Total déclaré partiel")));
    }
  }

  /**
   * renderAgents(data) — Fill #agents-list with one card per agent showing literal IDs,
   * recorded status via engine(), provider, role. No live probing implied.
   * Throws if container missing.
   */
  function renderAgents(data) {
    const c = box('agents-list');
    if (data.agents.length === 0) {
      c.appendChild(el('div', 'notice info', t("Aucune tentative enregistrée.")));
      return;
    }
    for (const a of data.agents) {
      const card = el('div', 'card');
      card.appendChild(el('div', 'notice info', t("Agent") + ': ' + a.id));
      card.appendChild(el('div', 'notice info', t("Tâche") + ': ' + a.task_id + ' / ' + t("Tentative") + ': ' + a.attempt_id));
      card.appendChild(el('div', 'notice info', t("État enregistré") + ': ' + engine(a.status)));
      card.appendChild(el('div', 'notice info', t("Fournisseur") + ': ' + a.provider + ' — ' + t("Rôle") + ': ' + a.role));
      card.appendChild(el('div', 'notice info', t("Début enregistré") + ': ' + (a.started || t("Non enregistré")) + ' — ' + t("Fin enregistrée") + ': ' + (a.ended || t("Non enregistré"))));
      c.appendChild(card);
    }
  }

  /**
   * renderHierarchy(data) — Fill #hierarchy with parent/previous links as plain text per agent.
   * Throws if container missing.
   */
  function renderHierarchy(data) {
    const c = box('hierarchy');
    for (const a of data.agents) {
      const line = el('div', 'notice info',
        a.id + ' → ' + t("Parent enregistré") + ': ' + (a.parent || t("Aucun lien enregistré")) + ' — ' + t("Tentative précédente enregistrée") + ': ' + (a.previous || t("Aucun lien enregistré")));
      c.appendChild(line);
    }
    if (data.agents.length === 0) {
      c.appendChild(el('div', 'notice info', t("Aucune tentative enregistrée.")));
    }
  }

  /**
   * render(data) — Validate payload, show work_id, then fill all four containers.
   * Input: {schema_version:1, work_id, budget:null|WorkBudgetState, agents}. Side effects: DOM mutation of the four containers only.
   * Errors: throws 'web-native-observation-invalid' on malformed data or missing containers.
   * Limits: read-only rendering; no networking, polling, timeouts, or arithmetic.
   */
  function render(data) {
    validate(data);
    renderBudget(data);
    renderAgents(data);
    renderHierarchy(data);
    const usage = box('usage-list');
    usage.appendChild(el('div', 'notice info', t("Mission") + ': ' + data.work_id));
  }

  /**
   * clear(message) — Empty all four containers and show message (string) in each as notice info.
   * Side effects: DOM mutation only. Errors: throws if any container missing.
   */
  function clear(message) {
    for (const id of ['budget-info', 'usage-list', 'agents-list', 'hierarchy']) {
      const c = box(id);
      c.appendChild(el('div', 'notice info', String(message)));
    }
  }

  globalThis.SwarmRustCockpitPanels = { render: render, clear: clear };
})();
