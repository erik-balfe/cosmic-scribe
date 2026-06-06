import { chromium } from 'playwright';
import path from 'node:path';

const base = process.argv[2];
const outDir = process.argv[3];

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 920, height: 900 } });

await page.goto(`${base}/`);
await page.waitForSelector('.entry');
await page.screenshot({ path: path.join(outDir, 'app-history.png') });

await page.goto(`${base}/settings`);
await page.waitForSelector('form.panel');
await page.screenshot({ path: path.join(outDir, 'app-settings.png') });

await page.goto(`${base}/`);
await page.waitForSelector('.entry');
await page.locator('.entry').first().click();
await page.waitForSelector('text=Back to history');
await page.waitForTimeout(600);
await page.screenshot({ path: path.join(outDir, 'app-detail.png') });

await browser.close();
console.log('wrote app-history.png, app-settings.png, app-detail.png');