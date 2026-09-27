/**
 * URL（/exams/:examId/...）から試験 ID を取り出すフック。
 */

import { useParams } from 'react-router-dom'

/** 現在の画面の試験 ID を返す。URL が不正な場合は 0（API が 404 を返す）。 */
export function useExamId(): number {
  const { examId } = useParams()
  const id = Number(examId)
  return Number.isInteger(id) && id > 0 ? id : 0
}
