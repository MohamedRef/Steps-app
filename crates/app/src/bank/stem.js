// A paper's stem highlights (bank/stem.rs). Words selected inside a stem
// (`.q-stem[data-hl]`) are sent to Rust as "question:start:end", counted in
// code points of the stem's text (steps_core::exam::stem counts the same),
// and the selection is let go; Rust draws the <mark> and keeps it with the
// paper. A mouse or pen sends when the button is let go; a finger's
// selection is sent once its handles rest, so it can still be widened.
(() => {
  if (window.__stepsStem) return;
  const S = (window.__stepsStem = { send: null });
  let touch = false;
  let timer = 0;

  // Code points of the stem's text before (node, offset).
  function count(stem, node, offset) {
    const r = document.createRange();
    r.setStart(stem, 0);
    try { r.setEnd(node, offset); } catch (_) { return -1; }
    return Array.from(r.toString()).length;
  }

  function take() {
    clearTimeout(timer);
    const sel = window.getSelection();
    if (!S.send || !sel || sel.isCollapsed || !sel.rangeCount) return;
    const r = sel.getRangeAt(0);
    const host = (n) => (n.nodeType === 1 ? n : n.parentElement)?.closest?.(".q-stem[data-hl]");
    const stem = host(r.startContainer);
    if (!stem || stem !== host(r.endContainer)) return;
    let a = count(stem, r.startContainer, r.startOffset);
    let b = count(stem, r.endContainer, r.endOffset);
    if (a < 0 || b <= a) return;
    // A double click's trailing space is not part of the words.
    const text = Array.from(stem.textContent);
    while (a < b && /\s/.test(text[a])) a++;
    while (b > a && /\s/.test(text[b - 1])) b--;
    if (b <= a) return;
    sel.removeAllRanges();
    S.send(`${stem.dataset.hl}:${a}:${b}`);
  }

  document.addEventListener("pointerdown", (e) => { touch = e.pointerType === "touch"; }, true);
  document.addEventListener("pointerup", () => { if (!touch) setTimeout(take, 0); }, true);
  // Shift and the arrows select from the keyboard.
  document.addEventListener("keyup", (e) => { if (e.key === "Shift") take(); }, true);
  document.addEventListener("selectionchange", () => {
    if (!touch) return;
    clearTimeout(timer);
    timer = setTimeout(take, 900);
  });

  S.start = (send) => { S.send = send; };
  S.stop = () => { S.send = null; clearTimeout(timer); };
})();
