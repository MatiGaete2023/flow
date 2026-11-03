const { chromium } = require('playwright');
const { execSync } = require('child_process');
const fs = require('fs');
const EXT = '/home/user/flow/descargador_lite/extension';
const OUT = process.argv[2];
const PIN = process.argv[3];  // "x,y" del alfiler en el menú de extensiones
const sh = c => execSync(c).toString();
const shot = n => sh(`import -window root ${OUT}/raw/${n}`);
const wait = ms => new Promise(r => setTimeout(r, ms));
const args = ['--window-position=0,0', '--window-size=1280,760', '--hide-crash-restore-bubble', '--no-first-run', '--no-default-browser-check'];
const boxes = {};
const box = async (page, sel, key) => { const b = await page.locator(sel).first().boundingBox(); boxes[key] = {x: b.x, y: b.y + 87, width: b.width, height: b.height}; };
(async () => {
  for (const p of ['p1','p2']) fs.rmSync(OUT + '/' + p, {recursive: true, force: true});
  let ctx = await chromium.launchPersistentContext(OUT + '/p1', {headless: false, channel: 'chromium', viewport: null, args});
  let page = ctx.pages()[0];
  await page.goto('chrome://extensions/'); await wait(1500); await page.mouse.click(700, 600);
  await box(page, '#devMode', 'dev'); shot('r01_extensiones.png');
  await page.locator('#devMode').click(); await wait(900);
  await box(page, '#loadUnpacked', 'load'); shot('r02_modo_desarrollador.png');
  await ctx.close();
  ctx = await chromium.launchPersistentContext(OUT + '/p2', {headless: false, channel: 'chromium', viewport: null,
    args: [...args, `--disable-extensions-except=${EXT}`, `--load-extension=${EXT}`]});
  page = ctx.pages()[0];
  await page.goto('chrome://extensions/'); await wait(1500);
  if ((await page.locator('#devMode').getAttribute('aria-pressed')) !== 'true' && !(await page.locator('#loadUnpacked').isVisible())) { await page.locator('#devMode').click(); await wait(800); }
  await page.mouse.click(700, 650);
  await box(page, 'extensions-item', 'card'); shot('r03_extension_cargada.png');
  sh('xdotool mousemove 1166 62 click 1'); await wait(1200); shot('r04_menu_extensiones.png');
  if (PIN) {
    const [x, y] = PIN.split(',');
    sh(`xdotool mousemove ${x} ${y} click 1`); await wait(800);
    sh('xdotool key Escape'); await wait(600); shot('r05_fijada.png');
  }
  const sw = ctx.serviceWorkers()[0] || await ctx.waitForEvent('serviceworker');
  const id = sw.url().split('/')[2];
  await page.goto(`chrome-extension://${id}/conexion.html?tab=1`); await wait(900);
  shot('r06_conexion.png');
  await page.fill('#code', '51234-0123456789abcdef0123456789abcdef'); await wait(300);
  await box(page, '#code', 'code'); await box(page, '#connect', 'connect');
  shot('r07_codigo_pegado.png');
  await ctx.close();
  fs.writeFileSync(OUT + '/boxes.json', JSON.stringify(boxes, null, 1));
})().catch(e => { console.error(e.message); process.exit(1); });
