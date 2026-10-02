/**
 * 実績（バッジ）の判定。
 *
 * ダッシュボードの集計（累計の解答数・連続学習日数・習得済みの問題数・準備度・学習カレンダー）から求める。
 * 保存はせず、表示のたびに計算する（集計が変われば実績も変わる）。
 */

import type { Dashboard } from '../api/types'

/** 実績 1 つ分。 */
export interface Achievement {
  /** 識別子。 */
  key: string
  /** 名前。 */
  title: string
  /** 達成の条件の説明。 */
  description: string
  /** 今の値。 */
  current: number
  /** 達成に必要な値。 */
  target: number
  /** 単位（「問」「日」など）。 */
  unit: string
}

/** 達成済みかどうか。 */
export function isAchieved(a: Achievement): boolean {
  return a.current >= a.target
}

/** 実績を種類ごとに段階を付けて作る。 */
function tiers(
  key: string,
  current: number,
  unit: string,
  steps: { target: number; title: string; description: string }[],
): Achievement[] {
  return steps.map((s) => ({ key: `${key}-${s.target}`, current, unit, ...s }))
}

/** ダッシュボードの集計から、すべての実績を求める（達成しやすい順）。 */
export function achievements(d: Dashboard): Achievement[] {
  const goal = d.today.dailyGoal
  const goalDays = goal ? d.heatmap.filter((day) => day.count >= goal).length : 0
  const list: Achievement[] = [
    ...tiers('answers', d.totalCount, '問', [
      { target: 1, title: 'はじめの一歩', description: '1 問目を採点する' },
      { target: 100, title: '100 問突破', description: '累計 100 問を採点する' },
      { target: 500, title: '500 問突破', description: '累計 500 問を採点する' },
      { target: 1000, title: '1000 問突破', description: '累計 1000 問を採点する' },
    ]),
    ...tiers('streak', d.today.longestStreak, '日', [
      { target: 3, title: '三日坊主卒業', description: '3 日続けて学習する' },
      { target: 7, title: '1 週間継続', description: '7 日続けて学習する' },
      { target: 30, title: '1 か月継続', description: '30 日続けて学習する' },
    ]),
    ...tiers('mastered', d.today.masteredCount, '問', [
      { target: 10, title: '習得 10 問', description: '10 問を習得済みにする（5 回連続で正解）' },
      { target: 50, title: '習得 50 問', description: '50 問を習得済みにする' },
      { target: 200, title: '習得 200 問', description: '200 問を習得済みにする' },
    ]),
    ...tiers('readiness', Math.floor(d.readiness.percent), '%', [
      { target: 50, title: '折り返し地点', description: '合格準備度 50% に到達する' },
      { target: 80, title: '合格圏へ', description: '合格準備度 80% に到達する' },
    ]),
  ]
  if (goal) {
    list.push(
      ...tiers('goal-days', goalDays, '日', [
        { target: 1, title: '目標達成', description: '1 日の目標を達成する' },
        { target: 7, title: '目標達成 7 日', description: '1 日の目標を 7 日達成する（過去 26 週）' },
      ]),
    )
  }
  return list
}

/** まだ達成していない実績のうち、達成に最も近いもの（達成率が高い順の先頭）。 */
export function nextAchievement(list: Achievement[]): Achievement | null {
  const pending = list.filter((a) => !isAchieved(a))
  if (pending.length === 0) return null
  return pending.reduce((best, a) => (a.current / a.target > best.current / best.target ? a : best))
}
