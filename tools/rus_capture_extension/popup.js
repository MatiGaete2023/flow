const button = document.getElementById("capture"), status = document.getElementById("status");
button.addEventListener("click", async () => {
  button.disabled = true;
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab?.id || new URL(tab.url).origin !== "https://familia.pjud.cl")
      throw new Error("Abre RUS en familia.pjud.cl y vuelve a pulsar la extensión.");
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id, allFrames: true }, func: captureRusDocument
    });
    const documents = results.filter(x => x.result && !x.result.skipped)
      .map(x => ({ frame_id: x.frameId, document: x.result }));
    if (!documents.length) throw new Error("No se pudo leer el documento. Abre directamente la ventana de la bitácora.");
    const encoded = new TextEncoder().encode(JSON.stringify(documents));
    const digest = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", encoded)))
      .map(x => x.toString(16).padStart(2, "0")).join("");
    const bundle = { schema: "rus-capture-v1", stage: document.getElementById("stage").value,
      captured_at: new Date().toISOString(), documents_sha256: digest, documents,
      inaccessible_frames_possible: true };
    const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob), link = document.createElement("a");
    link.href = url; link.download = `RUS_${bundle.stage}_${bundle.captured_at.replace(/[:.]/g, "-")}.json`;
    link.click(); setTimeout(() => URL.revokeObjectURL(url), 30000);
    status.textContent = `Captura de ${documents.length} documento(s) descargada. No se ha registrado ninguna observación.`;
  } catch (error) { status.textContent = error.message || "No se pudo capturar esta pantalla."; }
  finally { button.disabled = false; }
});
