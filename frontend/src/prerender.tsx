/**
 * 紹介ページを HTML として書き出すための入口（ビルド時に scripts/prerender.mjs から使う）。
 *
 * 画面は JavaScript で描くため、そのままでは検索エンジンや SNS が最初に受け取る HTML が空になる。
 * 紹介ページだけは、ビルド時に中身を HTML にしておく（画面を開くと、いつもどおり React が描き直す）。
 */

import { renderToString } from 'react-dom/server'
import { MemoryRouter } from 'react-router-dom'

import { faqItems, FEATURES, LANDING_DESCRIPTION, LANDING_TITLE, SITE_NAME } from './content/landing'
import { LandingPage } from './pages/LandingPage'

/** 紹介ページの本文（#root の中身）を HTML にする。 */
export function renderLanding(examNames: string[]): string {
  return renderToString(
    <MemoryRouter initialEntries={['/']}>
      <LandingPage initialExamNames={examNames} />
    </MemoryRouter>,
  )
}

/** HTML の属性値・本文に入れる文字をエスケープする。 */
function escapeHtml(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

/** JSON-LD を script 要素に安全に埋め込む（`</script>` で閉じられないようにする）。 */
function jsonLd(data: unknown): string {
  return `<script type="application/ld+json">${JSON.stringify(data).replace(/</g, '\\u003c')}</script>`
}

/**
 * 紹介ページの head に入れるタグ（タイトル・説明・正規の URL・SNS 向けの情報・構造化データ）。
 *
 * `siteUrl` は公開する URL（例：https://study.example.com）。末尾の / は付けない。
 */
export function landingHead(siteUrl: string, examNames: string[]): string {
  const url = `${siteUrl}/`
  const image = `${siteUrl}/og-image.png`
  const app = {
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    name: SITE_NAME,
    url,
    description: LANDING_DESCRIPTION,
    applicationCategory: 'EducationalApplication',
    operatingSystem: 'Web',
    inLanguage: 'ja',
    featureList: FEATURES.map((f) => f.title),
  }
  const faq = {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: faqItems(examNames).map((item) => ({
      '@type': 'Question',
      name: item.q,
      acceptedAnswer: { '@type': 'Answer', text: item.a },
    })),
  }
  return [
    `<title>${escapeHtml(LANDING_TITLE)}</title>`,
    `<meta name="description" content="${escapeHtml(LANDING_DESCRIPTION)}" />`,
    '<meta name="robots" content="index, follow" />',
    `<link rel="canonical" href="${escapeHtml(url)}" />`,
    `<meta property="og:site_name" content="${escapeHtml(SITE_NAME)}" />`,
    `<meta property="og:title" content="${escapeHtml(LANDING_TITLE)}" />`,
    `<meta property="og:description" content="${escapeHtml(LANDING_DESCRIPTION)}" />`,
    '<meta property="og:type" content="website" />',
    '<meta property="og:locale" content="ja_JP" />',
    `<meta property="og:url" content="${escapeHtml(url)}" />`,
    `<meta property="og:image" content="${escapeHtml(image)}" />`,
    '<meta property="og:image:width" content="1200" />',
    '<meta property="og:image:height" content="630" />',
    '<meta name="twitter:card" content="summary_large_image" />',
    jsonLd(app),
    jsonLd(faq),
  ].join('\n    ')
}
