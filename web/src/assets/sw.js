// sw.js

// Version identifier for cache and update management
const cacheVersion = 'v2';
// Function to fetch the asset manifest
async function fetchManifest() {
  const response = await fetch('/web/src/assets/workers/manifest.json');
  return response.json();
}
self.addEventListener('install', function(event) {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(cacheVersion);
      const manifest = await fetchManifest();

      // List of files to cache
      const filesToCache = [
      ];

      // Add hashed filenames from the manifest
      Object.keys(manifest).forEach(key => {
        if(key == "index.html") {
          filesToCache.push(`web/${manifest[key]["file"]}`);
        }
      });

      console.log(filesToCache);
      // await cache.addAll(filesToCache);
      // Cache files with error handling
      await Promise.all(
        filesToCache.map(async (file) => {
          try {
            const response = await fetch(file);
            if (!response.ok) {
              throw new Error(`Request for ${file} failed with status ${response.status}`);
            }
            await cache.put(file, response);
          } catch (error) {
            console.error(`Failed to cache ${file}:`, error);
          }
        })
      );
    })()
  );
});

self.addEventListener('activate', function(event) {
  // Clean up old caches if any
  event.waitUntil(
    caches.keys().then(function(cacheNames) {
      return Promise.all(
        cacheNames.filter(function(cacheName) {
          return cacheName !== cacheVersion;
        }).map(function(cacheName) {
          return caches.delete(cacheName);
        })
      );
    })
  );
});

self.addEventListener('fetch', function(event) {
  // Intercept fetch requests and serve from cache if available
  event.respondWith(
    caches.match(event.request).then(function(response) {
      return response || fetch(event.request);
    })
  );
});

self.addEventListener('message', function(event) {
  if (event.data === 'skipWaiting') {
    self.skipWaiting(); // Activate the new service worker immediately
  }
});