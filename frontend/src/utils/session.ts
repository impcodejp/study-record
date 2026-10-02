/**
 * 学習開始画面から回答画面へ「最初の問題数」（と、解き直すときのカテゴリ）を渡すための一時保存。
 *
 * 最初の問題数はサーバーに送らない（F-01）ため、同じタブの sessionStorage に置く。
 * 学習 ID と組にして保存し、別の学習の値は使わない（次の学習開始時に上書きされる）。
 * 保存できない環境では、回答画面で問題数が空欄になるだけで動作は止まらない。
 */

const KEY = 'study-record:first-number'

/** 学習開始時に回答画面へ渡す値。 */
export interface SessionStart {
  /** 最初の問題数。 */
  firstNumber: number
  /** 最初の回答のカテゴリ（復習画面の「解き直す」から始めたとき）。無ければ null。 */
  area: string | null
}

/** 学習 ID と最初の問題数（・カテゴリ）を保存する。 */
export function saveSessionStart(practiceId: number, firstNumber: number, area: string | null = null): void {
  try {
    window.sessionStorage.setItem(KEY, JSON.stringify({ practiceId, firstNumber, area }))
  } catch {
    // 保存できなくても動作に影響しないため無視する。
  }
}

/** 学習 ID に対応する開始時の値を返す。無ければ null。 */
export function readSessionStart(practiceId: number): SessionStart | null {
  try {
    const raw = window.sessionStorage.getItem(KEY)
    if (!raw) return null
    const value = JSON.parse(raw) as { practiceId?: number; firstNumber?: number; area?: string | null }
    if (value.practiceId !== practiceId || typeof value.firstNumber !== 'number') return null
    return { firstNumber: value.firstNumber, area: typeof value.area === 'string' ? value.area : null }
  } catch {
    return null
  }
}
