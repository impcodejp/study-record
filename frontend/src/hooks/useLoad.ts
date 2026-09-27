/**
 * API からデータを読み込むためのフック。
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import { errorMessage } from '../api/client'

/** 読み込みの状態。 */
export interface LoadState<T> {
  /** 読み込んだデータ。読み込み前は null。 */
  data: T | null
  /** エラーメッセージ。 */
  error: string | null
  /** 読み込み中なら true。 */
  loading: boolean
  /** もう一度読み込む。 */
  reload: () => void
  /** 更新 API の結果などで、データを直接差し替える。 */
  setData: (data: T) => void
}

/**
 * `loader` を呼んでデータを読み込む。`deps` が変わるたびに読み込み直す。
 * 古い要求の結果が後から届いても無視する。
 */
export function useLoad<T>(loader: () => Promise<T>, deps: readonly unknown[]): LoadState<T> {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [version, setVersion] = useState(0)
  const requestId = useRef(0)

  // loader は呼び出し側で毎回作り直されるため、deps と version だけで再実行を判断する。
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const load = useCallback(loader, deps)

  useEffect(() => {
    const id = ++requestId.current
    setLoading(true)
    setError(null)
    load()
      .then((result) => {
        if (id === requestId.current) setData(result)
      })
      .catch((err: unknown) => {
        if (id === requestId.current) setError(errorMessage(err))
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false)
      })
  }, [load, version])

  const reload = useCallback(() => setVersion((v) => v + 1), [])
  return { data, error, loading, reload, setData }
}
