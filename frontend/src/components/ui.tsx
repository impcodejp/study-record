/**
 * 画面で共通に使う小さな部品。
 */

import type { ReactNode } from 'react'

/** エラーや完了のお知らせ。 */
export function Notice({ kind = 'error', children }: { kind?: 'error' | 'info' | 'success'; children: ReactNode }) {
  if (!children) return null
  return (
    <div className={`notice notice-${kind}`} role={kind === 'error' ? 'alert' : 'status'}>
      {children}
    </div>
  )
}

/** 読み込み中の表示。 */
export function Loading() {
  return (
    <p className="muted" aria-live="polite">
      読み込み中…
    </p>
  )
}

/** 正誤の表示（色だけに頼らず記号と文字も出す）。 */
export function ResultBadge({ correct }: { correct: boolean | null }) {
  if (correct === null) return <span className="badge badge-pending">未採点</span>
  return correct ? (
    <span className="badge badge-good">○ 正解</span>
  ) : (
    <span className="badge badge-bad">× 不正解</span>
  )
}

/** ページの見出し。右側に操作ボタンを置ける。 */
export function PageHeader({ title, sub, actions }: { title: string; sub?: ReactNode; actions?: ReactNode }) {
  return (
    <div className="page-header">
      <div>
        <h1>{title}</h1>
        {sub && <p className="muted">{sub}</p>}
      </div>
      {actions && <div className="page-actions">{actions}</div>}
    </div>
  )
}

/** データが無いときの表示。 */
export function Empty({ children }: { children: ReactNode }) {
  return <p className="empty">{children}</p>
}

/** 改行を保ったまま回答などの文章を表示する。 */
export function MultilineText({ text, className = '' }: { text: string | null | undefined; className?: string }) {
  if (!text) return <span className="muted">―</span>
  return <span className={`multiline ${className}`}>{text}</span>
}
