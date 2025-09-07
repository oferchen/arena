const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const makeServiceWorkerEnv = require("service-worker-mock");

test("service worker serves cached response and refreshes asynchronously", async () => {
  Object.assign(global, makeServiceWorkerEnv());

  const swSrc = fs
    .readFileSync(path.join(__dirname, "..", "web", "sw.js"), "utf8")
    .replace("__PRECACHE_VERSION__", "test");
  eval(swSrc);

  const url = `${self.location.origin}/foo.txt`;
  const cache = await caches.open("runtime");
  await cache.put(url, new Response("old"));

  global.fetch = () => Promise.resolve(new Response("new"));

  const res = await self.trigger(
    "fetch",
    new FetchEvent("fetch", { request: new Request(url) }),
  );
  assert.strictEqual(await res.text(), "old");

  const updated = await cache.match(url);
  assert.strictEqual(await updated.clone().text(), "new");
});
