chrome.action.onClicked.addListener(async tab => {
  if (!tab.id || !tab.url?.startsWith("https://familia.pjud.cl/")) {
    await chrome.tabs.create({url:"https://familia.pjud.cl/SITFAWEB/jsp/Login/LoginB4.jsp"});
    return;
  }
  await chrome.tabs.create({url:chrome.runtime.getURL("conexion.html")+"?tab="+tab.id});
});
