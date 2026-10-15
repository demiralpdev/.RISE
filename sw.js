/* RISE offline cache (sw.js) — performance infrastructure only, zero visual change.
   Strategy:
   - HTML navigations (*.html, "/"): NETWORK-FIRST (fresh markup/routes), cache fallback offline.
   - verify.js WITHOUT a ?v= query: NETWORK-FIRST (verification logic must never
     go stale), cache fallback offline.
   - Same-origin static assets WITH ?v= cache-buster (site.js, styles.css) plus
     stable binaries (mp4/webm/jpg/png/webp/avif/svg/gif/ico/woff2): CACHE-FIRST.
   - Cross-origin (e.g. Google Fonts): untouched pass-through (opaque responses
     would complicate versioning; network behavior identical to today).
   Fail-safe: any error falls through to network; if registration fails the
   site works exactly as today. Cache name is versioned: bump to retire old. */
var RISE_SW_VERSION = "rise-static-v1";
var RISE_PAGE_CACHE = "rise-pages-v1";

function riseIsSameOrigin(url) {
  return url.origin === self.location.origin;
}

function riseHasVersionBust(url) {
  return url.searchParams.has("v");
}

var BINARY_EXT = /\.(mp4|webm|jpg|jpeg|png|webp|avif|gif|svg|ico|woff2?)$/i;

self.addEventListener("install", function (event) {
  // Activate immediately but take control only of subsequent loads
  // (no skipWaiting clients.claim storm: claim on activate below).
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (keys) {
        return Promise.all(
          keys.map(function (k) {
            if (k !== RISE_SW_VERSION && k !== RISE_PAGE_CACHE) {
              return caches.delete(k);
            }
            return Promise.resolve(false);
          })
        );
      })
      .then(function () {
        return self.clients.claim();
      })
  );
});

function netFirst(request, cacheName) {
  return fetch(request).then(
    function (res) {
      try {
        if (res && (res.ok || res.type === "opaque")) {
          var copy = res.clone();
          caches.open(cacheName).then(function (c) {
            try {
              c.put(request, copy);
            } catch (_) {}
          });
        }
      } catch (_) {}
      return res;
    },
    function () {
      return caches.match(request, { ignoreSearch: false }).then(function (hit) {
        if (hit) return hit;
        // Offline + never cached: last-resort cached shell for navigations.
        if (request.mode === "navigate") {
          return caches.match("./index.html", { ignoreSearch: true });
        }
        throw new Error("offline");
      });
    }
  );
}

function cacheFirst(request, cacheName) {
  return caches.match(request, { ignoreSearch: false }).then(function (hit) {
    if (hit) return hit;
    return fetch(request).then(function (res) {
      try {
        if (res && (res.ok || res.type === "opaque")) {
          var copy = res.clone();
          caches.open(cacheName).then(function (c) {
            try {
              c.put(request, copy);
            } catch (_) {}
          });
        }
      } catch (_) {}
      return res;
    });
  });
}

self.addEventListener("fetch", function (event) {
  var req = event.request;
  if (!req || req.method !== "GET") return;
  var url;
  try {
    url = new URL(req.url);
  } catch (_) {
    return;
  }
  if (!riseIsSameOrigin(url)) return; // cross-origin: pass through untouched
  var path = url.pathname;
  var isHTML =
    req.mode === "navigate" ||
    req.destination === "document" ||
    path.endsWith(".html") ||
    path.endsWith("/");
  if (isHTML) {
    event.respondWith(netFirst(req, RISE_PAGE_CACHE));
    return;
  }
  // verify.js ships WITHOUT ?v= today: always revalidate so a fresh
  // verifier is never shadowed by a stale cached copy.
  if (/(^|\/)verify\.js$/.test(path) && !riseHasVersionBust(url)) {
    event.respondWith(netFirst(req, RISE_SW_VERSION));
    return;
  }
  if (
    riseHasVersionBust(url) ||
    BINARY_EXT.test(path) ||
    /(^|\/)(site|styles)\.(js|css)$/.test(path)
  ) {
    event.respondWith(cacheFirst(req, RISE_SW_VERSION));
  }
  // Anything else same-origin: leave to network (identical to today).
});
