/**
 * 画面の表示条件（ログイン必須・未ログイン専用・学習中の固定）。
 */

import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router-dom'

import { useApp } from '../hooks/useApp'
import { Loading } from './ui'

/** ログインが必要な画面。未ログインならログイン画面へ。学習中なら学習画面へ固定する。 */
export function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading, activePracticeId } = useApp()
  const location = useLocation()
  if (loading) return <Loading />
  if (!user) return <Navigate to="/login" replace state={{ from: location.pathname }} />
  // 学習中（回答・採点の途中）は他の画面へ移動できない（F-04）。
  const allowed = location.pathname === '/session' || location.pathname.startsWith('/result/')
  if (activePracticeId !== null && !allowed) return <Navigate to="/session" replace />
  return <>{children}</>
}

/** ログイン前だけ表示する画面（ログイン・新規登録など）。ログイン済みならトップへ。 */
export function GuestOnly({ children }: { children: ReactNode }) {
  const { user, loading } = useApp()
  if (loading) return <Loading />
  if (user) return <Navigate to="/" replace />
  return <>{children}</>
}
