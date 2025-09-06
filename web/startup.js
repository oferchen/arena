const canvasId = "bevy-canvas";
window.BEVY_CANVAS_ID = canvasId;

const canvas = document.getElementById(canvasId);

function handleInteraction() {
  canvas.requestPointerLock();
  const ctx = window.__bevy_audio_context || window.__bevyAudioContext;
  if (ctx && ctx.state === "suspended") {
    ctx.resume();
  }
}

window.addEventListener("mousedown", handleInteraction, { once: true });
window.addEventListener("keydown", handleInteraction, { once: true });
