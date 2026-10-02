/**
 * 学習計画の部品（今日やること・学習目標・習熟度・合格準備度・学習カレンダー・苦手なカテゴリ）。
 *
 * 習熟度と復習日の決め方は backend/src/domain/study_plan.rs を参照。
 */

import { useEffect, useRef, useState, type FormEvent } from 'react'
import { Link } from 'react-router-dom'

import { errorMessage } from '../api/client'
import type { GoalInput } from '../api/endpoints'
import {
  MAX_MASTERY_LEVEL,
  type AreaStat,
  type DailyCount,
  type Dashboard,
  type Readiness,
  type ReadinessPoint,
  type TodayPlan,
} from '../api/types'
import { achievements, isAchieved, nextAchievement } from '../utils/achievements'
import { formatRate } from '../utils/format'
import { masteryLabel, WEAK_AREA_MIN_ANSWERS, WEAK_AREA_RATE, weakAreas } from '../utils/studyPlan'
import { Notice } from './ui'

// ---------------------------------------------------------------------------
// 今日やること
// ---------------------------------------------------------------------------

/** 試験日までの残り日数の表示。 */
function countdownText(days: number | null): { value: string; unit: string; note: string } {
  if (days === null) return { value: '―', unit: '', note: '試験日を設定すると残り日数を表示します' }
  if (days > 0) return { value: String(days), unit: '日', note: '試験まであと' }
  if (days === 0) return { value: '当日', unit: '', note: '今日が試験日です。落ち着いて！' }
  return { value: String(-days), unit: '日経過', note: '試験日を過ぎました。次の目標を設定しましょう' }
}

/**
 * 今日やること（試験日までの日数・今日の目標・連続学習日数・今日の復習）。
 *
 * 「今日の復習」は忘却曲線にもとづいて復習の時期が来た問題の数で、復習画面へ移動できる。
 */
export function TodayPanel({
  examId,
  today,
  onSaveGoal,
}: {
  examId: number
  today: TodayPlan
  onSaveGoal: (goal: GoalInput) => Promise<void>
}) {
  const [editing, setEditing] = useState(false)
  const countdown = countdownText(today.daysUntilExam)
  const goal = today.dailyGoal
  const progress = goal ? Math.min(100, (today.answeredCount / goal) * 100) : 0
  const achieved = goal !== null && today.answeredCount >= goal
  const remaining = goal !== null ? Math.max(0, goal - today.answeredCount) : 0

  return (
    <section className="today card" aria-labelledby="today-heading">
      <div className="today-head">
        <h2 id="today-heading">今日やること</h2>
        <button type="button" className="link-button" onClick={() => setEditing((v) => !v)}>
          {editing ? '閉じる' : '試験日・目標を設定'}
        </button>
      </div>

      {editing && (
        <GoalForm
          examDate={today.examDate}
          dailyGoal={today.dailyGoal}
          onSave={async (input) => {
            await onSaveGoal(input)
            setEditing(false)
          }}
        />
      )}

      <div className="today-grid">
        <div className="today-item">
          <span className="today-label">{countdown.note}</span>
          <span className="today-value">
            {countdown.value}
            {countdown.unit && <small>{countdown.unit}</small>}
          </span>
          {today.examDate && <span className="muted small">試験日 {today.examDate}</span>}
        </div>

        <div className="today-item">
          <span className="today-label">今日の目標</span>
          {goal === null ? (
            <>
              <span className="today-value">
                {today.answeredCount}
                <small>問</small>
              </span>
              <span className="muted small">1日の目標を設定すると達成度を表示します</span>
            </>
          ) : (
            <>
              <span className="today-value">
                {today.answeredCount}
                <small>/ {goal}問</small>
              </span>
              <span
                className="goal-track"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={goal}
                aria-valuenow={Math.min(goal, today.answeredCount)}
                aria-label="今日の目標の達成度"
              >
                <span className={`goal-fill${achieved ? ' is-done' : ''}`} style={{ width: `${progress}%` }} />
              </span>
              <span className={achieved ? 'good-text small' : 'muted small'}>
                {achieved ? '目標達成！この調子です' : `あと ${remaining} 問`}
              </span>
            </>
          )}
        </div>

        <div className="today-item">
          <span className="today-label">連続学習</span>
          <span className="today-value">
            {today.currentStreak}
            <small>日</small>
          </span>
          <span className="muted small">
            {today.currentStreak > 0 && today.answeredCount === 0
              ? '今日も解くと記録が伸びます'
              : `最長 ${today.longestStreak} 日`}
          </span>
        </div>

        <div className="today-item today-review">
          <span className="today-label">今日の復習</span>
          <span className="today-value">
            {today.dueCount}
            <small>問</small>
          </span>
          {today.dueCount > 0 ? (
            <Link className="button small primary" to={`/exams/${examId}/review`}>
              復習する
            </Link>
          ) : (
            <span className="muted small">
              {today.questionCount > 0 ? '復習の時期が来た問題はありません' : '問題を解くと復習日を計算します'}
            </span>
          )}
        </div>
      </div>

      {today.questionCount > 0 ? (
        <p className="today-foot muted small">
          習得済み {today.masteredCount} / {today.questionCount} 問（{MAX_MASTERY_LEVEL} 回連続で正解した問題）
        </p>
      ) : (
        // まだ 1 問も解いていない利用者には、最初にやることをはっきり示す。
        <div className="first-step">
          <div>
            <strong>まずは 1 回、問題を解いてみましょう</strong>
            <p className="muted small">
              問題集の名前と番号、自分の回答を記録して自己採点するだけ。採点すると、復習日と準備度をアプリが計算します。
            </p>
          </div>
          <Link className="button primary" to={`/exams/${examId}/start`}>
            最初の学習を始める
          </Link>
        </div>
      )}
    </section>
  )
}

