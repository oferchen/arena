async function init() {
  const wasm = await import("./pkg/client.js");
  if (wasm && wasm.default) {
    await wasm.default();
  }
}

init();
