/**
 * 「まとめて復習」の問題リスト（復習キュー）の一時保存と、次に解く問題の判定。
 *
 * 復習画面で選んだ問題を、学習 ID と組にして同じタブの sessionStorage に置く。
 * 回答画面は、リストのうちまだ記録していない最初の問題を、問題名称・問題数・カテゴリに入れて表示する。
 * 記録済みかどうかは学習の回答（問題名称＋問題数）で判断するため、回答を削除・修正しても順番が崩れない。
 * 保存できない環境では、ふつうの学習として続けられる（自動で入力されないだけ）。
 */

import type { Practice } from '../api/types'

const KEY = 'study-record:review-queue'

/** まとめて復習で一度に解く問題の上限（1 回の学習が長くなりすぎないようにする）。 */
export const REVIEW_QUEUE_MAX = 30

/** 復習する 1 問。 */
export interface QueueItem {
  title: string
  questionNumber: number
  area: string
}

/** 学習 ID と問題リストを保存する。 */
export function saveReviewQueue(practiceId: number, items: QueueItem[]): void {
  try {
    window.sessionStorage.setItem(KEY, JSON.stringify({ practiceId, items }))
  } catch {
    // 保存できなくても学習は続けられるため無視する。
  }
}

/** 学習 ID に対応する問題リストを返す。無ければ null。 */
export function readReviewQueue(practiceId: number): QueueItem[] | null {
  try {
    const raw = window.sessionStorage.getItem(KEY)
    if (!raw) return null
    const value = JSON.parse(raw) as { practiceId?: number; items?: QueueItem[] }
    return value.practiceId === practiceId && Array.isArray(value.items) ? value.items : null
  } catch {
    return null
  }
}

/** 問題リストの進み具合。 */
export interface QueueProgress {
  /** 次に解く問題（すべて記録済みなら null）。 */
  next: QueueItem | null
  /** 記録済みの問題数。 */
  done: number
  /** リストの問題数。 */
  total: number
}

/** 問題リストの 1 問を識別するキー（問題名称＋問題数）。 */
export function queueKey(item: { title: string; questionNumber: number }): string {
  return `${item.title}\u0000${item.questionNumber}`
}

/**
 * 学習の回答から、問題リストの進み具合と次に解く問題を求める。
 *
 * `skipped` は利用者が「とばす」を選んだ問題のキー（次の問題の候補から外す）。
 */
export function queueProgress(
  practice: Practice,
  items: QueueItem[],
  skipped: ReadonlySet<string> = new Set(),
): QueueProgress {
  const recorded = new Set(practice.answers.map(queueKey))
  const isDone = (item: QueueItem) => recorded.has(queueKey(item))
  return {
    next: items.find((item) => !isDone(item) && !skipped.has(queueKey(item))) ?? null,
    done: items.filter(isDone).length,
    total: items.length,
  }
}
