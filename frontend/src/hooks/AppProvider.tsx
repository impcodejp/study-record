/**
 * アプリ全体で共有する状態（ログイン中のユーザー、進行中の学習）を提供するコンポーネント。
 */

import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'

import { setCsrfToken, setUnauthorizedHandler } from '../api/client'
import { authApi, practiceApi } from '../api/endpoints'
import type { User } from '../api/types'
import { AppContext } from './appContextValue'

/** アプリ全体の状態を提供するコンポーネント。 */
export function AppProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [activePracticeId, setActivePracticeId] = useState<number | null>(null)

  /** ログイン後に進行中の学習を確認する（F-04 再開）。 */
  const loadActive = useCallback(async () => {
    const active = await practiceApi.active()
    setActivePracticeId(active?.id ?? null)
  }, [])

  // 初期表示時にログイン状態を確認する。
  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const session = await authApi.me()
        if (cancelled) return
        setCsrfToken(session.csrfToken)
        setUser(session.user)
        await loadActive()
      } catch {
        if (!cancelled) setUser(null)
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [loadActive])

  // セッション切れ（401）を受け取ったらログイン画面に戻す。
  useEffect(() => {
    setUnauthorizedHandler(() => {
      setCsrfToken(null)
      setUser(null)
      setActivePracticeId(null)
    })
    return () => setUnauthorizedHandler(null)
  }, [])

  const login = useCallback(
    async (email: string, password: string) => {
      const session = await authApi.login(email, password)
      setCsrfToken(session.csrfToken)
      setUser(session.user)
      await loadActive()
    },
    [loadActive],
  )

  const logout = useCallback(async () => {
    try {
      await authApi.logout()
    } finally {
      setCsrfToken(null)
      setUser(null)
      setActivePracticeId(null)
    }
  }, [])

  const value = useMemo(
    () => ({ user, loading, activePracticeId, setActivePracticeId, login, logout }),
    [user, loading, activePracticeId, login, logout],
  )
  return <AppContext.Provider value={value}>{children}</AppContext.Provider>
}
