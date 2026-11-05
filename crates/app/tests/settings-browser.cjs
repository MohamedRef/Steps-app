// adapted from batna-app main: crates/app/tests/settings-browser.cjs
// Browser layout and keyboard checks of HTML rendered from production Settings.
// No Dioxus hydration: domain callbacks, native Back and device QA are separate.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');

async function run() {
  const fixtures = path.resolve(process.argv[2]);
  const output = path.resolve(process.argv[3] || path.join(fixtures, 'results'));
  await fs.mkdir(output, { recursive: true });
  const fixturesManifest = JSON.parse(await fs.readFile(path.join(fixtures, 'manifest.json'), 'utf8'));
  const manifest = fixturesManifest.flatMap(item => (item.theme.startsWith('system') ? ['light', 'dark'] : ['light']).map(scheme => ({ ...item, scheme })));
  const controller = await fs.readFile(path.join(__dirname, '../assets/settings-dialog.js'), 'utf8');
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ reducedMotion: 'reduce' });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const results = [];
  try {
    for (const item of manifest) {
      const name = item.file.replace(/\.html$/, '') + '-' + item.scheme;
      await page.setViewportSize({ width: item.width, height: 900 });
      await page.emulateMedia({ colorScheme: item.scheme });
      await page.goto(pathToFileURL(path.join(fixtures, item.file)).href);
      await page.evaluate(() => document.fonts.ready);
      const layout = await page.evaluate(() => {
        const root = document.querySelector('.app');
        const dialog = document.querySelector('.settings-window');
        const box = dialog.getBoundingClientRect();
        const pane = document.querySelector('.settings-pane');
        const list = document.querySelector('.settings-index');
        const search = document.querySelector('.settings-search input');
        const visible = el => Boolean(el && el.getClientRects().length);
        return {
          direction: getComputedStyle(root).direction,
          fontSize: parseFloat(getComputedStyle(root).fontSize),
          htmlFontSize: parseFloat(getComputedStyle(document.documentElement).fontSize),
          viewport: innerWidth,
          documentWidth: document.documentElement.scrollWidth,
          dialog: { left: box.left, right: box.right, width: box.width, height: box.height, inner: dialog.clientWidth },
          paneOverflow: visible(pane) ? pane.scrollWidth - pane.clientWidth : 0,
          listVisible: visible(list) || visible(search),
          title: document.querySelector('#settings-title')?.textContent.trim(),
          modal: dialog.getAttribute('aria-modal'),
          appearance: root.getAttribute('data-appearance'),
          settingsBackground: getComputedStyle(dialog, '::before').backgroundImage,
          settingsField: getComputedStyle(dialog, '::before').opacity,
          settingsCardBackground: getComputedStyle(pane.querySelector('.card')).backgroundColor,
          settingsCardShadow: getComputedStyle(pane.querySelector('.card')).boxShadow,
          role: dialog.getAttribute('role'),
          sample: document.querySelector('.text-sample')?.textContent.trim(),
          back: [...dialog.querySelectorAll('[data-back="true"]')].filter(visible).at(-1)?.className,
        };
      });
      assert.equal(layout.direction, item.dir, name);
      assert.equal(layout.appearance, item.appearance, `${name}: appearance preference`);
      if (item.appearance === 'liquid-glass') {
        assert.ok(layout.settingsBackground.includes('radial-gradient'), `${name}: glass must be visible in Settings`);
        const translucent = /^(rgba|color)\(.*(\/|,)\s*0?\.\d+\)$/.test(layout.settingsCardBackground);
        const trueBlack = item.theme === 'black' || (item.theme === 'system-black' && item.scheme === 'dark');
        if (trueBlack) {
          assert.equal(layout.settingsField, '0', `${name}: True black keeps its pixels off`);
          assert.ok(!translucent, `${name}: True black cards stay opaque, got ${layout.settingsCardBackground}`);
        } else {
          assert.equal(layout.settingsField, '1', `${name}: the colour field shows behind Settings`);
          assert.ok(translucent, `${name}: cards must be translucent glass, got ${layout.settingsCardBackground}`);
        }
        assert.notEqual(layout.settingsCardShadow, 'none', `${name}: card rim must be present`);
        const calmBackground = await page.locator('.settings-pane .card').first().evaluate(el => {
          const root = document.querySelector('.app');
          root.setAttribute('data-appearance', 'calm');
          const background = getComputedStyle(el).backgroundColor;
          root.setAttribute('data-appearance', 'liquid-glass');
          return background;
        });
        if (!trueBlack) {
          assert.notEqual(layout.settingsCardBackground, calmBackground, `${name}: appearance change must apply without reload`);
        }
      }
      assert.equal(layout.role, 'dialog', name);
      assert.equal(layout.modal, 'true', name);
      assert.ok(layout.title && layout.sample, `${name}: production heading and sample`);
      assert.ok(layout.documentWidth <= item.width + 1, `${name}: document overflow ${JSON.stringify(layout)}`);
      assert.ok(layout.dialog.left >= -1 && layout.dialog.right <= item.width + 1, `${name}: dialog outside viewport`);
      assert.ok(layout.dialog.width > 100 && layout.dialog.height > 100, `${name}: hidden dialog`);
      assert.ok(layout.paneOverflow <= 1, `${name}: pane overflow ${layout.paneOverflow}`);
      // Steps intentionally uses 0.9375rem on compact screens; verify the
      // real CSS scale, not an incorrect hard-coded 16px minimum for .app.
      const expectedRootSize = item.size === 'larger' ? 20 : 16;
      const expectedAppSize = expectedRootSize * (item.width < 600 ? 0.9375 : 1);
      assert.ok(Math.abs(layout.htmlFontSize - expectedRootSize) < 0.1,
        `${name}: root text-size preference not applied: ${JSON.stringify(layout)}`);
      assert.ok(Math.abs(layout.fontSize - expectedAppSize) < 0.1,
        `${name}: unexpected responsive text size: ${JSON.stringify(layout)}`);
      // Two panes only when the dialog is 38rem wide inside (UX #587), so
      // 600px and Larger text keep the phone's list-then-section flow.
      const split = layout.dialog.inner / layout.htmlFontSize >= 38;
      assert.equal(layout.listVisible, split, `${name}: section index visibility`);
      assert.equal(layout.back.includes('settings-back'), !split, `${name}: responsive Back target`);

      if ((item.size === 'larger' && item.appearance === 'calm') ||
          (item.size === 'normal' && item.appearance === 'liquid-glass' &&
            item.width === 384 && ['paper', 'night', 'black'].includes(item.theme))) {
        await page.screenshot({ path: path.join(output, name + '.png') });
      }
      await page.locator('#opener').focus();
      await page.addScriptTag({ content: controller });
      assert.equal(await page.locator('#background').evaluate(el => el.inert), true, name);
      await page.evaluate(() => {
        const dialog = document.querySelector('.settings-window');
        const targets = [...dialog.querySelectorAll('button, a[href], input, select, textarea, [tabindex]')]
          .filter(el => !el.disabled && el.tabIndex >= 0 && el.getClientRects().length && !el.closest('[hidden], [inert]'));
        window.fixtureTargets = targets;
        window.fixtureEscapes = [];
        for (const button of dialog.querySelectorAll('[data-back="true"]')) {
          button.addEventListener('click', () => window.fixtureEscapes.push(button.className));
        }
        targets.at(-1).focus();
      });
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement === window.fixtureTargets[0]), true, `${name}: Tab wrap`);
      await page.keyboard.press('Shift+Tab');
      assert.equal(await page.evaluate(() => document.activeElement === window.fixtureTargets.at(-1)), true, `${name}: Shift+Tab wrap`);
      await page.keyboard.press('Escape');
      assert.deepEqual(await page.evaluate(() => window.fixtureEscapes), [layout.back], `${name}: Escape chose visible Back`);
      // Simulate unmount only: this is not an application-close callback test.
      await page.locator('.settings-window').evaluate(el => el.remove());
      await page.waitForFunction(() => !document.querySelector('#background').inert);
      assert.equal(await page.evaluate(() => document.activeElement.id), 'opener', `${name}: restored focus`);
      results.push({ ...item, layout, passed: true });
    }
    assert.deepEqual(errors, [], 'browser page errors');
    await fs.writeFile(path.join(output, 'report.json'), JSON.stringify({ cases: results.length, results }, null, 2));
    console.log(`Passed ${results.length} production Settings layout and keyboard cases.`);
  } catch (error) {
    await page.screenshot({ path: path.join(output, 'failure.png') }).catch(() => {});
    throw error;
  } finally {
    await browser.close();
  }
}

run().catch(error => { console.error(error); process.exitCode = 1; });
