"""P2: ejecuta únicamente el fixture local en Chromium de desarrollo (Playwright)."""
import os
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
fixture = (ROOT / 'tests/fake_rus/form_detection.html').read_text(encoding='utf-8')
script = (ROOT / 'extension/observer.js').read_text(encoding='utf-8')
marker = '<script src="../../extension/observer.js"></script>'
assert fixture.count(marker) == 1
fixture = fixture.replace(marker, '<script>' + script + '</script>')
with sync_playwright() as p:
    options = {'executable_path': os.environ['REGISTRADOR_CHROMIUM']} if os.getenv('REGISTRADOR_CHROMIUM') else {}
    browser = p.chromium.launch(headless=True, timeout=5000, **options)
    try:
        page = browser.new_page()
        page.route('**/*', lambda route: route.abort())
        page.set_content(fixture, timeout=5000)
        result = page.locator('#test-result').inner_text(timeout=5000)
        print(result)
        assert result == 'PASS 14', result
    finally:
        browser.close()
