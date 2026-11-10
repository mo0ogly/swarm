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
   * Show or remove the small explanation line under one metric counter.
   * @param {string} id counter element id inside a `.metric` card
   * @param {string|null} text translated hint, or null to remove it
   * @returns {void} DOM-only; missing elements are ignored, nothing is written
   */
  function setMetricHint(id, text) {
    const card = document.getElementById(id)?.closest('.metric');
    if (!card) return;
    let hint = card.querySelector('.metric-hint');
    if (!text) { hint?.remove(); return; }
    if (!hint) { hint = document.createElement('small'); hint.className = 'metric-hint'; card.appendChild(hint); }
    hint.textContent = text;
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
   * Build one task row (5 columns, text-only content and shared status badge).
   * The badge reflects the recorded task status, never inferred validation or
   * projection authority. DOM/translation errors propagate without an empty pass.
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
    for (const [index,c] of cells.entries()) {
      const td = document.createElement('td');
      if(index===1)td.appendChild(globalThis.SwarmWorkbench.statusBadge(task.status));
      else td.textContent = c; // plain text, no HTML interpolation
      tr.appendChild(td);
    }
    return tr;
  }

  /**
   * Fill conduite plan and state sections.
   * Current mode preserves calculation failures. History mode renders public
   * recorded task objects verbatim as safe text; it infers no validity or activity.
   * Each history summary shows the exact status through the shared workbench
   * badge (translated label + shape + tone), falling back to engine text when
   * the workbench module is absent.
   * @param {object|null} mission @param {string|null} missionError
   * @param {object} work @param {string} readMode @returns {void}
   */
  function renderConduite(mission, missionError, work, readMode) {
    const plan = document.getElementById('conduite-plan');
    const state = document.getElementById('conduite-state');
    if (plan) {
      plan.textContent = '';
      if (readMode === 'history' && !missionError) {
        const note = document.createElement('p');
        note.className = 'notice info';
        note.textContent = t('Historique enregistré : tâches, tentatives et revues. La validité actuelle des preuves n’est pas évaluée ; aucune relance ne découle de cette lecture.');
        plan.appendChild(note);
        for (const task of work.tasks) {
          const details = document.createElement('details');
          details.className = 'history-task';
          const summary = document.createElement('summary');
          // Same recorded-state badge as lanes/inspector: translated label,
          // shape and tone from the exact status code; unknown codes stay neutral.
          const badge = globalThis.SwarmWorkbench?.statusBadge;
          if (badge) summary.append(document.createTextNode(task.title || task.id), badge(task.status || '', task.status || ''));
          else summary.textContent = `${task.title || task.id} — ${engine(task.status || '')}`;
          const original = document.createElement('pre');
          original.className = 'history-original';
          original.textContent = JSON.stringify(task, null, 2);
          details.append(summary, original);
          plan.appendChild(details);
        }
      } else if (missionError) {
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
      state.textContent = readMode === 'history' && !missionError
        ? t('Lecture de l’historique enregistré. Les compteurs de validation actuelle et d’activité restent inconnus.')
        : missionError
        ? engine(missionError)
        : mission && (mission.summary != null || mission.next != null)
          ? [mission.summary != null ? engine(mission.summary) : '', mission.next != null ? engine(mission.next) : ''].filter(Boolean).join(' — ')
          : t('État non disponible.');
    }
  }

  /**
   * Render cockpit read-only view from validated inputs.
   * History displays all task fields already returned by the protected public
   * Work reader, as inert text. It does not qualify their contents, scan files,
   * hide read failures or turn saved acceptance into current validation.
   * @param {{work?:object,mission?:object|null,missionError?:string|null,control?:object|null,readMode?:string}} data
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
    // Say why the counters stay empty instead of showing a bare dash: the native
    // engine has no decision producer, and history reads do not evaluate current
    // validation. No count is inferred from recorded task states.
    document.getElementById('decision-count')?.closest('.metric')?.setAttribute('data-unavailable', 'true');
    setMetricHint('decision-count', t('Non fourni par le moteur natif.'));
    setMetricHint('accepted-count', mission && isSafeCount(mission.validated) ? null : t('Validation courante non évaluée dans cette lecture.'));

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

    renderConduite(mission, missionError, work, data.readMode);
    globalThis.SwarmWorkbench?.render(data);

    // Reuse canonical graph helpers/CSS with actual stored dependencies only.
    // The caller supplies an explicit Work; failures stay visible, no empty pass.
    try { SwarmRustDependencyGraph.render(work); }
    catch (cause) { SwarmRustDependencyGraph.clear(t('Graphe indisponible : ') + String(cause.message)); }

  }

  globalThis.SwarmRustCockpitRead = Object.freeze({ render });
})();
