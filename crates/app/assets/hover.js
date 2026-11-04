// Steps hover peeks (PARITY.md §6.3, §6.4): a small card that shows while
// the S Pen hovers (in range, not touching) or the mouse rests:
//   - over an element with data-peek: its text (a book page's quote, an MCQ
//     option's `why` after answering);
//   - over any English word that is in the glossary: its expansion.
// Settings turn pen and mouse peeks off per device: data-pen-hover="off" /
// data-mouse-hover="off" on <html>. A finger has no hover, so a tap on a
// glossary word in plain text, or on a plain element with data-peek, shows
// the same card, and the next tap or a scroll hides it (no dead
// highlighted words on the phone). The book page chip is a button that
// opens its own sheet.
// Adapted from batna-app main: crates/app/assets/reader.js (pointer plumbing).
(() => {
  if (window.__stepsHover) return;
  const state = { exact: {}, lower: {}, courses: {}, unverified: "", timer: 0, shownFor: null };
  const card = document.createElement("div");
  card.className = "peek";
  card.setAttribute("role", "tooltip");
  card.hidden = true;
  document.body.appendChild(card);

  const root = document.documentElement;
  const enabled = (type) =>
    (type === "pen" ? root.dataset.penHover : root.dataset.mouseHover) !== "off";

  // The word under the pointer, from the text itself: the DOM is Dioxus's,
  // so nothing is wrapped or changed.
  function wordAt(x, y) {
    let node, offset;
    if (document.caretPositionFromPoint) {
      const p = document.caretPositionFromPoint(x, y);
      if (!p) return null;
      node = p.offsetNode;
      offset = p.offset;
    } else if (document.caretRangeFromPoint) {
      const r = document.caretRangeFromPoint(x, y);
      if (!r) return null;
      node = r.startContainer;
      offset = r.startOffset;
    } else return null;
    if (!node || node.nodeType !== Node.TEXT_NODE) return null;
    const text = node.textContent;
    const isWord = (c) => /[A-Za-z0-9-]/.test(c);
    let a = offset, b = offset;
    while (a > 0 && isWord(text[a - 1])) a--;
    while (b < text.length && isWord(text[b])) b++;
    if (a === b) return null;
    // The caret lands beside the nearest letter: make sure the pointer is
    // really on the word, not in the margin next to it.
    const range = document.createRange();
    range.setStart(node, a);
    range.setEnd(node, b);
    const rect = range.getBoundingClientRect();
    if (x < rect.left - 2 || x > rect.right + 2 || y < rect.top - 2 || y > rect.bottom + 2) return null;
    return { word: text.slice(a, b).replace(/^-+|-+$/g, ""), rect, node, a };
  }

  // A word inside an element marked data-course (an item, a question, a
  // case, an oral prompt) reads that course's own meaning first (#318).
  function courseOf(node) {
    const el = node && (node.nodeType === Node.TEXT_NODE ? node.parentElement : node);
    const c = el && el.closest ? el.closest("[data-course]") : null;
    return c ? state.courses[c.dataset.course] : null;
  }

  function lookup(word, node) {
    if (!word) return null;
    const own = courseOf(node);
    if (own) {
      const hit = (own.exact && own.exact[word]) || (own.lower && own.lower[word.toLowerCase()]);
      if (hit) return hit;
    }
    return state.exact[word] || state.lower[word.toLowerCase()] || null;
  }

  function show(text, note, rect) {
    card.textContent = "";
    const body = document.createElement("span");
    body.lang = "en";
    body.dir = "ltr";
    body.textContent = text;
    card.appendChild(body);
    if (note) {
      const n = document.createElement("small");
      n.className = "peek-note";
      n.textContent = note;
      card.appendChild(n);
    }
    card.hidden = false;
    const w = card.offsetWidth, h = card.offsetHeight;
    const vw = window.innerWidth, vh = window.innerHeight;
    let left = Math.min(Math.max(8, rect.left + rect.width / 2 - w / 2), vw - w - 8);
    let top = rect.top - h - 8;
    if (top < 8) top = Math.min(rect.bottom + 8, vh - h - 8);
    card.style.left = left + "px";
    card.style.top = top + "px";
  }

  function hide() {
    clearTimeout(state.timer);
    card.hidden = true;
    state.shownFor = null;
  }

  function peekAt(x, y, target) {
    const el = target && target.closest ? target.closest("[data-peek]") : null;
    if (el && el.dataset.peek) {
      if (state.shownFor === el) return;
      state.shownFor = el;
      show(el.dataset.peek, el.dataset.peekNote || "", el.getBoundingClientRect());
      return;
    }
    const hit = wordAt(x, y);
    const entry = hit && lookup(hit.word, hit.node);
    if (!entry) return hide();
    const key = hit.word + "@" + hit.a;
    if (state.shownFor === key) return;
    state.shownFor = key;
    show(entry[0], entry[1] ? "" : state.unverified, hit.rect);
  }

  document.addEventListener("pointermove", (e) => {
    // Hover is the mouse, or a pen in range that is not touching the screen.
    const hovering = e.pointerType === "mouse" || (e.pointerType === "pen" && e.buttons === 0);
    if (!hovering) return;
    if (!enabled(e.pointerType)) return hide();
    clearTimeout(state.timer);
    const { clientX: x, clientY: y, target } = e;
    state.timer = setTimeout(() => peekAt(x, y, target), e.pointerType === "pen" ? 60 : 250);
  }, { passive: true });
  // Touch: a tap on a glossary word or a plain data-peek element (not on
  // a control) toggles its card.
  let touchShown = null;
  let lastTouch = 0;
  document.addEventListener("pointerup", (e) => {
    if (e.pointerType === "touch") lastTouch = Date.now();
  }, { passive: true, capture: true });
  document.addEventListener("pointerdown", (e) => {
    touchShown = e.pointerType === "touch" ? state.shownFor : null;
    hide();
  }, { passive: true });
  document.addEventListener("click", (e) => {
    if (!lastTouch || Date.now() - lastTouch > 800) return;
    const t = e.target;
    if (!t || !t.closest || t.closest("button, a, input, textarea, select, summary, label, [role=button], .card-action")) return;
    const el = t.closest("[data-peek]");
    if (el && el.dataset.peek) {
      if (touchShown === el) return; // the same element again: leave it closed
      state.shownFor = el;
      show(el.dataset.peek, el.dataset.peekNote || "", el.getBoundingClientRect());
      return;
    }
    const hit = wordAt(e.clientX, e.clientY);
    const entry = hit && lookup(hit.word, hit.node);
    if (!entry) return;
    const key = hit.word + "@" + hit.a;
    if (touchShown === key) return;
    state.shownFor = key;
    show(entry[0], entry[1] ? "" : state.unverified, hit.rect);
  });
  document.addEventListener("pointercancel", hide, { passive: true });
  // The pen lifting out of range, or the mouse leaving the window: the
  // pointer goes out to nothing. pointerleave doesn't bubble, so it is caught
  // on the way down, and only for the page itself, so crossing from one
  // element to the next doesn't blink the card. hide() also drops a peek
  // still waiting on its timer.
  document.addEventListener("pointerout", (e) => {
    if (e.pointerType !== "touch" && !e.relatedTarget) hide();
  }, { passive: true });
  document.addEventListener("pointerleave", (e) => {
    if (e.pointerType !== "touch" && e.target === root) hide();
  }, { passive: true, capture: true });
  document.addEventListener("scroll", hide, { passive: true, capture: true });
  window.addEventListener("blur", hide);
  // Android Back and screen changes take the peek's target away (#322).
  window.addEventListener("popstate", hide);

  window.__stepsHover = {
    // exact: { term: [text, approved] } for abbreviations and drugs;
    // lower: { lowercased drug: [text, approved] }.
    setGlossary(g, unverifiedNote) {
      state.exact = g.exact || {};
      state.lower = g.lower || {};
      state.unverified = unverifiedNote || "";
    },
    // { course: { exact, lower } }: each course's meanings that differ
    // from the shared table.
    setCourses(c) {
      state.courses = c || {};
    },
    hide,
    visible: () => !card.hidden,
  };
})();
