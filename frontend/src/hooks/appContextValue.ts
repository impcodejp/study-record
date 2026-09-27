/**
 * アプリ全体で共有する状態の型と React コンテキスト。
 *
 * 提供側は AppProvider.tsx、利用側は useApp.ts を使う。
 */

import { createContext } from 'react'

import type { User } from '../api/types'

/** 共有する状態と操作。 */
export interface AppContextValue {
  /** ログイン中のユーザー。未ログインなら null。 */
  user: User | null
  /** ログイン状態を確認中なら true。 */
  loading: boolean
  /** 進行中の学習の ID。無ければ null。 */
  activePracticeId: number | null
  /** 進行中の学習の ID を更新する（開始・完了・中断時）。 */
  setActivePracticeId: (id: number | null) => void
  /** ログインする。 */
  login: (email: string, password: string) => Promise<void>
  /** ログアウトする。 */
  logout: () => Promise<void>
}

export const AppContext = createContext<AppContextValue | null>(null)
