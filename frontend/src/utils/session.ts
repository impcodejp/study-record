/**
 * 学習開始画面から回答画面へ「最初の問題数」を渡すための一時保存。
 *
 * 最初の問題数はサーバーに送らない（F-01）ため、同じタブの sessionStorage に置く。
 * 学習 ID と組にして保存し、別の学習の値は使わない（次の学習開始時に上書きされる）。
 * 保存できない環境では、回答画面で問題数が空欄になるだけで動作は止まらない。
 */

const KEY = 'study-record:first-number'

/** 学習 ID と最初の問題数を保存する。 */
export function saveSessionStart(practiceId: number, firstNumber: number): void {
  try {
    window.sessionStorage.setItem(KEY, JSON.stringify({ practiceId, firstNumber }))
  } catch {
    // 保存できなくても動作に影響しないため無視する。
  }
}

/** 学習 ID に対応する最初の問題数を返す。無ければ null。 */
export function readSessionStart(practiceId: number): number | null {
  try {
    const raw = window.sessionStorage.getItem(KEY)
    if (!raw) return null
    const value = JSON.parse(raw) as { practiceId?: number; firstNumber?: number }
    return value.practiceId === practiceId && typeof value.firstNumber === 'number' ? value.firstNumber : null
  } catch {
    return null
  }
}
