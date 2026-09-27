/**
 * アプリ全体で共有する状態を取り出すフック。
 */

import { useContext } from 'react'

import { AppContext, type AppContextValue } from './appContextValue'

/** アプリ全体の状態を取得する。 */
export function useApp(): AppContextValue {
  const value = useContext(AppContext)
  if (!value) throw new Error('useApp は AppProvider の内側で使ってください')
  return value
}
