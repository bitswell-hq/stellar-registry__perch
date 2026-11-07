// Interactive figures: progressive enhancement over the static SVG that the
// LaTeX edition also draws (build/svg2tikz.py). Without this script every
// step and scenario shows at once, which is the print and no-JavaScript form.
//
// Markup, on <figure data-interactive="stepper">:
//   <g data-step="N">              drawn as past / current / future
//   [data-scenario="a b"]          shown only under the listed scenarios
//   [data-unreached="a b"]         drawn as not reached under the listed scenarios
//   [data-print="no"]              hidden until this script runs; never printed
//   [data-cursor] on a step group  shown only while its step is current
//   [data-detail="text"]           shown below the figure when selected or stepped to
//   [data-phase="id"]              with <main data-walkthrough>, links the step to the walkthrough
//   data-scenarios="id:Label|…"    on the figure: scenario picker
//   <li data-step="N"> in the figure's text alternative: the step narrative; a nested
//   <li data-step="N" data-scenario="s"> adds the narrative for scenario s, or replaces it
//   with data-mode="replace".
// A figure may instead name a controller registered with PerchFigures.register(name, fn);
// fn(fig, helpers) runs once, after the page has loaded.
(() => {
  const CONTROLLERS = {};
  const el = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    if (text != null) e.textContent = text;
    return e;
  };
  const squash = (s) => s.replace(/\s+/g, ' ').trim();
  // An item's own text, without the nested scenario items.
  const ownText = (li) => {
    const c = li.cloneNode(true);
    for (const n of c.querySelectorAll('ul, ol')) n.remove();
    return squash(c.textContent);
  };
  const bar = (fig) => {
    const b = el('div', { class: 'fig-bar', role: 'group', 'aria-label': 'Figure controls' });
    fig.prepend(b);
    return b;
  };
  const liveRegion = (fig, cls = 'fig-live') => {
    const p = el('p', { class: cls, 'aria-live': 'polite' });
    fig.querySelector('figcaption').before(p);
    return p;
  };

  // [data-detail] nodes: the text shows below the figure when a node is selected.
  function details(fig) {
    const nodes = [...fig.querySelectorAll('[data-detail]')];
    if (!nodes.length) return null;
    const out = liveRegion(fig, 'fig-detail');
    out.textContent = fig.dataset.detailHint || 'Source lines for a box appear here when you select it or step to it.';
    const show = (list, mark = true) => {
      out.textContent = list.map((n) => n.dataset.detail).join(' | ');
      for (const m of nodes) m.classList.toggle('hot', mark && list.includes(m));
    };
    for (const n of nodes) {
      n.addEventListener('mouseenter', () => show([n]));
      n.addEventListener('click', () => show([n]));
    }
    return { nodes, show };
  }

  function stepper(fig, { detail }) {
    const groups = [...fig.querySelectorAll('svg [data-step]')];
    const steps = [...new Set(groups.map((g) => Number(g.dataset.step)))].sort((a, b) => a - b);
    const items = [...fig.querySelectorAll('.fig-text li[data-step]')];
    const general = new Map(items.filter((li) => !li.dataset.scenario).map((li) => [Number(li.dataset.step), ownText(li)]));
    const scenarios = (fig.dataset.scenarios || '').split('|').filter(Boolean).map((s) => s.split(':'));
    const b = bar(fig);
    const prev = el('button', { type: 'button' }, 'Previous');
    const next = el('button', { type: 'button' }, 'Next');
    const all = el('button', { type: 'button' }, 'Show all');
    const count = el('span', { class: 'fig-count', 'aria-live': 'off' });
    let pick = null;
    if (scenarios.length) {
      pick = el('select', { 'aria-label': 'Scenario' });
      for (const [id, label] of scenarios) pick.append(el('option', { value: id }, label));
      b.append(pick);
    }
    b.append(prev, next, all, count);
    const main = document.querySelector('main[data-walkthrough]');
    let walk = null;
    if (main && groups.some((g) => g.dataset.phase)) {
      walk = el('a', { class: 'fig-walk' });
      b.append(walk);
    }
    const live = liveRegion(fig);
    let i = -1;
    const scenario = () => (pick ? pick.value : null);
    function draw() {
      const s = scenario();
      for (const n of fig.querySelectorAll('[data-scenario]')) {
        if (n.closest('.fig-text')) continue;
        n.classList.toggle('off', s != null && !n.dataset.scenario.split(' ').includes(s));
      }
      for (const n of fig.querySelectorAll('[data-unreached]')) {
        n.classList.toggle('unreached', s != null && n.dataset.unreached.split(' ').includes(s));
      }
      const cur = steps[i];
      for (const g of groups) {
        const n = Number(g.dataset.step);
        g.dataset.state = i < 0 ? '' : n < cur ? 'past' : n === cur ? 'current' : 'future';
      }
      prev.disabled = i <= 0;
      next.disabled = i >= steps.length - 1;
      count.textContent = i < 0 ? `${steps.length} steps` : `Step ${i + 1} of ${steps.length}`;
      if (i < 0) {
        live.textContent = '';
      } else {
        const extra = s && items.find((li) => Number(li.dataset.step) === cur && (li.dataset.scenario || '').split(' ').includes(s));
        const parts = extra && extra.dataset.mode === 'replace' ? [] : [general.get(cur) || ''];
        live.textContent = [...parts, extra ? squash(extra.textContent) : ''].filter(Boolean).join(' ');
      }
      if (detail) {
        const here = i < 0 ? [] : detail.nodes.filter((n) => {
          const g = n.closest('[data-step]');
          return g && Number(g.dataset.step) === cur && !n.closest('.off') && n.getBoundingClientRect().width > 0;
        });
        if (here.length) detail.show(here, false);
      }
      if (walk) {
        const g = groups.find((x) => Number(x.dataset.step) === cur && x.dataset.phase);
        walk.hidden = !g;
        if (g) {
          walk.href = `${main.dataset.walkthrough.replace(/index\.html$/, '')}#${s || 'lost'}/${g.dataset.phase}`;
          walk.textContent = 'This phase in the walkthrough';
        }
      }
      fig.dispatchEvent(new CustomEvent('figstep', { detail: { step: cur, scenario: s } }));
    }
    prev.onclick = () => { i = Math.max(0, i - 1); draw(); };
    next.onclick = () => { i = Math.min(steps.length - 1, i + 1); draw(); };
    all.onclick = () => { i = -1; draw(); };
    if (pick) pick.onchange = draw;
    b.addEventListener('keydown', (e) => {
      if (e.target === pick && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) return;
      const k = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
      if (k) { i = Math.min(steps.length - 1, Math.max(0, i + k)); draw(); e.preventDefault(); }
      if (e.key === 'Home') { i = 0; draw(); e.preventDefault(); }
      if (e.key === 'End') { i = steps.length - 1; draw(); e.preventDefault(); }
    });
    draw();
  }

  // fig-cap-window: a range input moves ledger L; the window (L − period, L] slides with it.
  // Verdicts come from the figure's text alternative (generated from the facts), never from here.
  CONTROLLERS['cap-window'] = (fig) => {
    const svgs = [...fig.querySelectorAll('svg[data-ledger-x]')];
    if (!svgs.length) return;
    const root = svgs[0];
    const limit = Number(root.dataset.limit);
    const period = Number(root.dataset.period);
    const charges = [...root.querySelectorAll('g[data-ledger][data-amount]')]
      .map((g) => [Number(g.dataset.ledger), Number(g.dataset.amount)]);
    const verdict = new Map([...fig.querySelectorAll('.fig-text li[data-step]')].map((li) => [li.dataset.step, squash(li.textContent)]));
    const at = new Map([...root.querySelectorAll('g[data-event][data-at]')].map((g) => [g.dataset.event, Number(g.dataset.at)]));
    const lab = (v) => (v === 0 ? 't' : v > 0 ? `t + ${v}` : `t − ${-v}`);
    const xOf = (svg) => {
      const pts = svg.dataset.ledgerX.split(' ').map((p) => p.split(':').map(Number));
      return (v) => {
        for (let k = 0; k < pts.length - 1; k++) {
          const [a, xa] = pts[k];
          const [b2, xb] = pts[k + 1];
          if (v >= a && v <= b2) return xa + ((v - a) / (b2 - a || 1)) * (xb - xa);
        }
        return v < pts[0][0] ? null : pts[pts.length - 1][1];
      };
    };
    const b = bar(fig);
    const lo = Math.min(...[...at.values()]) - 1;
    const hi = Math.max(...[...at.values()]) + 1;
    const label = el('label', { class: 'fig-range' }, 'Ledger L ');
    const range = el('input', { type: 'range', min: lo, max: hi, step: 1, value: root.dataset.static });
    label.append(range);
    b.append(label);
    const evs = el('span', { class: 'fig-events', role: 'group', 'aria-label': 'Scripted events' });
    const buttons = [...at.keys()].sort().map((k) => {
      const btn = el('button', { type: 'button', 'aria-pressed': 'false', 'aria-label': `Event ${k}` }, k);
      btn.onclick = () => { range.value = at.get(k); draw(k); };
      evs.append(btn);
      return btn;
    });
    b.append(evs);
    if (document.getElementById('cap-input')) b.append(el('a', { href: '#cap-input' }, 'Try your own amounts'));
    const live = liveRegion(fig);
    for (const n of fig.querySelectorAll('svg [data-static]')) n.classList.add('off');
    let touched = false;
    function draw(event) {
      const L = Number(range.value);
      const from = L - period;
      const total = charges.filter(([v]) => v > from && v <= L).reduce((a, [, amt]) => a + amt, 0);
      const text = `Ledger ${lab(L)}; window (${lab(from)}, ${lab(L)}]; ${total} of ${limit} XLM`;
      range.setAttribute('aria-valuetext', text);
      live.textContent = event ? `${text}. Event ${event}: ${verdict.get(String(event)) || ''}` : `${text}.`;
      for (const btn of buttons) btn.setAttribute('aria-pressed', String(btn.textContent === String(event)));
      for (const svg of svgs) {
        const x = xOf(svg);
        const narrow = svg.classList.contains('narrow');
        const cur = svg.querySelector('[data-live=cursor] line');
        cur.setAttribute('x1', x(L));
        cur.setAttribute('x2', x(L));
        for (const g of svg.querySelectorAll('g[data-ledger][data-amount]')) {
          const v = Number(g.dataset.ledger);
          const inside = v > from && v <= L;
          g.querySelector('rect').setAttribute('class', inside ? 'bx bx-hi' : 'bx dash');
          g.querySelector('text').setAttribute('class', inside ? 'x' : 'm');
        }
        for (const g of svg.querySelectorAll('g[data-event][data-at]')) {
          g.classList.toggle('hot', touched && (event ? g.dataset.event === String(event) : Number(g.dataset.at) === L));
        }
        const w = svg.querySelector('[data-live=window]');
        const [t, line, open, close] = [w.querySelector('text'), ...w.querySelectorAll('path')];
        const xa = x(from);
        const xb = x(L);
        const left = xa == null ? Number(svg.querySelector('[data-live=cursor]').dataset.left || 0) : xa;
        const y = Number(t.getAttribute('y')) + 12;
        t.setAttribute('x', left);
        t.textContent = narrow ? `at ${lab(L)}: (${lab(from)}, ${lab(L)}], ${total}` : `window at ${lab(L)}: (${lab(from)}, ${lab(L)}] holds ${total} of ${limit}`;
        line.setAttribute('d', `M${left} ${y}H${xb}`);
        open.setAttribute('d', xa == null ? '' : `M${xa + 6} ${y - 9}L${xa + 2} ${y - 5}L${xa + 1} ${y - 1}V${y + 1}L${xa + 2} ${y + 5}L${xa + 6} ${y + 9}`);
        close.setAttribute('d', `M${xb - 5} ${y - 8}H${xb}V${y + 8}H${xb - 5}`);
      }
    }
    range.addEventListener('input', () => { touched = true; draw(); });
    for (const btn of buttons) btn.addEventListener('click', () => { touched = true; }, { capture: true });
    draw();
    // Print the static figure, then restore the reader's ledger.
    let kept = null;
    window.addEventListener('beforeprint', () => {
      kept = [range.value, touched];
      range.value = root.dataset.static;
      touched = false;
      draw();
    });
    window.addEventListener('afterprint', () => {
      if (!kept) return;
      [range.value, touched] = kept;
      draw();
    });
  };

  // fig-proof-paths: pick a statement; the boxes for the objects it is about and the
  // trusted parts it assumes are marked. Reads the statement list's data-objects and
  // data-assumes (li#stmt-<id>), so the figure and the statements cannot disagree.
  CONTROLLERS['proof-paths'] = (fig) => {
    const stmts = [...document.querySelectorAll('li[id^="stmt-"][data-objects]')];
    const nodes = [...fig.querySelectorAll('svg [data-node]')];
    if (!stmts.length || !nodes.length) return;
    const name = (id) => (nodes.find((n) => n.dataset.node === id) || {}).dataset?.label || id;
    const b = bar(fig);
    const pick = el('select', { 'aria-label': 'Statement' });
    pick.append(el('option', { value: '' }, 'Every statement'));
    for (const li of stmts) {
      const strong = li.querySelector('strong');
      pick.append(el('option', { value: li.id }, squash(strong ? strong.textContent : li.id)));
    }
    b.append(pick);
    const link = el('a', { hidden: '' }, 'Read the statement');
    b.append(link);
    const live = liveRegion(fig);
    const list = (ids) => ids.map(name).join(', ');
    pick.onchange = () => {
      const li = pick.value && document.getElementById(pick.value);
      const objs = li ? li.dataset.objects.split(' ') : [];
      const ass = li ? (li.dataset.assumes || '').split(' ').filter(Boolean) : [];
      fig.classList.toggle('picking', !!li);
      for (const n of nodes) {
        n.classList.toggle('sel-obj', objs.includes(n.dataset.node));
        n.classList.toggle('sel-tr', ass.includes(n.dataset.node));
      }
      link.hidden = !li;
      if (li) link.href = `#${li.id}`;
      live.textContent = li
        ? `${squash(pick.selectedOptions[0].textContent)} About: ${list(objs)}. Assumes: ${list(ass)}.`
        : '';
    };
  };

  function init() {
    for (const fig of document.querySelectorAll('figure.fig')) {
      const kind = fig.dataset.interactive;
      fig.classList.add('js');
      try {
        const detail = details(fig);
        if (kind === 'stepper') stepper(fig, { detail });
        else if (kind && CONTROLLERS[kind]) CONTROLLERS[kind](fig, { el, bar, liveRegion, detail, stepper });
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
