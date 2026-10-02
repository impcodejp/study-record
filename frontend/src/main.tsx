/**
 * フロントエンドの起動処理。index.html の #root に React アプリを描画し、本番ではサービスワーカーを登録する。
 */

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import App from './App.tsx'
import './index.css'

const root = document.getElementById('root')
if (!root) throw new Error('#root 要素が見つかりません')

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

// ホーム画面に追加して使えるよう、本番だけサービスワーカーを登録する
// （開発中は画面の自動更新の妨げになるため登録しない）。失敗しても画面は通常どおり動く。
if (import.meta.env.PROD && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch((err: unknown) => {
      console.warn('サービスワーカーを登録できませんでした', err)
    })
  })
}
