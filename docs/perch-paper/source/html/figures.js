// Interactive figures: progressive enhancement over the static SVG that the
// LaTeX edition also draws (build/svg2tikz.py). Without this script every
// step and scenario shows at once, which is the print and no-JavaScript form.
//
// Markup, on <figure data-interactive="stepper">:
//   <g data-step="N">            drawn as past / current / future
//   [data-scenario="a b"]        shown only under the listed scenarios
//   [data-print="no"]            hidden until this script runs; never printed
//   [data-detail="text"]         hover, focus or click shows the text below
//   data-scenarios="id:Label|…"  on the figure: scenario picker
//   <li data-step="N"> in the figure's text alternative: the step narrative
// A figure may also name a controller registered in CONTROLLERS.
(() => {
  const CONTROLLERS = {};
  const el = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    if (text != null) e.textContent = text;
    return e;
  };

  function stepper(fig) {
    const groups = [...fig.querySelectorAll('svg [data-step]')];
    const steps = [...new Set(groups.map((g) => Number(g.dataset.step)))].sort((a, b) => a - b);
    const narrative = new Map(
      [...fig.querySelectorAll('.fig-text li[data-step]')].map((li) => [Number(li.dataset.step), li.textContent.trim()]));
    const scenarios = (fig.dataset.scenarios || '').split('|').filter(Boolean).map((s) => s.split(':'));
    const bar = el('div', { class: 'fig-bar', role: 'group', 'aria-label': 'Figure controls' });
    const prev = el('button', { type: 'button' }, 'Previous');
    const next = el('button', { type: 'button' }, 'Next');
    const all = el('button', { type: 'button' }, 'Show all');
    const count = el('span', { class: 'fig-count' });
    const live = el('p', { class: 'fig-live', 'aria-live': 'polite' });
    let pick = null;
    if (scenarios.length) {
      pick = el('select', { 'aria-label': 'Scenario' });
      for (const [id, label] of scenarios) pick.append(el('option', { value: id }, label));
      bar.append(pick);
    }
    bar.append(prev, next, all, count);
    fig.prepend(bar);
    fig.querySelector('figcaption').before(live);
    let i = -1;
    const scenario = () => (pick ? pick.value : null);
    function draw() {
      const s = scenario();
      for (const n of fig.querySelectorAll('[data-scenario]')) {
        n.classList.toggle('off', s != null && !n.dataset.scenario.split(' ').includes(s));
      }
      for (const g of groups) {
        const n = Number(g.dataset.step);
        g.dataset.state = i < 0 ? '' : n < steps[i] ? 'past' : n === steps[i] ? 'current' : 'future';
      }
      prev.disabled = i <= 0;
      next.disabled = i >= steps.length - 1;
      count.textContent = i < 0 ? `${steps.length} steps` : `Step ${i + 1} of ${steps.length}`;
      const sKey = s ? `${steps[i]}:${s}` : null;
      const scen = sKey && fig.querySelector(`.fig-text li[data-step="${steps[i]}"][data-scenario~="${s}"]`);
      live.textContent = i < 0 ? '' : (scen ? scen.textContent.trim() : narrative.get(steps[i]) || '');
      fig.dispatchEvent(new CustomEvent('figstep', { detail: { step: steps[i], scenario: s } }));
    }
    prev.onclick = () => { i = Math.max(0, i - 1); draw(); };
    next.onclick = () => { i = Math.min(steps.length - 1, i + 1); draw(); };
    all.onclick = () => { i = -1; draw(); };
    if (pick) pick.onchange = draw;
    bar.addEventListener('keydown', (e) => {
      const k = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
      if (e.target === pick && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) return;
      if (k) { i = Math.min(steps.length - 1, Math.max(0, i + k)); draw(); e.preventDefault(); }
      if (e.key === 'Home') { i = 0; draw(); e.preventDefault(); }
      if (e.key === 'End') { i = steps.length - 1; draw(); e.preventDefault(); }
    });
    draw();
  }

  function details(fig) {
    const nodes = fig.querySelectorAll('[data-detail]');
    if (!nodes.length) return;
    const out = el('p', { class: 'fig-detail', 'aria-live': 'polite' });
    out.textContent = fig.dataset.detailHint || 'Point at or tap a box for detail.';
    fig.querySelector('figcaption').before(out);
    for (const n of nodes) {
      const show = () => {
        out.textContent = n.dataset.detail;
        for (const m of nodes) m.classList.toggle('hot', m === n);
      };
      n.addEventListener('mouseenter', show);
      n.addEventListener('click', show);
    }
  }

  function init() {
    for (const fig of document.querySelectorAll('figure.fig')) {
      fig.classList.add('js');
      const kind = fig.dataset.interactive;
      try {
        if (kind === 'stepper') stepper(fig);
        else if (kind && CONTROLLERS[kind]) CONTROLLERS[kind](fig, { el, stepper });
        details(fig);
      } catch (err) {
        console.error(`figure ${fig.id}:`, err);
        fig.classList.remove('js');
      }
    }
  }
  window.PerchFigures = { register: (name, fn) => { CONTROLLERS[name] = fn; } };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else queueMicrotask(init);
})();
