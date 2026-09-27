/**
 * フロントエンドの起動処理。index.html の #root に React アプリを描画する。
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
