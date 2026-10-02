/**
 * 学習中の画面（F-02〜F-06）：回答の記録 ⇄ 自己採点。
 *
 * - 回答の記録：問題名称・問題数・カテゴリ・回答（自由記述、改行可）と、自動計測の回答時間
 * - 採点：未採点の回答を記録順に 1 件ずつ正解／不正解にする。最後の 1 件で学習が完了する
 * - 画面を開き直した場合は、回答の記録から再開する（F-04）
 * - まとめて復習（復習画面から開始）では、復習リストの問題を 1 問ずつ問題名称・問題数・カテゴリに入れる
 */

import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from 'react'
import { useNavigate } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { examApi, practiceApi, type AnswerInput } from '../api/endpoints'
import type { Answer, Category, Practice } from '../api/types'
import { Loading, MultilineText, Notice, ResultBadge } from '../components/ui'
import { useApp } from '../hooks/useApp'
import { useTimer } from '../hooks/useTimer'
import { formatSeconds } from '../utils/format'
import { queueKey, queueProgress, readReviewQueue, type QueueItem } from '../utils/reviewQueue'
import { readSessionStart } from '../utils/session'

/** 回答の最大文字数（サーバー側の検証と合わせる）。 */
const RESPONSE_MAX = 255
/** メモの最大文字数（サーバー側の検証と合わせる）。 */
const NOTE_MAX = 1000

/** 回答フォームの入力値。 */
interface AnswerForm {
  title: string
  questionNumber: string
  area: string
  response: string
}

/** フォームの入力値を API の入力に変換する。問題があればメッセージを返す。 */
function toInput(form: AnswerForm, elapsedSeconds: number): AnswerInput | string {
  const number = Number(form.questionNumber)
  if (!form.title.trim()) return '問題名称を入力してください。'
  if (!Number.isInteger(number) || number < 1) return '問題数は1以上の数字で入力してください。'
  if (!form.area) return 'カテゴリを選択してください。'
  if (!form.response.trim()) return '回答を入力してください。'
  if (form.response.length > RESPONSE_MAX) return `回答は${RESPONSE_MAX}文字以内で入力してください。`
  return { title: form.title, questionNumber: number, area: form.area, response: form.response, elapsedSeconds }
}

/** 学習中の画面。 */
export function SessionPage() {
  const { activePracticeId, setActivePracticeId } = useApp()
  const navigate = useNavigate()
  const [practice, setPractice] = useState<Practice | null>(null)
  const [categories, setCategories] = useState<Category[]>([])
  const [loadError, setLoadError] = useState<string | null>(null)
  const [mode, setMode] = useState<'answer' | 'grade'>('answer')

  // 進行中の学習とカテゴリを読み込む。
  useEffect(() => {
    if (activePracticeId === null) return
    let cancelled = false
    ;(async () => {
      try {
        const loaded = await practiceApi.get(activePracticeId)
        const cats = await examApi.categories(loaded.examId)
        if (cancelled) return
        setPractice(loaded)
        setCategories(cats)
      } catch (err) {
        if (!cancelled) setLoadError(errorMessage(err))
      }
    })()
    return () => {
      cancelled = true
    }
  }, [activePracticeId])

  if (activePracticeId === null) {
    return (
      <div className="narrow">
        <Notice kind="info">進行中の学習はありません。</Notice>
        <button type="button" className="button" onClick={() => navigate('/')}>
          ダッシュボードへ
        </button>
      </div>
    )
  }
  if (loadError) return <Notice>{loadError}</Notice>
  if (!practice) return <Loading />

  /** 学習が完了したら結果画面へ移る。 */
  const handleUpdated = (updated: Practice) => {
    setPractice(updated)
    if (updated.completedAt) {
      setActivePracticeId(null)
      navigate(`/result/${updated.id}`, { replace: true })
    }
  }

  /** 学習を中断する（未採点の回答を捨て、採点済みだけ残す）。 */
  const abort = async () => {
    const ungraded = practice.questionCount - practice.gradedCount
    const message =
      ungraded > 0
        ? `学習を中断します。未採点の回答 ${ungraded} 件は削除されます。よろしいですか？`
        : '学習を中断します。よろしいですか？'
    if (!window.confirm(message)) return
    try {
      const result = await practiceApi.abort(practice.id)
      setActivePracticeId(null)
      if (result.practice) navigate(`/result/${result.practice.id}`, { replace: true })
      else navigate(`/exams/${practice.examId}`, { replace: true })
    } catch (err) {
      setLoadError(errorMessage(err))
    }
  }

  return (
    <div className="session">
      <div className="session-head">
        <div>
          <p className="muted small">{practice.examName}</p>
          <h1>{mode === 'answer' ? '回答の記録' : '採点'}</h1>
        </div>
        <div className="session-progress" aria-label="進み具合">
          <span>
            記録 <strong>{practice.questionCount}</strong> 件
          </span>
          <span>
            採点済み <strong>{practice.gradedCount}</strong> / {practice.questionCount}
          </span>
        </div>
        <button type="button" className="button danger small" onClick={abort}>
          学習を中断
        </button>
      </div>
      {mode === 'answer' ? (
        <AnswerView
          practice={practice}
          categories={categories}
          onUpdated={handleUpdated}
          onGoGrade={() => setMode('grade')}
        />
      ) : (
        // 採点する回答が変わるたびに作り直し、正解・メモの入力を空に戻す。
        <GradeView
          key={practice.answers.find((a) => a.correct === null)?.id ?? 'none'}
          practice={practice}
          onUpdated={handleUpdated}
          onBack={() => setMode('answer')}
        />
      )}
    </div>
  )
}

