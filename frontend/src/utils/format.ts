/**
 * 表示用の書式を整える関数群。
 */

/** 正答率を「小数第 1 位までの %」で返す。分母が 0 なら 0.0%。 */
export function formatRate(correct: number, total: number): string {
  if (total <= 0) return '0.0%'
  return `${((correct / total) * 100).toFixed(1)}%`
}

/** 正答率を 0〜100 の数値で返す（グラフ用）。 */
export function rateValue(correct: number, total: number): number {
  return total <= 0 ? 0 : (correct / total) * 100
}

/** 秒数を「1分05秒」のような表示にする。60 秒未満は「45秒」。 */
export function formatSeconds(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds))
  const minutes = Math.floor(total / 60)
  const rest = total % 60
  return minutes > 0 ? `${minutes}分${String(rest).padStart(2, '0')}秒` : `${rest}秒`
}

/** 日時文字列（YYYY-MM-DD HH:MM:SS）から秒を落として表示する。 */
export function formatDateTime(value: string | null | undefined): string {
  if (!value) return '―'
  return value.slice(0, 16).replace('T', ' ')
}

/** 日付（YYYY-MM-DD）を「9/28」の形で表示する。 */
export function formatShortDate(value: string): string {
  const [, month, day] = value.split('-')
  return `${Number(month)}/${Number(day)}`
}

/** 数値を 3 桁区切りで表示する。 */
export function formatCount(value: number): string {
  return value.toLocaleString('ja-JP')
}

/** 今日の日付（端末の時刻、YYYY-MM-DD）を返す。復習日との比較に使う。 */
export function todayString(): string {
  const now = new Date()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${now.getFullYear()}-${month}-${day}`
}
