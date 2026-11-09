// The divider between a study card's two panes (desk/mod.rs Panes).
// Adapted from batna-app main: crates/app/src/desk/resize.js (#741).
// Drag it, or focus it and use the arrows (5% a press) or Home/End; a
// double click goes back to the item's default. The drag follows the
// pointer here frame by frame; Rust keeps the width per device, so only
// the final value is sent ("split:N", or "split:0" for the default).
if (window.__stepsResize) {
  for (const [name, fn] of Object.entries(window.__stepsResize)) document.removeEventListener(name, fn, true);
}
const root = document.documentElement;
const MIN = 35, MAX = 70;
const bound = (n) => Math.min(MAX, Math.max(MIN, Math.round(n)));
// The divider only works where the panes stand side by side (desk.css).
const live = (h) => h && getComputedStyle(h).display !== "none";
const show = (pct) => {
  root.style.setProperty("--desk-split", pct + "%");
  document.querySelectorAll(".desk-resize").forEach((h) => {
    h.setAttribute("aria-valuenow", String(pct));
    h.setAttribute("aria-valuetext", pct + "%");
  });
};
const save = (pct) => dioxus.send("split:" + pct);
let drag = null;
const pointerdown = (e) => {
  const handle = e.target.closest?.(".desk-resize");
  if (!live(handle) || e.button > 0) return;
  const split = handle.closest(".desk-split");
  if (!split) return;
  drag = { handle, split, id: e.pointerId, pct: null };
  handle.setPointerCapture?.(e.pointerId);
  root.classList.add("desk-resizing");
  e.preventDefault();
};
const pointermove = (e) => {
  if (!drag || e.pointerId !== drag.id) return;
  const b = drag.split.getBoundingClientRect();
  if (!b.width) return;
  // The question pane is at the inline start: the right, in this rtl app.
  const rtl = getComputedStyle(drag.split).direction === "rtl";
  const x = rtl ? b.right - e.clientX : e.clientX - b.left;
  drag.pct = bound((100 * x) / b.width);
  show(drag.pct);
};
const end = (e, keep) => {
  if (!drag || e.pointerId !== drag.id) return;
  const { handle, pct } = drag;
  drag = null;
  root.classList.remove("desk-resizing");
  try { handle.releasePointerCapture?.(e.pointerId); } catch (_) {}
  // A plain click moves nothing; an interrupted drag isn't saved.
  if (keep && pct !== null) save(pct);
};
const pointerup = (e) => end(e, true);
const pointercancel = (e) => end(e, false);
const keydown = (e) => {
  const handle = e.target.closest?.(".desk-resize");
  if (!live(handle) || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
  const rtl = getComputedStyle(handle).direction === "rtl";
  const now = Number(handle.getAttribute("aria-valuenow")) || Number(handle.dataset.default) || 60;
  let pct = null;
  // The arrow points the way the divider moves.
  if (e.key === "ArrowLeft") pct = now + (rtl ? 5 : -5);
  else if (e.key === "ArrowRight") pct = now + (rtl ? -5 : 5);
  else if (e.key === "Home") pct = MIN;
  else if (e.key === "End") pct = MAX;
  if (pct === null) return;
  e.preventDefault();
  e.stopPropagation();
  pct = bound(pct);
  show(pct);
  save(pct);
};
const dblclick = (e) => {
  const handle = e.target.closest?.(".desk-resize");
  if (!live(handle)) return;
  root.style.removeProperty("--desk-split");
  document.querySelectorAll(".desk-resize").forEach((h) => {
    const d = h.dataset.default || "60";
    h.setAttribute("aria-valuenow", d);
    h.setAttribute("aria-valuetext", d + "%");
  });
  save(0);
};
const listeners = { pointerdown, pointermove, pointerup, pointercancel, keydown, dblclick };
window.__stepsResize = listeners;
for (const [name, fn] of Object.entries(listeners)) document.addEventListener(name, fn, true);
await new Promise(() => {});