/** 復習リストの問題をフォームの値にする（カテゴリがマスタに無ければ先頭のカテゴリ）。 */
function formFromQueue(item: QueueItem, categories: Category[]): AnswerForm {
  return {
    title: item.title,
    questionNumber: String(item.questionNumber),
    area: categories.some((c) => c.name === item.area) ? item.area : (categories[0]?.name ?? ''),
    response: '',
  }
}

/** 再開時・開始時のフォームの初期値を作る（F-04）。まとめて復習中は、リストの次の問題を入れる。 */
function initialForm(practice: Practice, categories: Category[]): AnswerForm {
  const queue = readReviewQueue(practice.id)
  const next = queue ? queueProgress(practice, queue).next : null
  if (next) return formFromQueue(next, categories)
  const last = practice.answers[practice.answers.length - 1]
  const start = readSessionStart(practice.id)
  // 「解き直す」から始めた最初の 1 問は、前回のカテゴリを選んでおく（マスタに無ければ先頭）。
  const retryArea = !last && start?.area && categories.some((c) => c.name === start.area) ? start.area : null
  return {
    title: last?.title ?? practice.title,
    questionNumber: last ? String(last.questionNumber + 1) : start ? String(start.firstNumber) : '',
    area: retryArea ?? categories[0]?.name ?? '',
    response: '',
  }
}

