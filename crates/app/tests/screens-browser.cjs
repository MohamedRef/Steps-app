// Browser layout checks of HTML rendered from the production lesson player
// and study screen (learn/browser.rs, study/browser.rs) with the app's own
// stylesheets. No Dioxus hydration: taps, saving, grading, native Back and
// device QA are separate. Usage: screens-browser.cjs <fixtures> <results>
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');

const WIDTHS = [320, 384, 600, 840, 1280];
const SCHEMES = ['light', 'dark'];
const APPEARANCES = ['calm', 'glossy'];
const SIZES = ['normal', 'larger'];
// The floor each screen keeps: the lesson player promises the shared 48dp
// `--target` (assets/parts.css) for every control; the study screen sizes
// its buttons with the app's `--tap`/`--tap-lg` tokens (assets/main.css), so
// it is held to `--tap` (at most 48px) here and its controls under 48px are counted in the
// report as a known gap.
const FLOOR = { learn: 'var(--target)', study: 'min(var(--tap), var(--target))' };

// Gaps on main when this suite was added (2026-10-30). They are left out of
// the failures, listed in report.json and the log so they stay visible, and
// each is a separate fix (docs/SCREENS-BROWSER-QA.md). Remove an entry with
// its fix; an entry nothing matches any more is reported as stale.
const KNOWN = {
  // [screen, selector]: controls still under the screen's floor.
  small: [],
  // States whose controls or content are wider than the screen.
  overflow: [],
  // Running study items have no heading for screen readers to jump to.
  noHeading: [],
};

// Everything a finger or the keyboard can reach on screen, with the layout
// facts the assertions need. Runs in the page.
function measure([floor, known]) {
  const probe = document.createElement('div');
  probe.style.blockSize = floor;
  document.querySelector('.app').append(probe);
  const target = probe.getBoundingClientRect().height;
  probe.remove();
  const shown = el => el.getClientRects().length > 0 && !el.closest('[hidden], [inert]') &&
    getComputedStyle(el).visibility !== 'hidden';
  const main = document.querySelector('main.content');
  const label = el => {
    const by = el.getAttribute('aria-labelledby');
    return (el.getAttribute('aria-label') || (by && document.getElementById(by)?.textContent) ||
      (el.labels && [...el.labels].map(l => l.textContent).join(' ')) || el.getAttribute('title') ||
      el.getAttribute('placeholder') || el.textContent || '').trim();
  };
  const describe = el => {
    const r = el.getBoundingClientRect();
    const cls = typeof el.className === 'string' ? el.className : '';
    return `${el.tagName.toLowerCase()}.${cls.trim().replace(/\s+/g, '.')} "${label(el).slice(0, 30)}" ` +
      `${Math.round(r.width)}x${Math.round(r.height)}@${Math.round(r.left)}`;
  };
  // Links inside running text are exempt from the target size (WCAG 2.5.8).
  const inline = el => el.tagName === 'A' && getComputedStyle(el).display === 'inline';
  const controls = [...main.querySelectorAll(
    'button, a[href], input:not([type=hidden]), select, textarea, summary, [role=button], [role=radio], [role=checkbox], [role=tab], [tabindex]:not([tabindex="-1"])',
  )].filter(el => shown(el) && !el.disabled);
  const small = controls.filter(el => {
    if (inline(el)) return false;
    // A control inside a 48dp wrapper (a label around a checkbox) counts
    // as the wrapper.
    const hit = el.closest('label') || el;
    return hit.getBoundingClientRect().height < target - 0.5;
  });
  const knownAs = el => known.find(sel => el.matches(sel));
  const under48 = controls.filter(el => !inline(el) && (el.closest('label') || el).getBoundingClientRect().height < 47.5);
  const offscreen = controls.filter(el => {
    const r = el.getBoundingClientRect();
    // Inside a sideways scroller (a wide table) is reachable by scrolling.
    if (el.closest('.ls-table-scroll, .table-scroll, [data-scroll-x]')) return false;
    return r.left < -1 || r.right > innerWidth + 1;
  });
  return {
    direction: getComputedStyle(document.querySelector('.app')).direction,
    documentWidth: document.documentElement.scrollWidth,
    mainOverflow: main.scrollWidth - main.clientWidth,
    h1: [...main.querySelectorAll('h1')].filter(shown).map(h => h.textContent.trim()),
    back: [...main.querySelectorAll('[data-back="true"]')].filter(shown).map(describe),
    controls: controls.length,
    tooSmall: small.filter(el => !knownAs(el)).map(describe),
    target,
    under48: under48.length,
    knownSmall: [...new Set(small.map(knownAs).filter(Boolean))],
    offscreen: offscreen.map(describe),
    unnamed: controls.filter(el => !label(el)).map(describe),
  };
}

