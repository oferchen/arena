const test = require("node:test");
const assert = require("node:assert");
const makeServiceWorkerEnv = require("service-worker-mock");

test("level can be stored and retrieved", async () => {
  Object.assign(global, makeServiceWorkerEnv());
  const { storeLevel, loadLevel } = require("../client/level-storage.js");
  const level = { id: "1", name: "Test" };
  await storeLevel(level.id, JSON.stringify(level));
  const stored = await loadLevel(level.id);
  assert.deepStrictEqual(JSON.parse(stored), level);
});