/** 試験日と 1 日の目標を入力するフォーム。空欄にすると未設定に戻す。 */
function GoalForm({
  examDate,
  dailyGoal,
  onSave,
}: {
  examDate: string | null
  dailyGoal: number | null
  onSave: (goal: GoalInput) => Promise<void>
}) {
  const [date, setDate] = useState(examDate ?? '')
  const [goal, setGoal] = useState(dailyGoal ? String(dailyGoal) : '')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    const goalNumber = goal.trim() === '' ? null : Number(goal)
    if (goalNumber !== null && (!Number.isInteger(goalNumber) || goalNumber < 1 || goalNumber > 1000)) {
      setError('1日の目標は1～1000問で入力してください。')
      return
    }
    setBusy(true)
    setError(null)
    try {
      await onSave({ examDate: date || null, dailyGoal: goalNumber })
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <form className="goal-form" onSubmit={submit}>
      <Notice>{error}</Notice>
      <label className="field">
        <span>試験日</span>
        <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
      </label>
      <label className="field">
        <span>1日の目標（問）</span>
        <input
          type="number"
          min={1}
          max={1000}
          step={1}
          inputMode="numeric"
          placeholder="例：20"
          value={goal}
          onChange={(e) => setGoal(e.target.value)}
        />
      </label>
      <button type="submit" className="button primary" disabled={busy}>
        保存する
      </button>
    </form>
  )
}

// ---------------------------------------------------------------------------
// 習熟度
// ---------------------------------------------------------------------------

/** 習熟度の表示（5 段階の目盛りと呼び方。色だけに頼らず文字も出す）。 */
export function MasteryMeter({ level }: { level: number }) {
  return (
    <span className="mastery" title={`習熟度 ${level} / ${MAX_MASTERY_LEVEL}（連続で正解した回数）`}>
      <span className="mastery-steps" aria-hidden="true">
        {Array.from({ length: MAX_MASTERY_LEVEL }, (_, i) => (
          <span key={i} className={`mastery-step${i < level ? ' is-on' : ''}`} />
        ))}
      </span>
      <span className={`mastery-label level-${Math.min(level, MAX_MASTERY_LEVEL)}`}>
        <span className="visually-hidden">習熟度 {level} / {MAX_MASTERY_LEVEL}：</span>
        {masteryLabel(level)}
      </span>
    </span>
  )
}

// ---------------------------------------------------------------------------
// 学習カレンダー
// ---------------------------------------------------------------------------

/** 曜日の見出し（日曜始まり）。 */
const WEEKDAYS = ['日', '月', '火', '水', '木', '金', '土']

/** 件数を 0〜4 の濃さに分ける。基準は 1 日の目標（無ければ期間中の最大件数）。 */
function heatLevel(count: number, base: number): number {
  if (count <= 0) return 0
  const ratio = count / Math.max(1, base)
  if (ratio < 0.25) return 1
  if (ratio < 0.5) return 2
  if (ratio < 1) return 3
  return 4
}