// What each screen state must show, beyond the shared layout rules.
async function screenChecks(page, item, name) {
  if (item.screen === 'learn') {
    const s = await page.evaluate(() => {
      const shown = [...document.querySelectorAll('.ls-stage')].filter(el => !el.hidden && el.getClientRects().length);
      const meter = document.querySelector('.ls-segs');
      const next = document.querySelector('.ls-nav [data-nav="next"], .ls-nav [data-nav="check"]');
      return {
        view: document.querySelector('.ls-player')?.dataset.view,
        stages: shown.map(el => [el.dataset.kind, el.dataset.n]),
        meter: meter && [meter.getAttribute('role'), meter.getAttribute('aria-valuenow'), meter.getAttribute('aria-valuemax')],
        next: Boolean(next && next.getClientRects().length),
        skip: Boolean(document.querySelector('.ls-nav .ls-skip')),
        hint: document.querySelector('#ls-gate-hint')?.className,
      };
    });
    assert.equal(s.view, item.view, `${name}: view`);
    assert.deepEqual(s.stages, [[item.kind, String(item.step)]], `${name}: exactly the saved step is shown`);
    assert.deepEqual(s.meter, ['progressbar', String(item.step), String(item.steps)], `${name}: step meter`);
    assert.ok(s.next, `${name}: a visible way forward`);
    if (item.question && item.kind !== 'case') {
      assert.ok(s.skip, `${name}: an unanswered question offers Skip`);
      assert.equal(s.hint, 'ls-sr-only', `${name}: the gate note is for screen readers only`);
    }
  } else {
    const s = await page.evaluate(() => ({
      kind: document.querySelector('.st-item')?.dataset.kind,
      bar: document.querySelector('.st-bar')?.getAttribute('role'),
      grade: document.querySelectorAll('.st-item button').length,
    }));
    if (item.phase === 'running') {
      assert.equal(s.kind, item.kind, `${name}: item kind`);
      assert.equal(s.bar, 'progressbar', `${name}: session progress`);
      assert.ok(s.grade > 0, `${name}: the item has a control`);
    }
  }
}

// Every lesson figure (learn/browser.rs `export_screen_fixtures_figures`):
// each label inside its picture. A label past the viewBox edge is cut off on
// the phone ("first decompensation: insidiou", 2026-11-01). The viewBox is
// in user units, so one width shows every overflow.
async function figureChecks(page, fixtures) {
  await page.setViewportSize({ width: 384, height: 900 });
  await page.goto(pathToFileURL(path.join(fixtures, 'figures', 'figures.html')).href);
  await page.evaluate(() => document.fonts.ready);
  const r = await page.evaluate(() => {
    const out = [];
    const figures = [...document.querySelectorAll('.ls-fig-svg')];
    for (const fig of figures) {
      const svg = fig.querySelector('svg');
      const vb = svg.viewBox.baseVal;
      const m = svg.getScreenCTM();
      const a = new DOMPoint(vb.x, vb.y).matrixTransform(m);
      const b = new DOMPoint(vb.x + vb.width, vb.y + vb.height).matrixTransform(m);
      const [left, right, top, bottom] = [Math.min(a.x, b.x), Math.max(a.x, b.x), Math.min(a.y, b.y), Math.max(a.y, b.y)];
      for (const t of svg.querySelectorAll('text')) {
        const box = t.getBoundingClientRect();
        if (!box.width) continue;
        const px = Math.max(left - box.left, box.right - right, top - box.top, box.bottom - bottom);
        if (px > 1) out.push(`${fig.dataset.at}: "${t.textContent.trim().slice(0, 50)}" runs ${Math.round(px)}px past the edge`);
      }
    }
    return { figures: figures.length, out };
  });
  assert.deepEqual(r.out, [], 'figure labels cut off at the picture edge');
  console.log(`Checked the labels of ${r.figures} lesson figures.`);
}

