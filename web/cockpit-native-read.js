/* web/cockpit-native-read.js — read-only cockpit renderer (no authorizations). */
(() => {
  'use strict';
  const t = (k) => (globalThis.SwarmI18n && typeof globalThis.SwarmI18n.t === 'function' ? globalThis.SwarmI18n.t(k) : k);
  const engine = (m) => (globalThis.SwarmI18n && typeof globalThis.SwarmI18n.engine === 'function' ? globalThis.SwarmI18n.engine(m) : String(m));

  /**
   * Safe nonnegative integer check.
   * @param {*} v
   * @returns {boolean}
   */
  function isSafeCount(v) {
    return typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
  }

  /**
   * Set textContent of an element if present.
   * @param {string} id @param {string} text @returns {void} @throws {Error} if document missing
   */
  function setText(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  }

  /**
   * Toggle a notice element with explanation text.
   * @param {string} id @param {boolean} show @param {string} text @returns {void}
   */
  function setNotice(id, show, text) {
    const el = document.getElementById(id);
    if (!el) return;
    el.hidden = !show;
    if (show) el.textContent = text;
  }

  /**
   * Build one task row (5 columns, plain text).
   * @param {{id:string,title:string,status?:string,owner?:string,depends?:Array}} task
   * @param {object|null} proj matching projection (id,state,label,reason,attempt)
   * @returns {HTMLTableRowElement}
   */
  function taskRow(task, proj) {
    const tr = document.createElement('tr');
    const cells = [
      `${task.title != null ? String(task.title) : ''} (${task.id != null ? String(task.id) : ''})`,
      engine(task.status != null ? String(task.status) : t('État non disponible.')),
      task.owner != null ? String(task.owner) : t('Non attribué'),
      Array.isArray(task.depends) ? task.depends.join(', ') : t('Aucune dépendance enregistrée'),
      t('Commandes non raccordées')
    ];
    for (const c of cells) {
      const td = document.createElement('td');
      td.textContent = c; // plain text, no HTML interpolation
      tr.appendChild(td);
    }
    return tr;
  }

  /**
   * Fill conduite plan and state sections.
   * @param {object|null} mission @param {string|null} missionError @returns {void}
   */
  function renderConduite(mission, missionError) {
    const plan = document.getElementById('conduite-plan');
    const state = document.getElementById('conduite-state');
    if (plan) {
      plan.textContent = '';
      if (missionError) {
        plan.textContent = engine(missionError);
      } else if (mission && Array.isArray(mission.tasks)) {
        for (const p of mission.tasks) {
          const div = document.createElement('div');
          div.className = 'notice info';
          const parts = [`${p.title != null ? String(p.title) : String(p.id != null ? p.id : '')} : ${engine(p.label || p.state)}`];
          if (p.reason) parts.push(engine(p.reason));
          if (p.attempt) {
            if (p.attempt.agent_id) parts.push(`agent_id=${String(p.attempt.agent_id)}`);
            if (p.attempt.attempt_id) parts.push(`attempt_id=${String(p.attempt.attempt_id)}`);
          }
          div.textContent = parts.join(' — ');
          plan.appendChild(div);
        }
      } else {
        plan.textContent = t('Suivi détaillé indisponible.');
      }
    }
    if (state) {
      state.textContent = missionError
        ? engine(missionError)
        : mission && (mission.summary != null || mission.next != null)
          ? [mission.summary != null ? engine(mission.summary) : '', mission.next != null ? engine(mission.next) : ''].filter(Boolean).join(' — ')
          : t('État non disponible.');
    }
  }

  /**
   * Render cockpit read-only view from validated inputs.
   * @param {{work?:object,mission?:object|null,missionError?:string|null,control?:object|null}} data
   * @returns {void} @throws {Error} on unexpected DOM failures
   */
  function render(data) {
    if (!data?.work || typeof data.work.id !== 'string' || typeof data.work.title !== 'string' || !Array.isArray(data.work.tasks)) throw new Error('web-native-work-invalid');
    const work = data.work;
    const mission = data.mission && typeof data.mission === 'object' ? data.mission : null;
    const missionError = typeof data.missionError === 'string' ? data.missionError : null;
    const tasks = Array.isArray(work.tasks) ? work.tasks : [];

    setText('title', work.title != null ? String(work.title) : t('Mission non disponible'));
    setText('objective', work.objective != null ? String(work.objective) : t('Objectif non disponible'));
    setText('task-count', String(tasks.length));
    setText('accepted-count', mission && isSafeCount(mission.validated) ? String(mission.validated) : '—');
    setText('active-count', mission && isSafeCount(mission.active_agents) ? String(mission.active_agents) : '—');
    setText('decision-count', '—'); // no decisions producer available

    setNotice('validation-status', true,
      t('Les états enregistrés ne prouvent ni une activité en cours ni une validation indépendante.'));

    const tbody = document.getElementById('tasks-body');
    if (tbody) {
      tbody.textContent = '';
      const byId = mission && Array.isArray(mission.tasks) ? new Map(mission.tasks.map((p) => [String(p.id), p])) : new Map();
      for (const task of tasks) {
        tbody.appendChild(taskRow(task, byId.get(String(task.id))));
      }
      if (!tasks.length) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 5;
        td.textContent = t('Aucune tâche enregistrée.');
        tr.appendChild(td);
        tbody.appendChild(tr);
      }
    }

    renderConduite(mission, missionError);

    // Reuse canonical graph helpers/CSS with actual stored dependencies only.
    // The caller supplies an explicit Work; failures stay visible, no empty pass.
    try { SwarmRustDependencyGraph.render(work); }
    catch (cause) { SwarmRustDependencyGraph.clear(t('Graphe indisponible : ') + String(cause.message)); }

  }

  globalThis.SwarmRustCockpitRead = Object.freeze({ render });
})();
