/**
 * ブラウザのタブ（と検索結果）に出るページのタイトルを設定するフック。
 */

import { useEffect } from 'react'

import { SITE_NAME } from '../content/landing'

/**
 * ページのタイトルを「タイトル | 学習記録」にする。`raw` を指定すると、そのまま使う。
 * 画面を離れても元に戻さない（次の画面が自分のタイトルを設定するため）。
 */
export function useDocumentTitle(title: string, { raw = false }: { raw?: boolean } = {}) {
  useEffect(() => {
    document.title = raw ? title : `${title} | ${SITE_NAME}`
  }, [title, raw])
}