async function run() {
  const fixtures = path.resolve(process.argv[2]);
  const output = path.resolve(process.argv[3] || path.join(fixtures, 'results'));
  await fs.mkdir(output, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ reducedMotion: 'reduce' });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const results = [];
  const known = [];
  const seen = new Set();
  let current = '';
  try {
    for (const screen of ['learn', 'study']) {
      const dir = path.join(fixtures, screen);
      const manifest = JSON.parse(await fs.readFile(path.join(dir, 'manifest.json'), 'utf8'));
      const knownSmall = KNOWN.small.filter(([s]) => s === screen).map(([, sel]) => sel);
      for (const item of manifest) {
        await page.goto(pathToFileURL(path.join(dir, item.file)).href);
        await page.evaluate(() => document.fonts.ready);
        for (const scheme of SCHEMES) {
          await page.emulateMedia({ colorScheme: scheme });
          for (const appearance of APPEARANCES) {
            for (const size of SIZES) {
              // The same switch the app makes: the prefs stylesheet and the
              // appearance key on .app (main.rs).
              await page.evaluate(([appearance, size]) => new Promise(resolve => {
                document.querySelector('.app').dataset.appearance = appearance;
                const link = document.querySelector('#prefs');
                const href = `prefs-system-${size}.css`;
                if (link.getAttribute('href') === href) return resolve();
                link.onload = resolve;
                link.setAttribute('href', href);
              }), [appearance, size]);
              for (const width of WIDTHS) {
                const name = `${item.file.replace(/\.html$/, '')}-${width}-${scheme}-${appearance}-${size}`;
                current = name;
                await page.setViewportSize({ width, height: 900 });
                const m = await page.evaluate(measure, [FLOOR[screen], knownSmall]);
                assert.equal(m.direction, 'rtl', name);
                const overflowKnown = KNOWN.overflow.find(k => k.file === item.file && k.width === width && k.size === size);
                const wide = m.documentWidth > width + 1 || m.mainOverflow > 1;
                if (overflowKnown && wide) {
                  seen.add(overflowKnown);
                  known.push({ name, gap: 'overflow', px: Math.max(m.documentWidth - width, m.mainOverflow) });
                } else {
                  assert.ok(m.documentWidth <= width + 1, `${name}: page wider than the screen ${JSON.stringify(m)}`);
                  assert.ok(m.mainOverflow <= 1, `${name}: content overflows sideways by ${m.mainOverflow}px`);
                  assert.deepEqual(m.offscreen, [], `${name}: controls outside the screen`);
                }
                const headingKnown = KNOWN.noHeading.find(k => k.screen === item.screen && k.phase === item.phase);
                if (headingKnown && !m.h1.length) {
                  seen.add(headingKnown);
                  known.push({ name, gap: 'no heading' });
                } else {
                  assert.ok(m.h1.length >= 1, `${name}: a visible heading`);
                }
                for (const sel of m.knownSmall) {
                  seen.add(KNOWN.small.find(([s, k]) => s === screen && k === sel));
                  known.push({ name, gap: `under ${FLOOR[screen]}`, selector: sel });
                }
                assert.ok(m.back.length >= 1 || item.phase === 'setup' || item.phase === 'summary',
                  `${name}: a visible Back target for Escape and Android Back`);
                assert.deepEqual(m.unnamed, [], `${name}: controls without a name`);
                assert.deepEqual(m.tooSmall, [], `${name}: controls under ${FLOOR[screen]} (${m.target}px)`);
                await screenChecks(page, item, name);
                if (width === 384 && appearance === 'calm' && size === 'larger') {
                  await page.screenshot({ path: path.join(output, `${name}.png`), fullPage: true });
                }
                results.push({ ...item, width, scheme, appearance, size, controls: m.controls, under48: m.under48 });
              }
            }
          }
        }
      }
    }
    current = 'figures';
    await figureChecks(page, fixtures);
    assert.deepEqual(errors, [], 'browser page errors');
    const stale = [...KNOWN.small, ...KNOWN.overflow, ...KNOWN.noHeading].filter(k => !seen.has(k));
    await fs.writeFile(path.join(output, 'report.json'),
      JSON.stringify({ cases: results.length, known, stale, results }, null, 2));
    const gaps = [...new Set(known.map(k => `${k.gap}${k.selector ? ` ${k.selector}` : ''}`))];
    console.log(`Passed ${results.length} production lesson player and study layout cases.`);
    const study48 = results.filter(r => r.screen === 'study' && r.under48 > 0).length;
    console.log(`Study cases with a control under 48px (sized by --tap): ${study48} of ${results.filter(r => r.screen === 'study').length}.`);
    console.log(`Known gaps still present (${gaps.length}):\n  ${gaps.join('\n  ')}`);
    for (const k of stale) console.warn(`::warning::Known gap no longer seen, remove it from KNOWN: ${JSON.stringify(k)}`);
  } catch (error) {
    console.error(`Failed at ${current}`);
    await page.screenshot({ path: path.join(output, 'failure.png'), fullPage: true }).catch(() => {});
    throw error;
  } finally {
    await browser.close();
  }
}

run().catch(error => { console.error(error); process.exitCode = 1; });