/** 回答の記録（F-02, F-03）。 */
function AnswerView({
  practice,
  categories,
  onUpdated,
  onGoGrade,
}: {
  practice: Practice
  categories: Category[]
  onUpdated: (practice: Practice) => void
  onGoGrade: () => void
}) {
  const timer = useTimer()
  // まとめて復習の問題リスト（ふつうの学習なら null）。学習中は変わらないため最初に 1 回だけ読む。
  const [queue] = useState(() => readReviewQueue(practice.id))
  // 「とばす」を選んだ問題（この画面を開いている間だけ覚える）。
  const [skipped, setSkipped] = useState<ReadonlySet<string>>(() => new Set())
  const progress = queue ? queueProgress(practice, queue, skipped) : null
  const [form, setForm] = useState<AnswerForm>(() => initialForm(practice, categories))
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const responseRef = useRef<HTMLTextAreaElement>(null)

  const update = (patch: Partial<AnswerForm>) => setForm((f) => ({ ...f, ...patch }))

  /** 復習リストの今の問題をとばし、次の問題を入れる（手元に問題が無いときなど）。 */
  const skip = () => {
    if (!queue || !progress?.next) return
    const nextSkipped = new Set(skipped).add(queueKey(progress.next))
    setSkipped(nextSkipped)
    const next = queueProgress(practice, queue, nextSkipped).next
    if (next) setForm(formFromQueue(next, categories))
    else setForm((f) => ({ ...f, response: '' }))
    timer.reset()
  }

  /** 入力中の回答を記録する。成功したら true。 */
  const record = async (): Promise<boolean> => {
    const input = toInput(form, timer.read())
    if (typeof input === 'string') {
      setError(input)
      return false
    }
    setBusy(true)
    setError(null)
    try {
      const updated = await practiceApi.recordAnswer(practice.id, input)
      onUpdated(updated)
      // 記録後：まとめて復習中はリストの次の問題を入れる。
      // それ以外は問題数を +1、カテゴリを先頭に戻し、回答欄を空にする。どちらも計測はやり直す。
      const next = queue ? queueProgress(updated, queue, skipped).next : null
      setForm((f) =>
        next
          ? formFromQueue(next, categories)
          : {
              ...f,
              questionNumber: String(input.questionNumber + 1),
              area: categories[0]?.name ?? '',
              response: '',
            },
      )
      timer.reset()
      responseRef.current?.focus()
      return true
    } catch (err) {
      setError(errorMessage(err))
      return false
    } finally {
      setBusy(false)
    }
  }

  const submit = (e: FormEvent) => {
    e.preventDefault()
    void record()
  }

  /** 「採点へ」：入力があれば記録してから採点に移る。回答も入力も無ければ移らない（F-05）。 */
  const goGrade = async () => {
    if (form.response.trim()) {
      if (!(await record())) return
    } else if (practice.questionCount === 0) {
      setError('採点する回答がありません。回答を記録してください。')
      return
    }
    onGoGrade()
  }

  /** Ctrl+Enter（Mac は ⌘+Enter）で記録する。Enter だけなら改行。 */
  const onResponseKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      void record()
    }
  }

  return (
    <>
      <section className="card">
        {categories.length === 0 && (
          <Notice>カテゴリがありません。学習を中断して、カテゴリ画面で登録してください。</Notice>
        )}
        <Notice>{error}</Notice>
        {progress && (
          <div className="queue-banner" role="status">
            <div className="queue-head">
              <strong>まとめて復習</strong>
              <span className="muted small">
                {progress.done} / {progress.total} 問を記録
              </span>
            </div>
            <span
              className="goal-track"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={progress.total}
              aria-valuenow={progress.done}
              aria-label="復習リストの進み具合"
            >
              <span
                className={`goal-fill${progress.next ? '' : ' is-done'}`}
                style={{ width: `${(progress.done / Math.max(1, progress.total)) * 100}%` }}
              />
            </span>
            <div className="queue-foot">
              <span className="small">
                {progress.next
                  ? `いまの問題：${progress.next.title} 問${progress.next.questionNumber}（${progress.next.area}）`
                  : skipped.size > 0
                    ? `とばした ${skipped.size} 問以外はすべて記録しました。「採点へ」で答え合わせをしましょう。`
                    : 'リストの問題はすべて記録しました。「採点へ」で答え合わせをしましょう。'}
              </span>
              {progress.next && (
                <button type="button" className="link-button small" onClick={skip}>
                  この問題をとばす
                </button>
              )}
            </div>
          </div>
        )}
        <form className="answer-form" onSubmit={submit}>
          <label className="field span-2">
            <span>問題名称</span>
            <input value={form.title} maxLength={100} required onChange={(e) => update({ title: e.target.value })} />
          </label>
          <label className="field">
            <span>問題数</span>
            <input
              type="number"
              min={1}
              step={1}
              inputMode="numeric"
              value={form.questionNumber}
              required
              onChange={(e) => update({ questionNumber: e.target.value })}
            />
          </label>
          <label className="field">
            <span>カテゴリ</span>
            <select value={form.area} required onChange={(e) => update({ area: e.target.value })}>
              {categories.map((c) => (
                <option key={c.id} value={c.name}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
          <label className="field span-2">
            <span>
              回答
              <span className="muted small">
                {' '}
                {form.response.length}/{RESPONSE_MAX}文字・Ctrl+Enter で記録
              </span>
            </span>
            <textarea
              ref={responseRef}
              rows={3}
              value={form.response}
              maxLength={RESPONSE_MAX}
              autoFocus
              onKeyDown={onResponseKey}
              onChange={(e) => update({ response: e.target.value })}
            />
          </label>
          <div className="timer span-2" aria-live="off">
            <span className="muted small">回答時間</span>
            <span className={`timer-value${timer.paused ? ' is-paused' : ''}`}>{formatSeconds(timer.seconds)}</span>
            <button type="button" className="button small" onClick={timer.togglePause}>
              {timer.paused ? '再開' : '一時停止'}
            </button>
          </div>
          <div className="form-actions span-2">
            <button type="submit" className="button primary" disabled={busy || categories.length === 0}>
              記録する
            </button>
            <button type="button" className="button" onClick={goGrade} disabled={busy}>
              採点へ
            </button>
          </div>
        </form>
      </section>
      <RecordedList practice={practice} categories={categories} onUpdated={onUpdated} />
    </>
  )
}

/** この学習で記録した回答の一覧。未採点の回答は修正・削除できる。 */
function RecordedList({
  practice,
  categories,
  onUpdated,
}: {
  practice: Practice
  categories: Category[]
  onUpdated: (practice: Practice) => void
}) {
  const [editingId, setEditingId] = useState<number | null>(null)
  const [error, setError] = useState<string | null>(null)
  if (practice.answers.length === 0) return null

  const remove = async (answer: Answer) => {
    if (!window.confirm(`問題数 ${answer.questionNumber} の回答を削除します。よろしいですか？`)) return
    setError(null)
    try {
      onUpdated(await practiceApi.deleteAnswer(practice.id, answer.id))
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <section className="card">
      <h2>記録した回答</h2>
      <Notice>{error}</Notice>
      <table className="table">
        <thead>
          <tr>
            <th>問題名称</th>
            <th className="num">問題数</th>
            <th>カテゴリ</th>
            <th>回答</th>
            <th className="num">時間</th>
            <th>正誤</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {[...practice.answers].reverse().map((answer) =>
            editingId === answer.id ? (
              <tr key={answer.id}>
                <td colSpan={7}>
                  <EditAnswerForm
                    practiceId={practice.id}
                    answer={answer}
                    categories={categories}
                    onCancel={() => setEditingId(null)}
                    onSaved={(updated) => {
                      setEditingId(null)
                      onUpdated(updated)
                    }}
                  />
                </td>
              </tr>
            ) : (
              <tr key={answer.id}>
                <td>{answer.title}</td>
                <td className="num">{answer.questionNumber}</td>
                <td>{answer.area}</td>
                <td>
                  <MultilineText text={answer.response} />
                </td>
                <td className="num">{formatSeconds(answer.elapsedSeconds)}</td>
                <td>
                  <ResultBadge correct={answer.correct} />
                </td>
                <td className="actions">
                  {answer.correct === null && (
                    <>
                      <button type="button" className="button small" onClick={() => setEditingId(answer.id)}>
                        修正
                      </button>
                      <button type="button" className="button small danger" onClick={() => remove(answer)}>
                        削除
                      </button>
                    </>
                  )}
                </td>
              </tr>
            ),
          )}
        </tbody>
      </table>
    </section>
  )
}

/** 未採点の回答の修正フォーム。 */
function EditAnswerForm({
  practiceId,
  answer,
  categories,
  onCancel,
  onSaved,
}: {
  practiceId: number
  answer: Answer
  categories: Category[]
  onCancel: () => void
  onSaved: (practice: Practice) => void
}) {
  const [form, setForm] = useState<AnswerForm>({
    title: answer.title,
    questionNumber: String(answer.questionNumber),
    area: answer.area,
    response: answer.response,
  })
  const [error, setError] = useState<string | null>(null)
  const update = (patch: Partial<AnswerForm>) => setForm((f) => ({ ...f, ...patch }))

  const save = async (e: FormEvent) => {
    e.preventDefault()
    const input = toInput(form, answer.elapsedSeconds)
    if (typeof input === 'string') return setError(input)
    try {
      onSaved(await practiceApi.updateAnswer(practiceId, answer.id, input))
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  return (
    <form className="answer-form compact" onSubmit={save}>
      <Notice>{error}</Notice>
      <label className="field span-2">
        <span>問題名称</span>
        <input value={form.title} maxLength={100} onChange={(e) => update({ title: e.target.value })} />
      </label>
      <label className="field">
        <span>問題数</span>
        <input type="number" min={1} value={form.questionNumber} onChange={(e) => update({ questionNumber: e.target.value })} />
      </label>
      <label className="field">
        <span>カテゴリ</span>
        <select value={form.area} onChange={(e) => update({ area: e.target.value })}>
          {categories.map((c) => (
            <option key={c.id} value={c.name}>
              {c.name}
            </option>
          ))}
        </select>
      </label>
      <label className="field span-2">
        <span>回答</span>
        <textarea rows={2} maxLength={RESPONSE_MAX} value={form.response} onChange={(e) => update({ response: e.target.value })} />
      </label>
      <div className="form-actions span-2">
        <button type="submit" className="button primary small">
          保存
        </button>
        <button type="button" className="button small" onClick={onCancel}>
          取消
        </button>
      </div>
    </form>
  )
}

/** 自己採点（F-05, F-06）。 */
function GradeView({
  practice,
  onUpdated,
  onBack,
}: {
  practice: Practice
  onUpdated: (practice: Practice) => void
  onBack: () => void
}) {
  const target = practice.answers.find((a) => a.correct === null) ?? null
  const [correctAnswer, setCorrectAnswer] = useState('')
  const [note, setNote] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  if (!target) {
    return (
      <section className="card">
        <Notice kind="info">未採点の回答はありません。</Notice>
        <button type="button" className="button" onClick={onBack}>
          回答の記録に戻る
        </button>
      </section>
    )
  }

  const grade = async (correct: boolean) => {
    setBusy(true)
    setError(null)
    try {
      onUpdated(await practiceApi.grade(practice.id, target.id, { correct, correctAnswer, note }))
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="card grade-card">
      <Notice>{error}</Notice>
      <dl className="details">
        <dt>問題名称</dt>
        <dd>{target.title}</dd>
        <dt>問題数</dt>
        <dd>{target.questionNumber}</dd>
        <dt>カテゴリ</dt>
        <dd>{target.area}</dd>
        <dt>回答時間</dt>
        <dd>{formatSeconds(target.elapsedSeconds)}</dd>
      </dl>
      <div className="grade-response">
        <span className="muted small">記録した回答</span>
        <MultilineText text={target.response} className="response-text" />
      </div>
      <details className="memo">
        <summary>正解・メモを残す（任意）</summary>
        <div className="stack">
          <label className="field">
            <span>正解</span>
            <textarea rows={2} maxLength={RESPONSE_MAX} value={correctAnswer} onChange={(e) => setCorrectAnswer(e.target.value)} />
          </label>
          <label className="field">
            <span>メモ・解説（{NOTE_MAX}文字以内）</span>
            <textarea rows={3} maxLength={NOTE_MAX} value={note} onChange={(e) => setNote(e.target.value)} />
          </label>
        </div>
      </details>
      <div className="grade-buttons">
        <button type="button" className="button good large" disabled={busy} onClick={() => grade(true)}>
          ○ 正解
        </button>
        <button type="button" className="button bad large" disabled={busy} onClick={() => grade(false)}>
          × 不正解
        </button>
      </div>
      <button type="button" className="link-button" onClick={onBack}>
        回答の記録に戻る（回答を追加する）
      </button>
    </section>
  )
}
