// Service Worker for morphio
"use strict";

const VERSION = "0.1.4";
const CACHE_NAME = `morphio-${VERSION}`;
const APP_RESOURCE = [
    "/",
    "/index.js",
    "/manifest.json",
    "/favicon.svg",
    "/style.css",
    "/wasm/morphio.js",
    "/wasm/morphio_bg.wasm",
];

// Install event - cache files
self.addEventListener("install", (event) => {
    // Take control immediately
    self.skipWaiting();

    event.waitUntil(
        caches
            .open(CACHE_NAME)
            .then((cache) => {
                console.log("Opened cache");
                return cache.addAll(APP_RESOURCE);
            })
            .catch((error) => {
                console.error("Cache installation failed:", error);
            }),
    );
});

// Helper to fetch and cache
async function fetchAndCache(request, cacheName) {
    try {
        const response = await fetch(request);
        if (
            response &&
            (response.status === 200 ||
                response.status === 0 ||
                response.type === "basic")
        ) {
            const cache = await caches.open(cacheName);
            cache.put(request, response.clone());
        }
        return response;
    } catch (error) {
        const cached = await caches.match(request);
        return cached || new Response("You are offline", { status: 503 });
    }
}

// Helper to determine if a request is for an app resource
function isAppResource(requestUrl) {
    return APP_RESOURCE.some((urlStr) => {
        const url = new URL(urlStr, self.location.origin);
        return (
            url.origin === requestUrl.origin &&
            url.pathname === requestUrl.pathname
        );
    });
}

// Fetch event - serve from cache, fallback to network
self.addEventListener("fetch", (event) => {
    const requestUrl = new URL(event.request.url);

    if (requestUrl.pathname === "/index.html") {
        requestUrl.pathname = "/";
    }
    if (!isAppResource(requestUrl)) {
        // Do nothing
        return;
    }

    // Cache first strategy for both app resources and icons
    event.respondWith(
        caches
            .match(isAppResource(requestUrl) ? requestUrl : event.request, {
                ignoreSearch: true,
            })
            .then(
                (response) =>
                    response || fetchAndCache(event.request, CACHE_NAME),
            ),
    );
});

// Activate event - clean up old caches
self.addEventListener("activate", (event) => {
    // Take control of all clients immediately
    event.waitUntil(clients.claim());

    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames
                    .filter((cacheName) => cacheName !== CACHE_NAME)
                    .map((cacheName) => {
                        console.log("Deleting old cache:", cacheName);
                        return caches.delete(cacheName);
                    }),
            );
        }),
    );
});
