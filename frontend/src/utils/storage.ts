/**
 * ブラウザーに保存する利用者ごとの小さな設定（最後に開いた試験など）。
 *
 * プライベートモードなどで保存できない場合もあるため、失敗しても動作は止めない。
 */

const LAST_EXAM_KEY = 'study-record:last-exam-id'

/** 最後に開いた試験の ID を返す。 */
export function loadLastExamId(): number | null {
  try {
    const value = window.localStorage.getItem(LAST_EXAM_KEY)
    const id = value ? Number(value) : NaN
    return Number.isInteger(id) && id > 0 ? id : null
  } catch {
    return null
  }
}

/** 最後に開いた試験の ID を保存する。 */
export function saveLastExamId(id: number | null): void {
  try {
    if (id === null) window.localStorage.removeItem(LAST_EXAM_KEY)
    else window.localStorage.setItem(LAST_EXAM_KEY, String(id))
  } catch {
    // 保存できなくても動作に影響しないため無視する。
  }
}
