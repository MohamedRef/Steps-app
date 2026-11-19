// A paper on screen (bank/run.rs) in the app's one scroller, main.content.
// The sticky top bar is 4-9rem tall (two rows on a tablet), so its height is
// kept in --run-bar-h for the scroller's scroll-padding (bank.css): whatever
// is scrolled to stops below the bar, not behind it. An answered question's
// verdict is brought into view when it is not (UX-01). Focus moves without
// scrolling the page, and a sheet over the paper gives focus back to what
// opened it when it closes (UX-35). Safe to run more than once.
(() => {
  const R = (window.__stepsRun = window.__stepsRun || { openers: {} });
  const main = () => document.querySelector("main.content");
  const shown = (el) =>
    !!el && el.isConnected && !el.disabled && el.getClientRects().length > 0 &&
    !el.closest("[hidden], [inert], [aria-hidden=\"true\"]");
  // Focus that fell with a removed control rests on the body.
  const loose = () => {
    const a = document.activeElement;
    return !a || a === document.body || a === document.documentElement || !a.isConnected;
  };

  R.watch = () => {
    if (R.stop) R.stop();
    const bar = document.querySelector(".bank-run .run-sticky");
    const m = main();
    if (!bar || !m) return;
    const set = () => m.style.setProperty("--run-bar-h", `${Math.ceil(bar.getBoundingClientRect().height)}px`);
    const ro = new ResizeObserver(set);
    ro.observe(bar);
    set();
    R.stop = () => {
      ro.disconnect();
      m.style.removeProperty("--run-bar-h");
      R.stop = null;
    };
  };

  // The verdict in full, just under the bar: only when part of it is hidden.
  R.verdict = () => requestAnimationFrame(() => {
    const m = main();
    const v = document.querySelector(".bank-run .q-verdict");
    if (!m || !v) return;
    const view = m.getBoundingClientRect();
    const bar = document.querySelector(".bank-run .run-sticky");
    const foot = document.querySelector(".bank-run .run-foot");
    const top = bar ? Math.max(view.top, bar.getBoundingClientRect().bottom) : view.top;
    const bottom = shown(foot) ? Math.min(view.bottom, foot.getBoundingClientRect().top) : view.bottom;
    const r = v.getBoundingClientRect();
    if (r.top >= top && r.bottom <= bottom) return;
    m.scrollTop += r.top - top - 8;
  });

  // A control takes focus where it is; the page never scrolls to it.
  R.focus = (selector) => requestAnimationFrame(() => {
    const el = [...document.querySelectorAll(selector)].find(shown);
    if (el) el.focus({ preventScroll: true });
  });

  // A sheet opened: remember what had focus, then focus `selector` in it
  // (a long sheet scrolls itself to it; the page under it stays).
  R.opened = (key, selector) => requestAnimationFrame(() => {
    R.openers[key] = loose() ? null : document.activeElement;
    const el = [...document.querySelectorAll(selector)].find(shown);
    if (!el) return;
    el.focus({ preventScroll: true });
    if (el.closest(".sheet")) el.scrollIntoView({ block: "nearest" });
  });

  // The sheet closed: focus goes back to its opener, or to `fallback`,
  // unless something else (another sheet, a field) has taken it meanwhile.
  R.closed = (key, fallback) => requestAnimationFrame(() => {
    const opener = R.openers[key];
    delete R.openers[key];
    if (!loose()) return;
    const el = shown(opener) ? opener : [...document.querySelectorAll(fallback)].find(shown);
    if (el) el.focus({ preventScroll: true });
  });
})();
