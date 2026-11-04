// The full-screen figure viewer (steps.rs Figure): zoom with the − / 1:1 /
// + buttons, a two-finger pinch, or a double tap, from 1× (the figure fits
// the width) to 5×. One finger pans natively; the scroller is left to
// right, so a zoomed English figure opens at its top left (#337).
(() => {
  if (window.__stepsViewer) return;
  window.__stepsViewer = true;
  const MIN = 1, MAX = 5;

  function setScale(v, s, cx, cy) {
    const sc = v.querySelector(".ls-viewer-scroll");
    const inner = v.querySelector(".ls-viewer-inner");
    if (!sc || !inner) return;
    const old = +(v.dataset.scale || 1);
    s = Math.min(MAX, Math.max(MIN, s));
    const r = sc.getBoundingClientRect();
    // Keep the point under the fingers (or the middle) where it is.
    const px = (cx ?? r.left + r.width / 2) - r.left;
    const py = (cy ?? r.top + r.height / 2) - r.top;
    const fx = (sc.scrollLeft + px) / old;
    const fy = (sc.scrollTop + py) / old;
    v.dataset.scale = String(s);
    inner.style.width = s * 100 + "%";
    sc.scrollLeft = fx * s - px;
    sc.scrollTop = fy * s - py;
  }
  const viewer = (t) => (t && t.closest ? t.closest(".ls-viewer") : null);

  document.addEventListener("click", (e) => {
    const b = e.target.closest && e.target.closest(".ls-viewer [data-zoom]");
    if (!b) return;
    const v = viewer(b);
    const s = +(v.dataset.scale || 1);
    const z = b.dataset.zoom;
    setScale(v, z === "in" ? s * 1.5 : z === "out" ? s / 1.5 : 1);
  });

  // Double tap (or double click): 1× ↔ 2.5× at that point.
  let lastTap = { t: 0, x: 0, y: 0 };
  document.addEventListener("pointerup", (e) => {
    const sc = e.target.closest && e.target.closest(".ls-viewer-scroll");
    if (!sc || !e.isPrimary) return;
    const now = performance.now();
    if (now - lastTap.t < 320 && Math.hypot(e.clientX - lastTap.x, e.clientY - lastTap.y) < 30) {
      const v = viewer(sc);
      setScale(v, +(v.dataset.scale || 1) > 1.2 ? 1 : 2.5, e.clientX, e.clientY);
      lastTap = { t: 0, x: 0, y: 0 };
    } else {
      lastTap = { t: now, x: e.clientX, y: e.clientY };
    }
  }, { passive: true });

  // Pinch: two fingers scale around their midpoint.
  let pinch = null;
  const dist = (t) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
  const mid = (t) => [(t[0].clientX + t[1].clientX) / 2, (t[0].clientY + t[1].clientY) / 2];
  document.addEventListener("touchstart", (e) => {
    const v = viewer(e.target);
    if (v && e.touches.length === 2) pinch = { v, d: dist(e.touches), s: +(v.dataset.scale || 1) };
  }, { passive: true });
  document.addEventListener("touchmove", (e) => {
    if (!pinch || e.touches.length !== 2) return;
    e.preventDefault();
    const [x, y] = mid(e.touches);
    setScale(pinch.v, pinch.s * (dist(e.touches) / pinch.d), x, y);
  }, { passive: false });
  document.addEventListener("touchend", (e) => {
    if (e.touches.length < 2) pinch = null;
  }, { passive: true });

  // Ctrl + wheel (a trackpad pinch on the Mac) zooms too.
  document.addEventListener("wheel", (e) => {
    const v = viewer(e.target);
    if (!v || !e.ctrlKey) return;
    e.preventDefault();
    setScale(v, +(v.dataset.scale || 1) * Math.exp(-e.deltaY / 200), e.clientX, e.clientY);
  }, { passive: false });
})();
