/* Exclusivamente abre la página interna de demostración. */
chrome.action.onClicked.addListener(() => chrome.tabs.create({url: chrome.runtime.getURL('demo.html')}));
