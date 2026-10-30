// Imprime los HTML ilustrados a PDF A4 con Chromium. Uso: node pdf.js <carpeta_html> <carpeta_pdf>
const { chromium } = require('playwright');
const path = require('path');
const [src, out] = process.argv.slice(2);
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  for (const name of ['SITFA_Lite_Manual_Instalacion', 'SITFA_Lite_Manual_Uso']) {
    await page.goto('file://' + path.resolve(src, name + '.html'));
    await page.waitForLoadState('networkidle');
    await page.pdf({path: path.resolve(out, name + '.pdf'), format: 'A4', printBackground: true,
      displayHeaderFooter: true, headerTemplate: '<span></span>',
      footerTemplate: '<div style="font-size:8px;width:100%;text-align:center;color:#64748b">SITFA Lite 1.0.0 · página <span class="pageNumber"></span> de <span class="totalPages"></span></div>',
      margin: {top: '14mm', bottom: '16mm', left: '0', right: '0'}});
  }
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });
