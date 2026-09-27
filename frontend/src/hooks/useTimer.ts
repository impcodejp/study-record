/**
 * 回答時間を計測するフック（F-03）。
 *
 * 経過時間は時刻の差分で計算するため、タブが裏に回ってタイマーが間引かれても正確。
 * 一時停止中の時間は数えない。
 */

import { useCallback, useEffect, useRef, useState } from 'react'

/** 計測の状態と操作。 */
export interface Timer {
  /** 経過秒数（整数、切り捨て）。 */
  seconds: number
  /** 一時停止中なら true。 */
  paused: boolean
  /** 0 秒から計測し直す。 */
  reset: () => void
  /** 一時停止・再開を切り替える。 */
  togglePause: () => void
  /** 現在の経過秒数を返す（記録時に使う）。 */
  read: () => number
}

/** 回答時間の計測を始める。 */
export function useTimer(): Timer {
  /** 画面を開いた時刻（初回の描画時に 1 回だけ取得する）。 */
  const [openedAt] = useState(() => Date.now())
  /** 計測中の区間の開始時刻。一時停止中は null。 */
  const startedAt = useRef<number | null>(openedAt)
  /** 一時停止までに積み上げたミリ秒。 */
  const accumulated = useRef(0)
  const [seconds, setSeconds] = useState(0)
  const [paused, setPaused] = useState(false)

  const read = useCallback(() => {
    const running = startedAt.current === null ? 0 : Date.now() - startedAt.current
    return Math.floor((accumulated.current + running) / 1000)
  }, [])

  useEffect(() => {
    const id = window.setInterval(() => setSeconds(read()), 250)
    return () => window.clearInterval(id)
  }, [read])

  const reset = useCallback(() => {
    accumulated.current = 0
    startedAt.current = Date.now()
    setPaused(false)
    setSeconds(0)
  }, [])

  const togglePause = useCallback(() => {
    if (startedAt.current === null) {
      startedAt.current = Date.now()
      setPaused(false)
    } else {
      accumulated.current += Date.now() - startedAt.current
      startedAt.current = null
      setPaused(true)
    }
    setSeconds(read())
  }, [read])

  return { seconds, paused, reset, togglePause, read }
}