/**
 * 学習カレンダー（過去 26 週の日別の解答数を色の濃さで表す）。
 *
 * 列が週（日曜始まり）、行が曜日。1 日の目標がある場合は、目標を達成した日が最も濃くなる。
 */
export function StudyHeatmap({ data, dailyGoal }: { data: DailyCount[]; dailyGoal: number | null }) {
  const scrollRef = useRef<HTMLDivElement>(null)
  // 狭い画面では横にスクロールするため、最初は直近の週（右端）を見せる。
  useEffect(() => {
    const el = scrollRef.current
    if (el) el.scrollLeft = el.scrollWidth
  }, [data])
  if (data.length === 0) return null
  const base = dailyGoal ?? Math.max(1, ...data.map((d) => d.count))
  // 最初の週の日曜日までを空きで埋める。
  const firstWeekday = new Date(`${data[0].date}T00:00:00`).getDay()
  const cells: (DailyCount | null)[] = [...Array<null>(firstWeekday).fill(null), ...data]
  const weeks: (DailyCount | null)[][] = []
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7))
  const studiedDays = data.filter((d) => d.count > 0).length
  const total = data.reduce((sum, d) => sum + d.count, 0)

  return (
    <figure className="chart">
      <div className="chart-head">
        <figcaption>
          <h2>学習カレンダー</h2>
          <p className="muted">
            過去26週で {studiedDays} 日・{total.toLocaleString('ja-JP')} 問
            {dailyGoal ? '（最も濃い色が目標達成の日）' : ''}
          </p>
        </figcaption>
      </div>
      <div ref={scrollRef} className="heatmap" role="img" aria-label={`過去26週の学習日数 ${studiedDays}日、解答数 ${total}問`}>
        <div className="heatmap-weekdays" aria-hidden="true">
          {WEEKDAYS.map((w, i) => (
            <span key={w}>{i % 2 === 1 ? w : ''}</span>
          ))}
        </div>
        <div className="heatmap-weeks">
          {weeks.map((week, wi) => (
            <div key={wi} className="heatmap-week">
              {week.map((day, di) =>
                day ? (
                  <span
                    key={day.date}
                    className={`heat heat-${heatLevel(day.count, base)}`}
                    title={`${day.date}：${day.count}問（正解 ${day.correctCount}）`}
                  />
                ) : (
                  <span key={`pad-${di}`} className="heat heat-pad" />
                ),
              )}
            </div>
          ))}
        </div>
      </div>
      <div className="heatmap-legend muted small" aria-hidden="true">
        少ない
        {[0, 1, 2, 3, 4].map((level) => (
          <span key={level} className={`heat heat-${level}`} />
        ))}
        多い
      </div>
    </figure>
  )
}

// ---------------------------------------------------------------------------
// 合格準備度とペース
// ---------------------------------------------------------------------------

/** ペースの判定文。 */
function paceMessage(readiness: Readiness, daysUntilExam: number | null): { text: string; tone: 'good' | 'bad' | 'muted' } {
  const { requiredDaily, recentDailyAverage, remainingCorrectAnswers } = readiness
  // カテゴリ別の準備度は解いた問題があるカテゴリだけなので、空なら 1 問も解いていない。
  if (readiness.areas.length === 0) return { text: '問題を解くと、準備度と必要なペースを計算します。', tone: 'muted' }
  if (remainingCorrectAnswers === 0) return { text: 'すべての問題が習得済みです。新しい問題に挑戦しましょう。', tone: 'good' }
  if (daysUntilExam === null) return { text: '試験日を設定すると、必要なペースを計算します。', tone: 'muted' }
  if (requiredDaily === null) return { text: '試験日を過ぎたため、ペースは計算しません。', tone: 'muted' }
  const average = Math.round(recentDailyAverage * 10) / 10
  if (recentDailyAverage >= requiredDaily) {
    return { text: `直近の平均 ${average} 問/日 は必要なペースを上回っています。この調子です。`, tone: 'good' }
  }
  const gap = Math.ceil(requiredDaily - recentDailyAverage)
  return { text: `直近の平均は ${average} 問/日。あと 1 日 ${gap} 問ほどペースを上げましょう。`, tone: 'bad' }
}

