/**
 * ビルド後に、紹介ページを検索エンジン向けの HTML として書き出す（npm run build の最後に実行される）。
 *
 * 作るもの（すべて dist/ の中）：
 *   welcome/index.html  紹介ページ（本文・タイトル・説明・正規の URL・SNS 向けの情報・構造化データ入り）
 *   robots.txt          検索エンジンへの案内（API は巡回させない）
 *   sitemap.xml         検索エンジンに知らせるページの一覧
 *
 * 公開する URL は環境変数 SITE_URL で指定する。省略すると https://study.example.com になり、
 * deploy/nginx/install-nginx.ps1 -ServerName が配置するときに実際のドメインへ書き換える。
 *
 * nginx は、ログインしていない人（Cookie が無い人）がトップ（/）を開いたときに welcome/index.html を返す。
 */

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { createServer } from 'vite'

const frontendDir = join(dirname(fileURLToPath(import.meta.url)), '..')
const distDir = join(frontendDir, 'dist')
const siteUrl = (process.env.SITE_URL ?? 'https://study.example.com').replace(/\/+$/, '')

/**
 * 試験のテンプレートの名前を、バックエンドの定義（backend/src/domain/exam_templates.rs）から読む。
 * ビルド時は API サーバーが動いていないため、同じ定義から取り出して画面とずれないようにする。
 */
function readExamNames() {
  const source = readFileSync(join(frontendDir, '..', 'backend', 'src', 'domain', 'exam_templates.rs'), 'utf8')
  const names = [...source.matchAll(/ExamTemplate \{\s*key: "[^"]+",\s*name: "([^"]+)"/g)].map((m) => m[1])
  if (names.length === 0) throw new Error('exam_templates.rs から試験名を読み取れませんでした')
  return names
}

/** 書き出した HTML の要素を差し替える。見つからなければ止める（気づかないまま壊れないように）。 */
function replaceOnce(html, pattern, replacement, label) {
  if (!pattern.test(html)) throw new Error(`index.html に ${label} が見つかりません`)
  return html.replace(pattern, () => replacement)
}

const examNames = readExamNames()

// React の部品を Node で描くため、Vite の SSR の仕組みで src/prerender.tsx を読み込む。
const vite = await createServer({
  root: frontendDir,
  logLevel: 'error',
  server: { middlewareMode: true },
  appType: 'custom',
})
try {
  const { renderLanding, landingHead } = await vite.ssrLoadModule('/src/prerender.tsx')
  const template = readFileSync(join(distDir, 'index.html'), 'utf8')
  let html = replaceOnce(
    template,
    /<!-- seo:start -->[\s\S]*?<!-- seo:end -->/,
    landingHead(siteUrl, examNames),
    'seo:start 〜 seo:end',
  )
  html = replaceOnce(html, /<div id="root"><\/div>/, `<div id="root">${renderLanding(examNames)}</div>`, '#root')
  mkdirSync(join(distDir, 'welcome'), { recursive: true })
  writeFileSync(join(distDir, 'welcome', 'index.html'), html)
} finally {
  await vite.close()
}

writeFileSync(
  join(distDir, 'robots.txt'),
  ['User-agent: *', 'Allow: /', 'Disallow: /api/', '', `Sitemap: ${siteUrl}/sitemap.xml`, ''].join('\n'),
)
writeFileSync(
  join(distDir, 'sitemap.xml'),
  [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    `  <url><loc>${siteUrl}/</loc><changefreq>weekly</changefreq><priority>1.0</priority></url>`,
    '</urlset>',
    '',
  ].join('\n'),
)
console.log(`紹介ページを書き出しました（${siteUrl}、試験 ${examNames.length} 件）`)
