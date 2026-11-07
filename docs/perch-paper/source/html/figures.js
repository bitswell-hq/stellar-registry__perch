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
//   data-scenarios="id:Label|…"    on the figure: scenario picker, after "Every scenario"
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
  // The sticky head: the controls, then the narration, so both stay in view while stepping.
  const head = (fig) => {
    const h = el('div', { class: 'fig-head' });
    const bar = el('div', { class: 'fig-bar', role: 'group', 'aria-label': 'Figure controls' });
    const live = el('p', { class: 'fig-live', 'aria-live': 'polite' });
    h.append(bar, live);
    fig.prepend(h);
    return { head: h, bar, live };
  };
  // A button that cannot act is aria-disabled, not disabled, so it keeps keyboard focus.
  const button = (label, attrs = {}) => el('button', { type: 'button', ...attrs }, label);
  const setOff = (btn, off) => btn.setAttribute('aria-disabled', String(off));
  const isOff = (btn) => btn.getAttribute('aria-disabled') === 'true';
  const reduced = () => window.matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches;
  // Scroll the drawn nodes into view below the sticky head.
  function follow(hd, nodes) {
    const rects = nodes.map((n) => n.getBoundingClientRect()).filter((r) => r.width || r.height);
    if (!rects.length) return;
    const top = Math.min(...rects.map((r) => r.top));
    const bottom = Math.max(...rects.map((r) => r.bottom));
    const minTop = hd.offsetHeight + 8;
    let dy = 0;
    if (top < minTop) dy = top - minTop;
    else if (bottom > window.innerHeight - 8) dy = Math.min(bottom - window.innerHeight + 8, top - minTop);
    if (dy) window.scrollBy({ top: dy, behavior: reduced() ? 'auto' : 'smooth' });
  }

  // [data-detail] nodes: the text shows below the figure when a node is selected.
  function details(fig) {
    const nodes = [...fig.querySelectorAll('[data-detail]')];
    if (!nodes.length) return null;
    const out = el('p', { class: 'fig-detail', 'aria-live': 'polite' });
    fig.querySelector('figcaption').before(out);
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

  function stepper(fig, { detail } = {}) {
    const groups = [...fig.querySelectorAll('svg [data-step]')];
    const steps = [...new Set(groups.map((g) => Number(g.dataset.step)))].sort((a, b) => a - b);
    const items = [...fig.querySelectorAll('.fig-text li[data-step]')];
    const general = new Map(items.filter((li) => !li.dataset.scenario).map((li) => [Number(li.dataset.step), ownText(li)]));
    const scenarios = (fig.dataset.scenarios || '').split('|').filter(Boolean).map((s) => s.split(':'));
    const { head: hd, bar, live } = head(fig);
    let pick = null;
    if (scenarios.length) {
      pick = el('select', { 'aria-label': 'Scenario' });
      pick.append(el('option', { value: '' }, 'Every scenario (as printed)'));
      for (const [id, label] of scenarios) pick.append(el('option', { value: id }, label));
      bar.append(pick);
    }
    const prev = button('Previous');
    const next = button('Next');
    const all = button('Show all');
    const count = el('span', { class: 'fig-count' });
    bar.append(prev, next, all, count);
    let i = -1;
    function draw(move) {
      const s = pick && pick.value ? pick.value : null;
      for (const n of fig.querySelectorAll('svg [data-scenario]')) {
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
      setOff(prev, i <= 0);
      setOff(next, i >= steps.length - 1);
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
      if (move && i >= 0) follow(hd, groups.filter((g) => Number(g.dataset.step) === cur && !g.hasAttribute('data-cursor')));
      fig.dispatchEvent(new CustomEvent('figstep', { detail: { step: cur, scenario: s } }));
    }
    const go = (k) => { i = Math.min(steps.length - 1, Math.max(0, k)); draw(true); };
    prev.onclick = () => { if (!isOff(prev)) go(i - 1); };
    next.onclick = () => { if (!isOff(next)) go(i + 1); };
    all.onclick = () => { i = -1; draw(false); };
    if (pick) pick.onchange = () => draw(false);
    bar.addEventListener('keydown', (e) => {
      if (e.target === pick && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) return;
      if (e.target.tagName === 'INPUT') return;
      const k = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
      if (k) { go(i + k); e.preventDefault(); }
      if (e.key === 'Home') { go(0); e.preventDefault(); }
      if (e.key === 'End') { go(steps.length - 1); e.preventDefault(); }
    });
    draw(false);
    return { head: hd, bar, live };
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
    const { bar: b, live } = head(fig);
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
    const list = (ids) => ids.map(name).join(', ');
    const { bar, live } = head(fig);
    const pick = el('select', { 'aria-label': 'Statement' });
    pick.append(el('option', { value: '' }, 'Every statement'));
    // Statements about the same objects share a group.
    const groups = new Map();
    for (const li of stmts) {
      const key = li.dataset.objects;
      if (!groups.has(key)) {
        groups.set(key, el('optgroup', { label: `About ${list(key.split(' '))}` }));
        pick.append(groups.get(key));
      }
      const strong = li.querySelector('strong');
      groups.get(key).append(el('option', { value: li.id }, squash(strong ? strong.textContent : li.id)));
    }
    bar.append(pick);
    bar.append(el('span', { class: 'fig-key' }, 'Thick amber border: what it is about. Thick blue border: what it assumes.'));
    const link = el('a', { hidden: '' }, 'Read the statement');
    bar.append(link);
    pick.onchange = () => {
      const li = pick.value && document.getElementById(pick.value);
      const objs = li ? li.dataset.objects.split(' ') : [];
      const ass = li ? (li.dataset.assumes || '').split(' ').filter(Boolean) : [];
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

  // fig-same-task: a stepper over the recorded requests, plus recorded repeated sends
  // ("Send again" shows the next recorded send of a lane; it decides nothing) and a
  // switch between the recorded contracts per payment and what is documented but not run.
  CONTROLLERS['same-task'] = (fig, helpers) => {
    stepper(fig, helpers);
    const svgs = [...fig.querySelectorAll('svg')];
    const bar = el('div', { class: 'fig-bar fig-bar2', role: 'group', 'aria-label': 'Repeated payments and contracts' });
    const live = el('p', { class: 'fig-live', 'aria-live': 'polite' });
    svgs[svgs.length - 1].after(bar, live);
    const lanes = new Map();
    for (const g of fig.querySelectorAll('svg [data-lane]')) {
      lanes.set(g.dataset.lane, Math.max(lanes.get(g.dataset.lane) || 0, Number(g.dataset.send)));
    }
    const names = { percall: 'Send again: per-call bound', window: 'Send again: window' };
    for (const [lane, n] of lanes) {
      let pos = 0;
      const btn = button(names[lane] || `Send again: ${lane}`);
      btn.onclick = () => {
        pos = pos >= n ? 1 : pos + 1;
        let say = '';
        for (const g of fig.querySelectorAll(`svg [data-lane="${lane}"]`)) {
          const k = Number(g.dataset.send);
          g.classList.toggle('off', k > pos);
          g.dataset.state = k === pos ? 'current' : 'past';
          if (k === pos) say = g.dataset.say;
        }
        live.textContent = pos === n ? `${say} That was the last recorded send; press again to start over.` : say;
      };
      bar.append(btn);
    }
    const toggle = el('span', { class: 'fig-events', role: 'group', 'aria-label': 'Contracts per payment' });
    const views = [['recorded', 'Recorded per payment'], ['documented', 'Documented, not run']];
    const btns = views.map(([v, label]) => {
      const b = button(label, { 'aria-pressed': String(v === 'recorded') });
      b.onclick = () => {
        for (const g of fig.querySelectorAll('svg [data-view]')) g.classList.toggle('off', g.dataset.view !== v);
        for (const [w, x] of btns) x.setAttribute('aria-pressed', String(w === v));
        const shown = svgs.find((s) => s.getBoundingClientRect().width > 0) || svgs[0];
        live.textContent = [...shown.querySelectorAll(`[data-view="${v}"]`)].map((g) => g.dataset.say).join(' ');
      };
      toggle.append(b);
      return [v, b];
    });
    bar.append(toggle);
  };

  function init() {
    for (const fig of document.querySelectorAll('figure.fig')) {
      const kind = fig.dataset.interactive;
      fig.classList.add('js');
      try {
        const detail = details(fig);
        if (kind === 'stepper') stepper(fig, { detail });
        else if (kind && CONTROLLERS[kind]) CONTROLLERS[kind](fig, { el, head, button, detail, stepper });
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
