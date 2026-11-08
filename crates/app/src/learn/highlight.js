// Adapted from batna-app main: crates/app/src/learn/highlight.js (#770).
// Mohamed's own highlights in a lesson (learn/highlight.rs, 2026-11-08).
// Select words in the open step and a small bar offers five colours, a fill
// or an underline, and a pill or rectangle shape; tap a highlight to change
// or remove it. Each highlight is anchored by its words plus a little text
// before and after (steps_core::highlight::locate is the same rule), so it
// finds its place again after the step re-renders.
//
// Drawing never edits the step's own nodes (Dioxus owns them): the shapes
// are drawn in a `.uhl-layer` under the text of the box they sit in, and the
// text on a fill turns dark through the CSS Custom Highlight API where the
// WebView has it (otherwise the fill is lighter, so the theme's text still
// reads).
(() => {
  if (window.__stepsHl) return;
  const SKIP = "svg, button, input, textarea, select, .uhl-layer, .uhl-bar, [aria-hidden='true']";
  const H = (window.__stepsHl = { list: [], lesson: "", labels: {}, send: null, on: false });
  const ink = typeof CSS !== "undefined" && CSS.highlights && typeof Highlight === "function";
  document.documentElement.dataset.uhlInk = ink ? "yes" : "no";
  const pref = (k, d) => {
    try { return localStorage.getItem("steps.hl." + k) || d; } catch (_) { return d; }
  };
  const setPref = (k, v) => {
    try { localStorage.setItem("steps.hl." + k, v); } catch (_) { /* private mode */ }
  };
  let style = pref("style", "fill");
  let shape = pref("shape", "pill");
  let bar = null;
  let editing = null; // the highlight the bar is editing, or null for a new one
  let pending = null; // the selection a new highlight would take

  const stage = () => document.querySelector(".ls-player .ls-stage:not([hidden])");
  const stepOf = (el) => Number(el?.dataset?.n || 0) - 1;

  // The step's visible text, as code points, with the node each came from.
  function model(root) {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: (n) => (n.parentElement?.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
    });
    const nodes = [];
    const chars = [];
    for (let n = walker.nextNode(); n; n = walker.nextNode()) {
      const cps = Array.from(n.data);
      nodes.push({ n, start: chars.length, cps });
      for (const c of cps) chars.push(c);
    }
    return { nodes, chars };
  }

  // Code-point index of (node, utf16 offset) in the model.
  function indexOf(m, node, offset) {
    for (const e of m.nodes) {
      if (e.n === node) return e.start + Array.from(node.data.slice(0, offset)).length;
    }
    // An element boundary: the first text node at or after it.
    const at = document.createRange();
    try { at.setStart(node, offset); } catch (_) { return -1; }
    for (const e of m.nodes) if (at.comparePoint(e.n, 0) >= 0) return e.start;
    return m.chars.length;
  }


  function point(m, at) {
    for (const e of m.nodes) {
      if (at <= e.start + e.cps.length) {
        const utf16 = e.cps.slice(0, at - e.start).join("").length;
        return [e.n, utf16];
      }
    }
    const last = m.nodes[m.nodes.length - 1];
    return last ? [last.n, last.n.data.length] : null;
  }

  // steps_core::highlight::locate, on code points.
  function locate(m, h) {
    const q = Array.from(h.quote);
    const p = Array.from(h.prefix || "");
    const s = Array.from(h.suffix || "");
    const c = m.chars;
    if (!q.length || q.length > c.length) return null;
    let best = null;
    outer: for (let i = 0; i + q.length <= c.length; i++) {
      for (let j = 0; j < q.length; j++) if (c[i + j] !== q[j]) continue outer;
      let before = 0;
      while (before < p.length && before < i && p[p.length - 1 - before] === c[i - 1 - before]) before++;
      let after = 0;
      while (after < s.length && i + q.length + after < c.length && s[after] === c[i + q.length + after]) after++;
      const score = before + after;
      const dist = Math.abs(i - (h.at || 0));
      if (!best || score > best.score || (score === best.score && dist < best.dist)) best = { score, dist, i };
    }
    return best ? [best.i, best.i + q.length] : null;
  }

  function rangeOf(m, h) {
    const at = locate(m, h);
    if (!at) return null;
    const a = point(m, at[0]);
    const b = point(m, at[1]);
    if (!a || !b) return null;
    const r = document.createRange();
    r.setStart(a[0], a[1]);
    r.setEnd(b[0], b[1]);
    return r;
  }

  // The box a highlight is drawn in: the nearest ancestor (up to the step)
  // with a background of its own, so a quote box doesn't hide it.
  function hostOf(node, root) {
    for (let el = node.nodeType === 1 ? node : node.parentElement; el && el !== root; el = el.parentElement) {
      const bg = getComputedStyle(el).backgroundColor;
      if (bg && bg !== "transparent" && !/rgba\([^)]*,\s*0\)$/.test(bg)) return el;
    }
    return root;
  }

  // One box per line: the fragments of strong/bdi runs on the same line merge.
  function lines(rects) {
    const out = [];
    for (const r of [...rects].filter((r) => r.width > 0.5).sort((a, b) => a.top - b.top || a.left - b.left)) {
      const last = out[out.length - 1];
      if (last && Math.abs(last.top - r.top) < 4 && Math.abs(last.bottom - r.bottom) < 4 && r.left <= last.right + 8) {
        last.right = Math.max(last.right, r.right);
        last.left = Math.min(last.left, r.left);
        last.top = Math.min(last.top, r.top);
        last.bottom = Math.max(last.bottom, r.bottom);
      } else {
        out.push({ left: r.left, right: r.right, top: r.top, bottom: r.bottom });
      }
    }
    return out;
  }

  function clear() {
    document.querySelectorAll(".uhl-layer").forEach((l) => l.remove());
    document.querySelectorAll("[data-uhl-host]").forEach((el) => delete el.dataset.uhlHost);
    if (ink) CSS.highlights.delete("uhl-ink");
  }

  function draw() {
    clear();
    const root = stage();
    if (!root || !H.on) return;
    const step = stepOf(root);
    const mine = H.list.filter((h) => h.step === step);
    if (!mine.length) return;
    const m = model(root);
    const inked = [];
    for (const h of mine) {
      const r = rangeOf(m, h);
      if (!r) continue;
      if (h.style === "fill") inked.push(r);
      const host = hostOf(r.commonAncestorContainer, root);
      host.dataset.uhlHost = "";
      let layer = host.querySelector(":scope > .uhl-layer");
      if (!layer) {
        layer = document.createElement("div");
        layer.className = "uhl-layer";
        layer.setAttribute("aria-hidden", "true");
        host.append(layer);
      }
      const box = host.getBoundingClientRect();
      const ox = box.left + host.clientLeft - host.scrollLeft;
      const oy = box.top + host.clientTop - host.scrollTop;
      for (const l of lines(r.getClientRects())) {
        const i = document.createElement("i");
        i.dataset.c = h.colour;
        i.dataset.s = h.style;
        i.dataset.sh = h.shape;
        i.dataset.id = h.id;
        const pad = h.shape === "pill" ? 5 : 1;
        const top = h.style === "fill" ? l.top - 1 : l.bottom - 1;
        const height = h.style === "fill" ? l.bottom - l.top + 2 : 4;
        i.style.cssText = `inset-inline-start:auto;left:${l.left - ox - pad}px;top:${top - oy}px;width:${l.right - l.left + 2 * pad}px;height:${height}px`;
        layer.append(i);
      }
    }
    if (ink && inked.length) CSS.highlights.set("uhl-ink", new Highlight(...inked));
  }

  let queued = false;
  function redraw() {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      draw();
    });
  }

  // ---------- the bar ----------
  const COLOURS = ["yellow", "green", "blue", "pink", "orange"];
  function button(cls, label, on) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = cls;
    b.setAttribute("aria-label", label);
    b.title = label;
    b.addEventListener("pointerdown", (e) => e.preventDefault()); // keep the selection
    b.addEventListener("click", (e) => {
      e.preventDefault();
      on();
    });
    return b;
  }

  function makeBar() {
    const L = H.labels;
    bar = document.createElement("div");
    bar.className = "uhl-bar";
    bar.setAttribute("role", "toolbar");
    bar.setAttribute("aria-label", L.bar || "");
    bar.dir = "rtl";
    bar.hidden = true;
    const dots = document.createElement("div");
    dots.className = "uhl-dots";
    COLOURS.forEach((c) => {
      const b = button("uhl-dot", (L.colours || {})[c] || c, () => pick({ colour: c }));
      b.dataset.c = c;
      dots.append(b);
    });
    const seg = (items, key) => {
      const g = document.createElement("div");
      g.className = "uhl-seg";
      for (const [v, label] of items) {
        const b = button("uhl-opt", label, () => pick({ [key]: v }));
        b.dataset[key] = v;
        b.textContent = label;
        g.append(b);
      }
      return g;
    };
    const del = button("uhl-del", L.remove || "×", () => remove());
    del.textContent = L.remove || "×";
    bar.append(dots, seg([["fill", L.fill], ["underline", L.underline]], "style"), seg([["pill", L.pill], ["rect", L.rect]], "shape"), del);
    document.body.append(bar);
  }

  function sync() {
    const cur = editing || { style, shape, colour: null };
    bar.querySelectorAll(".uhl-opt").forEach((b) => {
      const k = b.dataset.style ? "style" : "shape";
      b.setAttribute("aria-pressed", String(b.dataset[k] === cur[k]));
    });
    bar.querySelectorAll(".uhl-dot").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.c === cur.colour)));
    bar.querySelector(".uhl-del").hidden = !editing;
  }

  function show(rect) {
    if (!bar) makeBar();
    sync();
    bar.hidden = false;
    const w = bar.offsetWidth;
    const h = bar.offsetHeight;
    const x = Math.min(Math.max(8, rect.left + rect.width / 2 - w / 2), innerWidth - w - 8);
    // A finger's selection gets the system's copy menu above it, so the bar
    // goes under the words on touch screens and above them with a mouse.
    const above = rect.top - h - 10;
    const below = rect.bottom + 14;
    const touch = matchMedia("(pointer: coarse)").matches;
    const y = touch
      ? (below + h < innerHeight - 8 ? below : Math.max(8, above))
      : (above > 8 ? above : Math.min(below, innerHeight - h - 8));
    bar.style.left = `${x}px`;
    bar.style.top = `${y}px`;
  }

  function hide() {
    if (bar) bar.hidden = true;
    editing = null;
    pending = null;
  }

  function pick(change) {
    if (editing) {
      Object.assign(editing, change);
      if (change.style) style = change.style, setPref("style", style);
      if (change.shape) shape = change.shape, setPref("shape", shape);
      H.send?.("put:" + JSON.stringify(editing));
      sync();
      redraw();
      return;
    }
    if (change.style) { style = change.style; setPref("style", style); sync(); return; }
    if (change.shape) { shape = change.shape; setPref("shape", shape); sync(); return; }
    if (!pending) return;
    const h = { ...pending, colour: change.colour, style, shape };
    H.list.push(h);
    H.send?.("add:" + JSON.stringify(h));
    window.getSelection()?.removeAllRanges();
    hide();
    redraw();
  }

  function remove() {
    if (!editing) return;
    const id = editing.id;
    H.list = H.list.filter((h) => h.id !== id);
    H.send?.("del:" + id);
    hide();
    redraw();
  }

  const newId = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 8);

  // A selection inside the open step becomes a possible new highlight.
  function fromSelection() {
    const sel = window.getSelection();
    const root = stage();
    if (!H.on || !sel || sel.isCollapsed || !sel.rangeCount || !root) return false;
    const r = sel.getRangeAt(0);
    if (!root.contains(r.startContainer) || !root.contains(r.endContainer)) return false;
    if (r.commonAncestorContainer.parentElement?.closest?.(SKIP)) return false;
    const m = model(root);
    let a = indexOf(m, r.startContainer, r.startOffset);
    let b = indexOf(m, r.endContainer, r.endOffset);
    if (a < 0 || b < 0 || b <= a) return false;
    // Trim spaces at the ends, so a double-click's trailing space isn't kept.
    while (a < b && /\s/.test(m.chars[a])) a++;
    while (b > a && /\s/.test(m.chars[b - 1])) b--;
    if (b <= a) return false;
    pending = {
      id: newId(),
      step: stepOf(root),
      quote: m.chars.slice(a, b).join(""),
      prefix: m.chars.slice(Math.max(0, a - 32), a).join(""),
      suffix: m.chars.slice(b, b + 32).join(""),
      at: a,
    };
    editing = null;
    show(r.getBoundingClientRect());
    return true;
  }

  // A tap on a highlight opens the bar for it.
  function hitTest(x, y) {
    const root = stage();
    if (!root) return null;
    for (const i of root.querySelectorAll(".uhl-layer i")) {
      const r = i.getBoundingClientRect();
      const slack = i.dataset.s === "underline" ? 14 : 2;
      if (x >= r.left && x <= r.right && y >= r.top - slack && y <= r.bottom + 2) {
        return { h: H.list.find((h) => h.id === i.dataset.id), r };
      }
    }
    return null;
  }

  let timer = 0;
  document.addEventListener("selectionchange", () => {
    if (!H.on) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      const sel = window.getSelection();
      if (sel && !sel.isCollapsed) fromSelection();
      else if (!editing) hide();
    }, 280);
  });
  document.addEventListener("pointerup", (e) => {
    if (!H.on || e.target.closest?.(".uhl-bar")) return;
    const sel = window.getSelection();
    if (sel && !sel.isCollapsed) return;
    const hit = e.target.closest?.(".ls-stage") && !e.target.closest(SKIP) ? hitTest(e.clientX, e.clientY) : null;
    if (hit?.h) {
      editing = hit.h;
      pending = null;
      show(hit.r);
    } else if (bar && !bar.hidden) {
      hide();
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && bar && !bar.hidden) {
      e.stopPropagation();
      hide();
    }
  }, true);
  document.addEventListener("scroll", () => bar && !bar.hidden && hide(), true);
  addEventListener("resize", redraw);
  document.fonts?.ready?.then(redraw);
  new MutationObserver((records) => {
    if (!H.on) return;
    if (records.every((r) => r.target.closest?.(".uhl-layer, .uhl-bar") || r.target.classList?.contains("uhl-layer"))) return;
    if (records.every((r) => [...r.addedNodes, ...r.removedNodes].every((n) => n.classList?.contains("uhl-layer")) && r.type === "childList" && (r.addedNodes.length || r.removedNodes.length))) return;
    redraw();
  }).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["hidden", "class", "data-read-size", "data-read-spacing", "data-read-width"], characterData: true });

  H.start = (init, send) => {
    H.lesson = init.lesson;
    H.list = init.list || [];
    H.labels = init.labels || {};
    H.send = send;
    H.on = true;
    if (bar) { bar.remove(); bar = null; }
    redraw();
  };
  H.stop = () => {
    H.on = false;
    H.send = null;
    hide();
    clear();
  };
  H.locate = (text, h) => locate({ chars: Array.from(text) }, h);
})();
