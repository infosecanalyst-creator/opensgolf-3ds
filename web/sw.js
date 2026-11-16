// The browser version's service worker (the port's own): what lets OpenSGolf, once played, start again without a
// connection, and be added to a phone's home screen as an app (web/manifest.webmanifest).
//
// * The page itself (a visit to the address, index.html) comes from the network when there is one, so a new version is
//   picked up at the next start as before, and from the copy kept here when there is none (or it is too slow to answer).
// * Every other file of this site (gl.js, files.js, opensgolf.js, the game's .wasm, the pictures, the icons) is named with
//   its version in its address (web/build.sh), so the copy kept here is always right: it is used first, and a file not kept
//   yet is fetched and kept. The game's .wasm (about 30 MB) is kept the first time the game starts, not on a first visit.
// * Other sites (the headline font, GitHub's news and downloads, the visit counter) are left to the browser.
// * Each version keeps its files under its own name, and a new version clears the older ones once it takes over.
//
// The player's game files, saves, packs and HD art never pass through here: they live in the browser's own storage
// (IndexedDB, local storage), which this worker does not touch.
const SG_TAG = "%SG_TAG%";
const SG_CACHE = "opensgolf-" + SG_TAG;
// what a first visit keeps at once: the start page and what it needs to show and to start the game
const SG_SHELL = ["./", "manifest.webmanifest", "icons/icon-192.png", "icons/icon-512.png", "icons/apple-touch-icon.png",
    "whatsnew.json", "%SG_SHELL%"];
// how long the start page waits for the network before it is taken from the kept copy (a weak signal)
const SG_PAGE_WAIT_MS = 4000;

self.addEventListener("install", event => {
    event.waitUntil((async () => {
        const cache = await caches.open(SG_CACHE);
        // one file that fails (a blocked request, a slow network) does not stop the others from being kept
        await Promise.all(SG_SHELL.filter(u => u && !u.startsWith("%")).map(u =>
            fetch(new Request(u, { cache: "reload" })).then(r => { if (r.ok) return cache.put(u, r); }).catch(() => {})));
        await self.skipWaiting();
    })());
});

self.addEventListener("activate", event => {
    event.waitUntil((async () => {
        for (const name of await caches.keys()) {
            if (name.startsWith("opensgolf-") && name !== SG_CACHE) await caches.delete(name);
        }
        await self.clients.claim();
    })());
});

function sgIsPage(request, url) {
    return request.mode === "navigate" || url.pathname.endsWith("/") || url.pathname.endsWith("/index.html");
}

async function sgPage(request) {
    const cache = await caches.open(SG_CACHE);
    const fromNet = fetch(request).then(r => {
        if (r.ok) cache.put("./", r.clone());
        return r;
    });
    const timeout = new Promise(res => setTimeout(() => res(null), SG_PAGE_WAIT_MS));
    try {
        const r = await Promise.race([fromNet, timeout]);
        if (r) return r;
    } catch (e) {}
    const kept = await cache.match("./", { ignoreSearch: true });
    if (kept) return kept;
    // nothing kept yet: wait for the network after all
    return fromNet;
}

async function sgFile(request) {
    const cache = await caches.open(SG_CACHE);
    const kept = await cache.match(request);
    if (kept) return kept;
    const r = await fetch(request);
    if (r.ok && r.type === "basic") cache.put(request, r.clone()).catch(() => {});
    return r;
}

self.addEventListener("fetch", event => {
    const request = event.request;
    if (request.method !== "GET") return;
    const url = new URL(request.url);
    if (url.origin !== self.location.origin) return;
    // the news list changes between versions: the network first, the kept copy offline
    if (url.pathname.endsWith("/whatsnew.json")) {
        event.respondWith(fetch(request).catch(() => caches.match(request, { ignoreSearch: true })));
        return;
    }
    event.respondWith(sgIsPage(request, url) ? sgPage(request) : sgFile(request));
});