/**
 * 合格準備度（習熟度の平均）と、試験日までに全問を習得するためのペースの目安。
 *
 * 準備度は「全問が習得済み（5 回連続で正解）なら 100%」。カテゴリ別の準備度も表示する。
 */
export function ReadinessCard({ readiness, daysUntilExam }: { readiness: Readiness; daysUntilExam: number | null }) {
  const pace = paceMessage(readiness, daysUntilExam)
  const percent = Math.round(readiness.percent)
  return (
    <section className="readiness card" aria-labelledby="readiness-heading">
      <div className="chart-head">
        <div>
          <h2 id="readiness-heading">合格準備度</h2>
          <p className="muted small">解いた問題の習熟度の平均（全問を習得すると 100%）</p>
        </div>
      </div>
      <div className="readiness-main">
        <span className="readiness-value">
          {percent}
          <small>%</small>
        </span>
        <span
          className="readiness-track"
          role="meter"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          aria-label="合格準備度"
        >
          <span className="readiness-fill" style={{ width: `${percent}%` }} />
        </span>
      </div>
      <dl className="pace">
        <div>
          <dt>習得までに必要な正解</dt>
          <dd>
            {readiness.remainingCorrectAnswers.toLocaleString('ja-JP')}
            <small>回</small>
          </dd>
        </div>
        <div>
          <dt>必要なペース</dt>
          <dd>
            {readiness.requiredDaily === null ? '―' : readiness.requiredDaily}
            {readiness.requiredDaily !== null && <small>問/日</small>}
          </dd>
        </div>
        <div>
          <dt>直近14日の平均</dt>
          <dd>
            {(Math.round(readiness.recentDailyAverage * 10) / 10).toLocaleString('ja-JP')}
            <small>問/日</small>
          </dd>
        </div>
      </dl>
      <p className={`small ${pace.tone === 'good' ? 'good-text' : pace.tone === 'bad' ? 'bad-text' : 'muted'}`}>{pace.text}</p>
      <ReadinessTrend history={readiness.history} />
      {readiness.areas.length > 0 && (
        <ul className="readiness-areas" aria-label="カテゴリ別の準備度">
          {readiness.areas.map((a) => (
            <li key={a.area}>
              <span className="area-name">{a.area}</span>
              <span className="bar-track" aria-hidden="true">
                {a.percent > 0 && <span className="bar-fill" style={{ width: `${a.percent}%` }} />}
              </span>
              <span className="area-value">
                {Math.round(a.percent)}%
                <span className="muted">
                  {' '}
                  習得 {a.masteredCount}/{a.questionCount}
                </span>
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

/** 準備度の推移のグラフの大きさ（SVG の座標）。 */
const TREND_WIDTH = 320
const TREND_HEIGHT = 90
const TREND_PAD = 6

/**
 * 準備度の推移（8 週分の折れ線）。最初と最後の差を「+N ポイント」で示す。
 *
 * まだ何も解いていなかった週は線を引かない（0% と区別するため）。
 */
export function ReadinessTrend({ history }: { history: ReadinessPoint[] }) {
  const points = history.filter((p) => p.questionCount > 0)
  if (points.length < 2) {
    return <p className="muted small">2 週間以上続けると、準備度の推移を表示します。</p>
  }
  const step = (TREND_WIDTH - TREND_PAD * 2) / Math.max(1, history.length - 1)
  const xy = (p: ReadinessPoint) => {
    const index = history.indexOf(p)
    return [TREND_PAD + index * step, TREND_HEIGHT - TREND_PAD - (p.percent / 100) * (TREND_HEIGHT - TREND_PAD * 2)]
  }
  const path = points.map((p, i) => `${i === 0 ? 'M' : 'L'}${xy(p)[0].toFixed(1)},${xy(p)[1].toFixed(1)}`).join(' ')
  const first = points[0]
  const last = points[points.length - 1]
  const delta = Math.round(last.percent - first.percent)

  return (
    <figure className="trend">
      <figcaption className="trend-head">
        <span className="today-label">準備度の推移（週ごと）</span>
        <span className={delta > 0 ? 'good-text small' : 'muted small'}>
          {delta > 0 ? `+${delta}` : delta} ポイント（{first.date.slice(5).replace('-', '/')} から）
        </span>
      </figcaption>
      <svg
        viewBox={`0 0 ${TREND_WIDTH} ${TREND_HEIGHT}`}
        className="trend-chart"
        role="img"
        aria-label={`準備度は ${first.date} の ${Math.round(first.percent)}% から ${last.date} の ${Math.round(last.percent)}% になりました`}
        preserveAspectRatio="none"
      >
        <line className="trend-grid" x1={TREND_PAD} x2={TREND_WIDTH - TREND_PAD} y1={TREND_HEIGHT / 2} y2={TREND_HEIGHT / 2} />
        <path className="trend-line" d={path} vectorEffect="non-scaling-stroke" />
        {points.map((p) => {
          const [x, y] = xy(p)
          // 横に引き伸ばしても丸く見えるよう、長さ 0 の線を丸い端で描く（circle は楕円になるため）。
          return (
            <g key={p.date}>
              <line className="trend-dot-ring" x1={x} y1={y} x2={x} y2={y} vectorEffect="non-scaling-stroke" />
              <line className="trend-dot" x1={x} y1={y} x2={x} y2={y} vectorEffect="non-scaling-stroke">
                <title>
                  {p.date}：{Math.round(p.percent)}%（{p.questionCount} 問）
                </title>
              </line>
            </g>
          )
        })}
      </svg>
    </figure>
  )
}

// ---------------------------------------------------------------------------
// 実績
// ---------------------------------------------------------------------------

/** 実績（バッジ）の一覧と、次に達成できそうな実績。 */
export function AchievementsCard({ dashboard }: { dashboard: Dashboard }) {
  const list = achievements(dashboard)
  const earned = list.filter(isAchieved)
  const next = nextAchievement(list)
  return (
    <section aria-labelledby="achievements-heading">
      <div className="chart-head">
        <h2 id="achievements-heading">実績</h2>
        <span className="muted small">
          {earned.length} / {list.length}
        </span>
      </div>
      {next && (
        <div className="next-badge">
          <span className="small">
            次の実績：<strong>{next.title}</strong>（{next.description}）
          </span>
          <span
            className="goal-track"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={next.target}
            aria-valuenow={Math.min(next.current, next.target)}
            aria-label={`${next.title}の進み具合`}
          >
            <span className="goal-fill" style={{ width: `${Math.min(100, (next.current / next.target) * 100)}%` }} />
          </span>
          <span className="muted small">
            あと {(next.target - next.current).toLocaleString('ja-JP')} {next.unit}
          </span>
        </div>
      )}
      <ul className="badges" aria-label="実績の一覧">
        {list.map((a) => {
          const done = isAchieved(a)
          return (
            <li key={a.key} className={`badge-item${done ? ' is-earned' : ''}`} title={a.description}>
              <span className="badge-mark" aria-hidden="true">
                {done ? '★' : '☆'}
              </span>
              <span className="badge-title">{a.title}</span>
              <span className="visually-hidden">{done ? '達成済み' : '未達成'}</span>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

// ---------------------------------------------------------------------------
// 苦手なカテゴリ
// ---------------------------------------------------------------------------

/** 苦手なカテゴリの一覧。選ぶとそのカテゴリの問題一覧へ移動する。 */
export function WeakAreas({ stats, onSelect }: { stats: AreaStat[]; onSelect: (stat: AreaStat) => void }) {
  const weak = weakAreas(stats)
  return (
    <section aria-labelledby="weak-heading">
      <h2 id="weak-heading">苦手なカテゴリ</h2>
      {weak.length === 0 ? (
        <p className="muted small">
          {stats.some((s) => s.totalCount >= WEAK_AREA_MIN_ANSWERS)
            ? `正答率が${WEAK_AREA_RATE}%未満のカテゴリはありません。`
            : `カテゴリごとに${WEAK_AREA_MIN_ANSWERS}問以上解くと、苦手なカテゴリを表示します。`}
        </p>
      ) : (
        <ol className="weak-list">
          {weak.map((stat) => (
            <li key={stat.area}>
              <button type="button" className="weak-row" onClick={() => onSelect(stat)}>
                <span className="weak-name">{stat.area}</span>
                <span className="bad-text">{formatRate(stat.correctCount, stat.totalCount)}</span>
                <span className="muted small">
                  {stat.correctCount}/{stat.totalCount}
                </span>
              </button>
            </li>
          ))}
        </ol>
      )}
    </section>
  )
}
