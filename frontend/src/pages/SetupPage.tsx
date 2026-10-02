/**
 * はじめての設定（試験を選ぶ → 試験日と 1 日の目標を決める → ダッシュボードへ）。
 *
 * 試験のテンプレートを選ぶと、出題分野のカテゴリがそろった状態で試験を作る。
 * 一覧に無い試験は名前を入力して作る（カテゴリは「未分類」だけになり、あとから追加できる）。
 * 試験が 1 つも無い利用者はトップからこの画面に案内する（試験の管理画面からも開ける）。
 */

import { useMemo, useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { examApi } from '../api/endpoints'
import type { ExamTemplate } from '../api/types'
import { Loading, Notice, PageHeader } from '../components/ui'
import { useLoad } from '../hooks/useLoad'
import { todayString } from '../utils/format'

/** 1 日の目標の候補（問）。 */
const GOAL_PRESETS = [10, 20, 30, 50]
/** 1 日の目標の初期値（問）。 */
const DEFAULT_GOAL = 20
/** 試験名の最大文字数（サーバー側の検証と合わせる）。 */
const EXAM_NAME_MAX = 100

/** 選んだ試験（テンプレート、または自分で入力した試験）。 */
type Choice = { kind: 'template'; template: ExamTemplate } | { kind: 'custom' }

/** はじめての設定の画面。 */
export function SetupPage() {
  const navigate = useNavigate()
  const templates = useLoad(() => examApi.templates(), [])
  const [choice, setChoice] = useState<Choice | null>(null)
  const [name, setName] = useState('')
  const [examDate, setExamDate] = useState('')
  const [goal, setGoal] = useState(String(DEFAULT_GOAL))
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /** 分類ごとにまとめたテンプレート（API の並び順を保つ）。 */
  const groups = useMemo(() => {
    const map = new Map<string, ExamTemplate[]>()
    for (const t of templates.data ?? []) map.set(t.group, [...(map.get(t.group) ?? []), t])
    return [...map.entries()]
  }, [templates.data])

  const choose = (next: Choice) => {
    setChoice(next)
    setName(next.kind === 'template' ? next.template.name : '')
    setError(null)
  }

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    const examName = name.trim()
    const goalNumber = goal.trim() === '' ? null : Number(goal)
    if (!examName || examName.length > EXAM_NAME_MAX) return setError(`試験名は1～${EXAM_NAME_MAX}文字で入力してください。`)
    if (goalNumber !== null && (!Number.isInteger(goalNumber) || goalNumber < 1 || goalNumber > 1000)) {
      return setError('1日の目標は1～1000問で入力してください。')
    }
    if (examDate && examDate < todayString()) return setError('試験日には今日以降の日付を選んでください。')
    setBusy(true)
    setError(null)
    try {
      const templateKey = choice?.kind === 'template' ? choice.template.key : undefined
      const exams = await examApi.create(examName, templateKey)
      // 作成した試験（同じ名前は 1 つだけなので名前で探す）に学習目標を設定する。
      const created = exams.find((x) => x.name === examName) ?? exams[exams.length - 1]
      if (examDate || goalNumber !== null) {
        await examApi.updateGoal(created.id, { examDate: examDate || null, dailyGoal: goalNumber })
      }
      navigate(`/exams/${created.id}`, { replace: true })
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="setup">
      <PageHeader
        title="はじめに：学習する試験を選びましょう"
        sub="試験を選ぶと、出題分野ごとのカテゴリがそろった状態で始められます。あとから変更・追加もできます。"
      />
      <ol className="setup-steps" aria-label="設定の手順">
        <li className={choice ? 'is-done' : 'is-current'}>1. 試験を選ぶ</li>
        <li className={choice ? 'is-current' : ''}>2. 試験日と目標を決める</li>
        <li>3. 学習を始める</li>
      </ol>

      {templates.loading && !templates.data ? (
        <Loading />
      ) : templates.error ? (
        <Notice>{templates.error}</Notice>
      ) : (
        <section aria-label="試験の一覧">
          {groups.map(([group, list]) => (
            <div key={group} className="template-group">
              <h2>{group}</h2>
              <div className="template-grid">
                {list.map((t) => {
                  const selected = choice?.kind === 'template' && choice.template.key === t.key
                  return (
                    <button
                      key={t.key}
                      type="button"
                      className={`template-card${selected ? ' is-selected' : ''}`}
                      aria-pressed={selected}
                      onClick={() => choose({ kind: 'template', template: t })}
                    >
                      <strong>{t.name}</strong>
                      <span className="muted small">{t.categories.join('・')}</span>
                    </button>
                  )
                })}
              </div>
            </div>
          ))}
          <div className="template-group">
            <h2>その他</h2>
            <div className="template-grid">
              <button
                type="button"
                className={`template-card${choice?.kind === 'custom' ? ' is-selected' : ''}`}
                aria-pressed={choice?.kind === 'custom'}
                onClick={() => choose({ kind: 'custom' })}
              >
                <strong>一覧に無い試験</strong>
                <span className="muted small">試験名を入力して作ります。カテゴリはあとから登録できます。</span>
              </button>
            </div>
          </div>
        </section>
      )}

      {choice && (
        <section className="card setup-form" aria-labelledby="setup-goal-heading">
          <h2 id="setup-goal-heading">試験日と 1 日の目標</h2>
          <p className="muted small">
            試験日を入れると、残り日数と必要なペースを毎日表示します。決まっていなければ空欄のままで構いません。
          </p>
          <Notice>{error}</Notice>
          <form className="stack" onSubmit={submit}>
            <label className="field">
              <span>試験名</span>
              <input value={name} maxLength={EXAM_NAME_MAX} required onChange={(e) => setName(e.target.value)} />
            </label>
            <div className="setup-row">
              <label className="field">
                <span>試験日（任意）</span>
                <input type="date" min={todayString()} value={examDate} onChange={(e) => setExamDate(e.target.value)} />
              </label>
              <label className="field">
                <span>1日の目標（問）</span>
                <input
                  type="number"
                  min={1}
                  max={1000}
                  step={1}
                  inputMode="numeric"
                  value={goal}
                  onChange={(e) => setGoal(e.target.value)}
                />
              </label>
            </div>
            <div className="goal-presets" role="group" aria-label="1日の目標の候補">
              {GOAL_PRESETS.map((n) => (
                <button
                  key={n}
                  type="button"
                  className={`chip${goal === String(n) ? ' is-selected' : ''}`}
                  onClick={() => setGoal(String(n))}
                >
                  {n}問
                </button>
              ))}
            </div>
            <button type="submit" className="button primary large" disabled={busy}>
              {busy ? '準備中…' : 'この内容で始める'}
            </button>
          </form>
        </section>
      )}
    </div>
  )
}
