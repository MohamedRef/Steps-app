// Drag and drop for the lesson sort step (steps/sorting.rs). A mouse drags
// a card once it moves a few pixels; a finger long-presses first, so a
// plain swipe still scrolls the lesson. Dropping replays the step's own
// taps (card, then bin), so the rules stay in Rust and tap-then-bin keeps
// working exactly as before. A card already in a bin can be dragged to
// another bin, or back to the pool.
(() => {
  if (window.__stepsSortDrag) return;
  window.__stepsSortDrag = true;
  const CARD = ".ls-sort .ls-tile[data-card], .ls-sort .ls-chiplet[data-card]";
  const PRESS_MS = 180;
  let d = null; // { id, el, sort, x, y, on, timer, ghost, over, dx, dy, scroller, raf }
  let swallowUntil = 0;

  const scrollerOf = (el) => {
    for (let p = el.parentElement; p; p = p.parentElement) {
      const s = getComputedStyle(p).overflowY;
      if ((s === "auto" || s === "scroll") && p.scrollHeight > p.clientHeight) return p;
    }
    return document.scrollingElement || document.documentElement;
  };

  function start() {
    const r = d.el.getBoundingClientRect();
    const g = d.el.cloneNode(true);
    g.classList.add("ls-ghost");
    g.removeAttribute("data-card");
    g.setAttribute("aria-hidden", "true");
    // The ghost lives on <body>, outside the lesson's own type styles.
    const cs = getComputedStyle(d.el);
    Object.assign(g.style, {
      inlineSize: `${r.width}px`,
      left: `${r.left}px`,
      top: `${r.top}px`,
      font: cs.font,
      letterSpacing: cs.letterSpacing,
    });
    document.body.appendChild(g);
    d.ghost = g;
    d.dx = d.x - r.left;
    d.dy = d.y - r.top;
    d.on = true;
    d.el.setAttribute("data-dragging", "yes");
    d.sort.setAttribute("data-drag", "yes");
    d.scroller = scrollerOf(d.sort);
    if (navigator.vibrate) try { navigator.vibrate(8); } catch (_) {}
    d.raf = requestAnimationFrame(edgeScroll);
  }

  // Where the finger is: a bin (by index), the pool (-1), or nowhere.
  function targetAt(x, y) {
    const hit = document.elementFromPoint(x, y);
    if (!hit || !d.sort.contains(hit)) return null;
    const bin = hit.closest(".ls-bin[data-bin]");
    if (bin) return bin;
    if (d.el.matches(".ls-chiplet") && hit.closest(".ls-pool, .ls-sort-bar")) return d.sort.querySelector(".ls-pool");
    return null;
  }

  function move(x, y) {
    d.cx = x;
    d.cy = y;
    d.ghost.style.left = `${x - d.dx}px`;
    d.ghost.style.top = `${y - d.dy}px`;
    const t = targetAt(x, y);
    if (t !== d.over) {
      d.over?.removeAttribute("data-over");
      t?.setAttribute("data-over", "yes");
      d.over = t;
    }
  }

  // Bins below the fold: holding a card near the top or bottom edge scrolls.
  function edgeScroll() {
    if (!d || !d.on) return;
    const s = d.scroller;
    const box = s === document.scrollingElement ? { top: 0, bottom: innerHeight } : s.getBoundingClientRect();
    const nav = document.querySelector(".ls-nav");
    const bottom = Math.min(box.bottom, nav ? nav.getBoundingClientRect().top : box.bottom);
    const edge = 72;
    let v = 0;
    if (d.cy !== undefined) {
      if (d.cy < box.top + edge) v = -Math.ceil(((box.top + edge - d.cy) / edge) * 14);
      else if (d.cy > bottom - edge) v = Math.ceil(((d.cy - (bottom - edge)) / edge) * 14);
    }
    if (v) {
      s.scrollTop += v;
      move(d.cx, d.cy);
    }
    d.raf = requestAnimationFrame(edgeScroll);
  }

  // Once Dioxus has redrawn, the card is back in the pool under its index.
  function whenTile(sort, card, then, tries = 30) {
    const t = sort.querySelector(`.ls-pool .ls-tile[data-card="${card}"]`);
    if (t) return then(t);
    if (tries > 0) requestAnimationFrame(() => whenTile(sort, card, then, tries - 1));
  }

  function drop() {
    const { el, sort, over } = d;
    const card = el.getAttribute("data-card");
    const bin = over?.getAttribute("data-bin");
    const toBin = (tile) => {
      if (tile.getAttribute("data-state") !== "picked") tile.click();
      sort.querySelector(`.ls-bin[data-bin="${bin}"]`)?.click();
    };
    if (el.matches(".ls-tile")) {
      if (bin != null) toBin(el);
    } else if (over && bin !== el.closest(".ls-bin")?.getAttribute("data-bin")) {
      el.click(); // back to the pool
      if (bin != null) whenTile(sort, card, toBin);
    }
  }

  function end(commit) {
    if (!d) return;
    clearTimeout(d.timer);
    if (d.on) {
      cancelAnimationFrame(d.raf);
      swallowUntil = performance.now() + 350;
      if (commit) drop();
      d.ghost.remove();
      d.over?.removeAttribute("data-over");
      d.el.removeAttribute("data-dragging");
      d.sort.removeAttribute("data-drag");
    }
    d = null;
  }

  document.addEventListener("pointerdown", (e) => {
    if (d || !e.isPrimary || e.button > 0 || !e.target.closest) return;
    const el = e.target.closest(CARD);
    if (!el || el.disabled) return;
    d = { id: e.pointerId, el, sort: el.closest(".ls-sort"), x: e.clientX, y: e.clientY, on: false, over: null };
    if (e.pointerType !== "mouse") {
      d.timer = setTimeout(() => {
        if (d && !d.on) {
          start();
          move(d.x, d.y);
        }
      }, PRESS_MS);
    }
  }, { passive: true });

  document.addEventListener("pointermove", (e) => {
    if (!d || e.pointerId !== d.id) return;
    if (!d.on) {
      const far = Math.hypot(e.clientX - d.x, e.clientY - d.y) > 6;
      if (!far) return;
      // A finger that moves before the long press is scrolling.
      if (e.pointerType !== "mouse") return end(false);
      start();
    }
    move(e.clientX, e.clientY);
  }, { passive: true });

  document.addEventListener("pointerup", (e) => d && e.pointerId === d.id && end(true));
  document.addEventListener("pointercancel", (e) => d && e.pointerId === d.id && end(false));
  // While a card is up, the finger moves the card, not the page.
  document.addEventListener("touchmove", (e) => d && d.on && e.cancelable && e.preventDefault(), { passive: false });
  document.addEventListener("contextmenu", (e) => {
    if (d || e.target.closest?.(CARD)) e.preventDefault();
  });
  // The click that ends a drag isn't a tap on whatever is under the finger.
  document.addEventListener("click", (e) => {
    if (e.isTrusted && performance.now() < swallowUntil) {
      e.stopPropagation();
      e.preventDefault();
    }
  }, true);
  window.addEventListener("blur", () => end(false));
})();
