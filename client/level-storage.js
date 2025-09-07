async function storeLevel(id, data) {
  const cache = await caches.open("levels");
  await cache.put(`/levels/${id}`, new Response(data));
}

async function loadLevel(id) {
  const cache = await caches.open("levels");
  const res = await cache.match(`/levels/${id}`);
  if (!res) return null;
  return await res.text();
}

const globalScope = typeof self !== "undefined" ? self : globalThis;
globalScope.storeLevel = storeLevel;
globalScope.loadLevel = loadLevel;

if (typeof module !== "undefined") {
  module.exports = { storeLevel, loadLevel };
}
