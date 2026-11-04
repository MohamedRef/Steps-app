// Lesson swipe (player.rs): drag a lesson page sideways to turn it. The app
// is right to left, so a finger moving right brings the next page in from
// the left, the way an Arabic book turns. The page follows the finger, and
// letting go far or fast enough presses the footer's own Back or Continue
// button, so the player's rules stay in one place: an unanswered question
// doesn't turn (the page springs back and the footer nudges toward Skip or
// Check). Drags that start on tiles, tables, fields or the open figure
// viewer are theirs; a figure in the lesson turns the page like text (#338).
(() => {
  if (window.__stepsSwipe) return;
  window.__stepsSwipe = true;
  const NOT_HERE =
    ".ls-tile, .ls-slot, .ls-chiplet, .ls-pool, .ls-bins, .ls-order, .ls-match, .ls-viewer, .ls-table-scroll, .ls-mn-letters, input, textarea, select, [data-noswipe]";
  const rtl = () => (document.querySelector(".app")?.getAttribute("dir") || document.dir) === "rtl";
  const button = (fwd) => document.querySelector(`.ls-nav [data-nav=${fwd ? "next" : "back"}]:not(:disabled)`);
  let g = null;
  let swallowUntil = 0;

  function settle(el, animate) {
    el.style.transition = animate ? "transform 180ms ease, opacity 180ms ease" : "";
    el.style.transform = "";
    el.style.opacity = "";
    if (animate) setTimeout(() => (el.style.transition = ""), 200);
  }

  document.addEventListener("pointerdown", (e) => {
    // A second finger makes it a pinch or a mistake, never a turn (#322).
    if (g && e.pointerId !== g.id) {
      settle(g.el, true);
      g = null;
      return;
    }
    if (e.pointerType === "mouse" || !e.isPrimary || !e.target.closest) return;
    // Anywhere on the lesson turns it, the empty space under a short step
    // too (#311), except the top bar and the footer's buttons.
    const player = e.target.closest(".ls-player");
    if (!player || e.target.closest(".ls-bar, .ls-nav") || e.target.closest(NOT_HERE)) return;
    const el = player.querySelector(".ls-stage:not([hidden])");
    if (!el) return;
    g = { id: e.pointerId, x: e.clientX, y: e.clientY, t: performance.now(), el, on: false, dx: 0 };
  }, { passive: true });

  document.addEventListener("pointermove", (e) => {
    if (!g || e.pointerId !== g.id) return;
    const dx = e.clientX - g.x;
    const dy = e.clientY - g.y;
    if (!g.on) {
      // Mostly vertical: he's scrolling the page, not turning it.
      if (Math.abs(dy) > 12 && Math.abs(dy) >= Math.abs(dx)) return void (g = null);
      if (Math.abs(dx) < 12) return;
      g.on = true;
    }
    g.dx = dx;
    const fwd = rtl() ? dx > 0 : dx < 0;
    // Heavy, so it reads as "not yet", when that way is closed.
    const d = button(fwd) ? dx : dx * 0.25;
    g.el.style.transition = "";
    g.el.style.transform = `translateX(${d}px) rotate(${d / 80}deg)`;
    g.el.style.opacity = String(1 - Math.min(Math.abs(d) / 700, 0.3));
  }, { passive: true });

  document.addEventListener("pointerup", (e) => {
    if (!g || e.pointerId !== g.id) return;
    const { el, on, dx, t } = g;
    g = null;
    if (!on) return;
    // The click that ends a drag isn't a tap on whatever is under the finger.
    swallowUntil = performance.now() + 350;
    const speed = Math.abs(dx) / Math.max(performance.now() - t, 1);
    if (Math.abs(dx) < 80 && !(speed > 0.5 && Math.abs(dx) > 30)) return settle(el, true);
    const fwd = rtl() ? dx > 0 : dx < 0;
    const btn = button(fwd);
    if (!btn) {
      settle(el, true);
      const nav = document.querySelector(".ls-nav");
      if (fwd && nav) {
        nav.classList.remove("ls-nudge");
        void nav.offsetWidth;
        nav.classList.add("ls-nudge");
      }
      return;
    }
    settle(el, false);
    btn.click();
  }, { passive: true });

  document.addEventListener("pointercancel", (e) => {
    if (!g || e.pointerId !== g.id) return;
    settle(g.el, true);
    g = null;
  }, { passive: true });

  document.addEventListener("click", (e) => {
    if (performance.now() < swallowUntil && e.isTrusted && e.target.closest && e.target.closest(".ls-player")) {
      e.preventDefault();
      e.stopPropagation();
    }
  }, true);
})();
