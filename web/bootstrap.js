async function init() {
  const canvasId = window.BEVY_CANVAS_ID || "bevy-canvas";
  const wasm = await import("./pkg/client.js");
  if (wasm && wasm.default) {
    await wasm.default(canvasId);
  }
}

init();
