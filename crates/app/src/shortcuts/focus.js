// Keeps keyboard focus on the current study task when the card it was on is
// replaced (reveal, grade, undo) or a lesson step is hidden (← / →, Enter).
// Pointer clicks keep their own focus and scroll; typing and an open dialog
// always win. It only moves focus: it never grades, reveals or navigates.
// Adapted from batna-app main: crates/app/src/shortcuts/focus.js (#745).
if (window.__stepsFocus?.stop) window.__stepsFocus.stop();

let pending = null;
let timeout = null;
let scheduled = false;
const $ = (selector, root = document) => root.querySelector(selector);
const modalOpen = () => Boolean($('[aria-modal="true"][role="dialog"], [aria-modal="true"][role="alertdialog"]'));
const editable = node => Boolean(node?.closest?.('input, textarea, select, [contenteditable], [role="textbox"]'));
const focusable = node => node && node.isConnected && !node.closest('[hidden], [inert], [aria-hidden="true"]') &&
  node.getClientRects().length && !node.disabled && getComputedStyle(node).visibility !== 'hidden';
const focus = node => {
  if (!focusable(node)) return false;
  node.focus({ preventScroll: true });
  return document.activeElement === node;
};
const activeStep = () => $('.ls-player .ls-stage:not([hidden])');
const activeItem = () => $('main.content[data-screen="Study"] .st-item');
const freshControl = root => [...root.querySelectorAll(
  'input:not([disabled]), textarea:not([disabled]), [contenteditable="true"], button[aria-keyshortcuts~="Space"]:not([disabled]), button[aria-keyshortcuts~="Enter"]:not([disabled])'
)].find(focusable);
const finish = () => {
  pending = null;
  if (timeout) clearTimeout(timeout);
  timeout = null;
};
const restore = () => {
  scheduled = false;
  const old = pending;
  if (!old) return;
  const active = document.activeElement;
  if (modalOpen() || editable(active) || (active && active !== document.body && active !== old.source &&
      active.isConnected && active !== document.documentElement)) {
    finish();
    return;
  }
  const main = $('main.content');
  if (!main || main.dataset.screen !== old.screen) { finish(); return; }
  if (old.screen === 'Study') {
    const item = activeItem();
    if (old.kind === 'grade' || old.kind === 'undo') {
      if (item && item !== old.item) {
        // No answer field or reveal button: the card itself (tabindex -1).
        const target = freshControl(item) || item;
        if (focus(target)) finish();
      } else if (!item) {
        const summary = $('.st-summary h1[tabindex="-1"]', main);
        if (focus(summary)) finish();
      }
    } else if (old.kind === 'reveal' && item === old.item && !old.hadGrades) {
      const good = $('.st-grades button[data-grade="good"]:not([disabled])', item);
      if (focus(good)) finish();
    }
  } else if (old.screen === 'Learn') {
    const step = activeStep();
    if (step && step !== old.step && focus(step)) finish();
    else if (!step) {
      const summary = $('.ls-summary h1[tabindex="-1"]', main);
      if (focus(summary)) finish();
    }
  }
};
const schedule = () => {
  if (scheduled || !pending) return;
  scheduled = true;
  queueMicrotask(restore);
};
const click = e => {
  // detail=0 is native keyboard/assistive-tech activation or a keyboard
  // shortcut's element.click(). A pointer click normally has detail > 0.
  if (e.detail !== 0 || modalOpen()) return;
  const button = e.target.closest?.('button');
  if (!button || button.disabled || !button.isConnected) return;
  const main = button.closest('main.content');
  if (!main) return;
  const screen = main.dataset.screen;
  let kind = null, item = null, step = null;
  if (screen === 'Study') {
    item = button.closest('.st-item');
    if (item) {
      if (button.matches('.rating[data-grade]')) kind = 'grade';
      else if (button.matches('[aria-keyshortcuts~="Space"], [aria-keyshortcuts~="Enter"]')) kind = 'reveal';
    } else if (button.matches('.st-progress .st-undo')) {
      kind = 'undo';
    }
  } else if (screen === 'Learn' && button.matches('.ls-nav button[data-nav]')) {
    kind = 'step';
    step = activeStep();
  }
  if (!kind) return;
  finish();
  pending = {
    screen, kind, source: button, item, step,
    hadGrades: Boolean(item && $('.st-grades', item))
  };
  timeout = setTimeout(finish, 800);
  schedule();
};
const pointerdown = () => finish();
const keydown = e => {
  // New user intent cancels a pending focus transfer, without treating
  // the activating Enter/Space as a competing intent.
  if (pending && ['Tab', 'Escape'].includes(e.key)) finish();
};
const observer = new MutationObserver(() => schedule());
observer.observe(document.documentElement, { childList: true, subtree: true, attributes: true,
  attributeFilter: ['hidden'] });
document.addEventListener('click', click, true);
document.addEventListener('pointerdown', pointerdown, true);
document.addEventListener('keydown', keydown, true);
window.__stepsFocus = {
  stop() {
    finish();
    observer.disconnect();
    document.removeEventListener('click', click, true);
    document.removeEventListener('pointerdown', pointerdown, true);
    document.removeEventListener('keydown', keydown, true);
  }
};
await new Promise(() => {});
