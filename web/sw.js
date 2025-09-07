const MANIFEST_VERSION = "__PRECACHE_VERSION__";
const PRECACHE = `precache-${MANIFEST_VERSION}`;
const RUNTIME = "runtime";

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(PRECACHE).then(async (cache) => {
      const res = await fetch("/assets/precache.json");
      const files = await res.json();
      return cache.addAll(files);
    }),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys.map((key) => {
          if (key !== PRECACHE) {
            return caches.delete(key);
          }
        }),
      ),
    ),
  );
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") {
    return;
  }
  if (event.request.url.startsWith(`${self.location.origin}/assets/`)) {
    event.respondWith(
      caches.match(event.request).then((res) => res || fetch(event.request)),
    );
    return;
  }

  event.respondWith(
    caches.match(event.request).then((cached) => {
      const fetchPromise = fetch(event.request).then((networkResponse) =>
        caches.open(RUNTIME).then((cache) => {
          cache.put(event.request, networkResponse.clone());
          return networkResponse;
        }),
      );
      event.waitUntil(fetchPromise);
      return cached || fetchPromise;
    }),
  );
});
