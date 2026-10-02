/**
 * サービスワーカー（ホーム画面に追加して使うための最小限のキャッシュ）。
 *
 * - 画面の HTML：ネットワークを優先し、つながらないときだけ保存しておいた index.html を返す
 * - ビルド済みの JS / CSS（/assets/、ファイル名にハッシュ付き）とアイコン：保存したものを優先する
 * - API（/api/）：個人の学習データを端末に残さないため、キャッシュしない
 *
 * 新しい版を配置したら CACHE_VERSION を上げる（古いキャッシュは有効化時に削除する）。
 */

const CACHE_VERSION = 'study-record-v1'
/** 最初に保存しておくファイル。 */
const PRECACHE = ['/index.html', '/favicon.svg', '/icons/icon-192.png', '/icons/icon-512.png']

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE_VERSION)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  )
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE_VERSION).map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener('fetch', (event) => {
  const request = event.request
  if (request.method !== 'GET') return
  const url = new URL(request.url)
  // 別のサイトと API は扱わない（ブラウザの通常の通信に任せる）。
  if (url.origin !== self.location.origin || url.pathname.startsWith('/api/')) return

  if (request.mode === 'navigate') {
    event.respondWith(fetch(request).catch(() => caches.match('/index.html')))
    return
  }

  if (url.pathname.startsWith('/assets/') || url.pathname.startsWith('/icons/')) {
    event.respondWith(
      caches.match(request).then(
        (cached) =>
          cached ??
          fetch(request).then((response) => {
            if (response.ok) {
              const copy = response.clone()
              caches.open(CACHE_VERSION).then((cache) => cache.put(request, copy))
            }
            return response
          }),
      ),
    )
  }
})
