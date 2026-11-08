// Adapted from batna-app main: crates/app/tests/highlight-browser.cjs (#770).
// Your own highlights in a lesson (learn/highlight.js) against a real
// Chromium DOM: select words, pick a colour, the shape is drawn under the
// words and the add message names them; tap it to edit and remove it; a
// saved list draws again after a reload.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

const LABELS = { bar: 'هايلايت', colours: {}, fill: 'هايلايت', underline: 'خط تحت', pill: 'كبسولة',
  rect: 'مستطيل', remove: 'امسح' };

async function highlightChecks(browser, fixtures) {
  const script = await fs.readFile(path.join(__dirname, '../src/learn/highlight.js'), 'utf8');
  const learn = path.join(fixtures, 'learn');
  const file = (await fs.readdir(learn)).find(f => /^lesson-.*-steps\.html$/.test(f) && !f.includes('summary'));
  const page = await browser.newPage({ viewport: { width: 1280, height: 820 } });
  const start = async list => {
    await page.goto(pathToFileURL(path.join(learn, file)).href);
    await page.evaluate(([src, list, labels]) => {
      window.sent = [];
      (0, eval)(src);
      window.__stepsHl.start({ lesson: 'test', list, labels }, m => window.sent.push(m));
    }, [script, list, LABELS]);
  };
  try {
    await start([]);
    // The longest word of the open step's first paragraph-ish text.
    const word = await page.evaluate(() => {
      const st = document.querySelector('.ls-stage:not([hidden])');
      const w = document.createTreeWalker(st, NodeFilter.SHOW_TEXT);
      for (let n = w.nextNode(); n; n = w.nextNode()) {
        if (n.parentElement.closest('button, svg')) continue;
        const m = n.data.match(/[A-Za-z؀-ۿ]{5,}/);
        if (m) {
          const r = document.createRange();
          r.setStart(n, m.index);
          r.setEnd(n, m.index + m[0].length);
          getSelection().removeAllRanges();
          getSelection().addRange(r);
          return m[0];
        }
      }
      return null;
    });
    assert.ok(word, `${file}: a word to highlight`);
    await page.waitForSelector('.uhl-bar:not([hidden])', { timeout: 3000 });
    await page.click('.uhl-dot[data-c="yellow"]');
    await page.waitForFunction(() => document.querySelectorAll('.uhl-layer i').length > 0);
    const added = await page.evaluate(() => window.sent);
    assert.equal(added.length, 1, `one add message: ${added}`);
    const h = JSON.parse(added[0].slice(4));
    assert.equal(h.quote, word);
    assert.equal(h.colour, 'yellow');

    const at = await page.evaluate(() => {
      const r = document.querySelector('.uhl-layer i').getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    });
    await page.mouse.click(at.x, at.y);
    await page.waitForSelector('.uhl-del:not([hidden])', { timeout: 3000 });
    await page.click('.uhl-dot[data-c="green"]');
    await page.click('.uhl-del');
    const sent = await page.evaluate(() => window.sent);
    assert.ok(sent[1].startsWith('put:') && JSON.parse(sent[1].slice(4)).colour === 'green', `edit: ${sent}`);
    assert.equal(sent[2], `del:${h.id}`);
    await page.waitForFunction(() => document.querySelectorAll('.uhl-layer i').length === 0, null, { timeout: 3000 });

    // A saved highlight finds its words again on a fresh page.
    await start([h]);
    await page.waitForFunction(() => document.querySelectorAll('.uhl-layer i').length > 0, null, { timeout: 3000 });
    console.log(`Own highlights: select, colour, edit, remove and reload passed on ${file}.`);
  } finally {
    await page.close();
  }
}

module.exports = { highlightChecks };
