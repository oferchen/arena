self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open("precache").then(async (cache) => {
      const res = await fetch("/assets/precache.json");
      const files = await res.json();
      return cache.addAll(files);
    }),
  );
});

self.addEventListener("fetch", (event) => {
  event.respondWith(
    caches
      .match(event.request)
      .then((response) => response || fetch(event.request)),
  );
});
