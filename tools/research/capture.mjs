// Screenshot harness for firmforge competitor teardowns.
// Usage: node capture.mjs
import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';

const OUT = path.resolve('../../plan/spec/product-research');

const targets = JSON.parse(fs.readFileSync('./targets.json', 'utf8'));

const run = async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    userAgent:
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36',
  });
  const log = [];

  for (const [slug, shots] of Object.entries(targets)) {
    const dir = path.join(OUT, slug, 'screenshots');
    fs.mkdirSync(dir, { recursive: true });
    for (const shot of shots) {
      const file = path.join(dir, `${shot.name}.png`);
      const page = await ctx.newPage();
      try {
        await page.goto(shot.url, { waitUntil: 'domcontentloaded', timeout: 60000 });
        await page.waitForTimeout(shot.wait ?? 3500);
        // dismiss common cookie/consent overlays
        for (const sel of ['button:has-text("Accept")', 'button:has-text("Got it")', 'button:has-text("I agree")']) {
          const b = page.locator(sel).first();
          if (await b.count() && await b.isVisible().catch(() => false)) {
            await b.click({ timeout: 2000 }).catch(() => {});
            await page.waitForTimeout(500);
          }
        }
        await page.screenshot({ path: file, fullPage: shot.fullPage ?? true });
        log.push({ slug, name: shot.name, url: shot.url, ok: true });
        console.log(`OK   ${slug}/${shot.name}`);
      } catch (e) {
        log.push({ slug, name: shot.name, url: shot.url, ok: false, error: String(e).split('\n')[0] });
        console.log(`FAIL ${slug}/${shot.name} :: ${String(e).split('\n')[0]}`);
      } finally {
        await page.close();
      }
    }
  }

  await browser.close();
  fs.writeFileSync(path.join(OUT, 'capture-log.json'), JSON.stringify(log, null, 2));
  const ok = log.filter((l) => l.ok).length;
  console.log(`\n${ok}/${log.length} screenshots captured.`);
};

run();
