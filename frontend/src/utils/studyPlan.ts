/**
 * 学習計画の表示用の計算（習熟度の呼び方・復習日の相対表示・苦手なカテゴリ）。
 *
 * 習熟度と復習日そのものはサーバー（backend/src/domain/study_plan.rs）で計算する。
 */

import { MAX_MASTERY_LEVEL, type AreaStat, type QuestionItem } from '../api/types'
import { rateValue } from './format'

/** 苦手なカテゴリとして扱う最低の解答数（少なすぎる件数で判断しないため）。 */
export const WEAK_AREA_MIN_ANSWERS = 3
/** この正答率（%）未満のカテゴリを苦手とする。 */
export const WEAK_AREA_RATE = 80
/** 苦手なカテゴリとして表示する最大件数。 */
const WEAK_AREA_LIMIT = 3

/** 1 日のミリ秒。 */
const DAY_MS = 86_400_000

/** 習熟度の呼び方。 */
export function masteryLabel(level: number): string {
  if (level <= 0) return '要復習'
  if (level >= MAX_MASTERY_LEVEL) return '習得'
  if (level >= 3) return 'ほぼ定着'
  return '定着中'
}

/** 次の復習日（YYYY-MM-DD）を「今日」「3日後」「2日超過」のような相対表示にする。 */
export function reviewDueText(nextReviewOn: string, today: string): string {
  const diff = Math.round((Date.parse(nextReviewOn) - Date.parse(today)) / DAY_MS)
  if (diff === 0) return '今日'
  if (diff > 0) return `${diff}日後`
  return `${-diff}日超過`
}

/** 苦手なカテゴリ（解答数が一定以上で、正答率が低い順）を求める。 */
export function weakAreas(stats: AreaStat[]): AreaStat[] {
  return stats
    .filter((s) => s.inMaster && s.totalCount >= WEAK_AREA_MIN_ANSWERS)
    .filter((s) => rateValue(s.correctCount, s.totalCount) < WEAK_AREA_RATE)
    .sort((a, b) => rateValue(a.correctCount, a.totalCount) - rateValue(b.correctCount, b.totalCount))
    .slice(0, WEAK_AREA_LIMIT)
}

/** 問題一覧の並べ替え。 */
export type QuestionSort = 'default' | 'rate' | 'time' | 'mastery'

/** 並べ替えの選択肢。`default` はサーバーが返した順（一覧は最終回答が新しい順、今日の復習は復習を過ぎた日数が長い順）。 */
export const QUESTION_SORTS: { value: QuestionSort; label: string }[] = [
  { value: 'default', label: '最終回答が新しい順' },
  { value: 'rate', label: '正答率が低い順' },
  { value: 'time', label: '平均時間が長い順' },
  { value: 'mastery', label: '習熟度が低い順' },
]

/** 問題一覧を並べ替える（元の配列は変えない。同じ値どうしは元の順を保つ）。 */
export function sortQuestions(items: QuestionItem[], sort: QuestionSort): QuestionItem[] {
  const copy = [...items]
  switch (sort) {
    case 'rate':
      return copy.sort((a, b) => rateValue(a.correctCount, a.attemptCount) - rateValue(b.correctCount, b.attemptCount))
    case 'time':
      return copy.sort((a, b) => b.averageSeconds - a.averageSeconds)
    case 'mastery':
      return copy.sort((a, b) => a.masteryLevel - b.masteryLevel)
    default:
      return copy
  }
}
