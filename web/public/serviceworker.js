/* Load the custom "manifest" which lists the files that make up the app
 * and a unique identifier for the cache, which we use to pre-cache the
 * entire app on worker install (so it works offline immediately). */
importScripts('worker-cache-manifest.js');

if (typeof cachePrefix !== 'undefined'
    && typeof cacheName !== 'undefined'
    && typeof cacheFiles !== 'undefined') {
    /* Cache things from worker-cache-manifest.js on install */
    self.addEventListener('install', e => {
        e.waitUntil(async () => {
            const cache = await caches.open(cacheName);
            console.log('Caching manifest resources', cacheFiles);
            await cache.addAll(cacheFiles);
        });
    });

    /* Serve data from cache */
    self.addEventListener('fetch', e => {
        async function handleRequest() {
            const r = await caches.match(e.request);
            if (r) {
                return r;
            }

            const response = await fetch(e.request);
            const cache = await caches.open(cacheName);
            cache.put(e.request, response.clone());
            return response;
        }

        e.respondWith(handleRequest());
    });

    /* Clean up stale caches on update */
    self.addEventListener('activate', e => {
        async function pruneObsoleteCaches() {
            let pruneCaches = [];
            for (const key of await caches.keys()) {
                // Ignore the current version and caches with different prefixes
                // (which are probably different apps on the same origin).
                if (!key.startsWith(cachePrefix) || key === cacheName) {
                    continue;
                }
                console.log('Cleaning obsolete cache ' + key);
                pruneCaches.push(caches.delete(key));
            }
            await Promise.all(pruneCaches);
        }

        e.waitUntil(pruneObsoleteCaches());
    });
} else {
    console.log('Worker cache manifest not configured; not caching data');
}

self.addEventListener('message', messageEvent => {
    if (messageEvent.data === 'skipWaiting')
        return self.skipWaiting();
});
