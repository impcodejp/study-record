/**
 * 振り返りの画面（F-07 採点結果、F-11〜F-16）。
 */

import { useMemo, useState } from 'react'
import { Link, useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { examApi, practiceApi, reviewApi, type QuestionFilter } from '../api/endpoints'
import type { Answer, Category, Practice, QuestionItem } from '../api/types'
import { CategorySelect, MemoEditor } from '../components/AnswerEditors'
import { MasteryMeter } from '../components/StudyPlan'
import { Empty, Loading, MultilineText, Notice, PageHeader, ResultBadge } from '../components/ui'
import { useApp } from '../hooks/useApp'
import { useExamId } from '../hooks/useExamId'
import { useLoad } from '../hooks/useLoad'
import { formatDateTime, formatRate, formatSeconds, todayString } from '../utils/format'
import { QUESTION_SORTS, reviewDueText, sortQuestions, type QuestionSort } from '../utils/studyPlan'
import { REVIEW_QUEUE_MAX, saveReviewQueue } from '../utils/reviewQueue'
import { saveSessionStart } from '../utils/session'

/** 採点済み回答であとから変更できる項目。 */
type AnswerPatch = Partial<Pick<Answer, 'area' | 'correctAnswer' | 'note'>>

/** 問題名称ごとの集計（採点結果の表示用）。 */
function groupByTitle(answers: Answer[]): { title: string; total: number; correct: number }[] {
  const groups = new Map<string, { title: string; total: number; correct: number }>()
  for (const a of answers) {
    if (a.correct === null) continue
    const group = groups.get(a.title) ?? { title: a.title, total: 0, correct: 0 }
    group.total += 1
    if (a.correct) group.correct += 1
    groups.set(a.title, group)
  }
  return [...groups.values()]
}

/** 採点結果の画面（F-07）。 */
export function ResultPage() {
  const { practiceId } = useParams()
  const practice = useLoad(() => practiceApi.get(Number(practiceId)), [practiceId])
  if (practice.loading && !practice.data) return <Loading />
  if (practice.error) return <Notice>{practice.error}</Notice>
  const p = practice.data
  if (!p) return null
  const groups = groupByTitle(p.answers)

  return (
    <div className="narrow">
      <PageHeader title="採点結果" sub={`${p.examName}・${formatDateTime(p.createdAt)} 開始`} />
      <section className="card result-summary">
        <div className="stat hero">
          <span className="stat-label">正答率</span>
          <span className="stat-value">{formatRate(p.correctCount, p.gradedCount)}</span>
        </div>
        <div className="stat">
          <span className="stat-label">正解数／問題数</span>
          <span className="stat-value">
            {p.correctCount}
            <small> / {p.gradedCount}</small>
          </span>
        </div>
      </section>
      <section className="card">
        <h2>問題名称別の結果</h2>
        <table className="table">
          <thead>
            <tr>
              <th>問題名称</th>
              <th className="num">正解数／問題数</th>
              <th className="num">正答率</th>
            </tr>
          </thead>
          <tbody>
            {groups.map((g) => (
              <tr key={g.title}>
                <td>
                  <Link to={`/exams/${p.examId}/practices/${p.id}?title=${encodeURIComponent(g.title)}`}>{g.title}</Link>
                </td>
                <td className="num">
                  {g.correct} / {g.total}
                </td>
                <td className="num">{formatRate(g.correct, g.total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <div className="form-actions">
        <Link className="button primary" to={`/exams/${p.examId}/start`}>
          続けて新しい学習を始める
        </Link>
        <Link className="button" to={`/exams/${p.examId}`}>
          ダッシュボードへ
        </Link>
      </div>
    </div>
  )
}

/** 学習履歴一覧（F-11）。 */
export function PracticeListPage() {
  const examId = useExamId()
  const list = useLoad(() => practiceApi.list(examId), [examId])
  return (
    <div>
      <PageHeader title="学習履歴" sub="学習ごと・問題名称ごとの結果です。" />
      {list.loading && !list.data ? (
        <Loading />
      ) : list.error ? (
        <Notice>{list.error}</Notice>
      ) : !list.data?.length ? (
        <Empty>まだ学習の記録がありません。</Empty>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>問題名称</th>
              <th>開始日時</th>
              <th className="num">問題数</th>
              <th className="num">正解数</th>
              <th className="num">正答率</th>
            </tr>
          </thead>
          <tbody>
            {list.data.map((row) => (
              <tr key={`${row.id}-${row.title}`}>
                <td>
                  <Link to={`/exams/${examId}/practices/${row.id}?title=${encodeURIComponent(row.title)}`}>{row.title}</Link>
                </td>
                <td>{formatDateTime(row.createdAt)}</td>
                <td className="num">{row.questionCount}</td>
                <td className="num">{row.correctCount}</td>
                <td className="num">{formatRate(row.correctCount, row.questionCount)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}

/** 学習履歴の詳細（F-12, F-17）。 */
export function PracticeDetailPage() {
  const examId = useExamId()
  const { practiceId } = useParams()
  const [params] = useSearchParams()
  const title = params.get('title')
  const practice = useLoad(() => practiceApi.get(Number(practiceId)), [practiceId])
  const categories = useLoad(() => examApi.categories(examId), [examId])
  const [error, setError] = useState<string | null>(null)

  const p = practice.data
  const answers = useMemo(
    () => (p ? p.answers.filter((a) => a.correct !== null && (title === null || a.title === title)) : []),
    [p, title],
  )
  if (practice.loading && !p) return <Loading />
  if (practice.error) return <Notice>{practice.error}</Notice>
  if (!p) return null
  const correct = answers.filter((a) => a.correct).length

  /** 1 件の回答を書き換えた学習で画面を更新する。 */
  const patchAnswer = (id: number, patch: AnswerPatch) => {
    const updated: Practice = { ...p, answers: p.answers.map((a) => (a.id === id ? { ...a, ...patch } : a)) }
    practice.setData(updated)
  }

  return (
    <div>
      <PageHeader
        title={title ?? p.title}
        sub={`${formatDateTime(p.createdAt)} 開始・正解 ${correct} / ${answers.length}（${formatRate(correct, answers.length)}）`}
        actions={
          <Link className="button" to={`/exams/${examId}/practices`}>
            学習履歴へ戻る
          </Link>
        }
      />
      <Notice>{error}</Notice>
      <AnswerTable
        rows={answers.map((a) => ({ ...a, key: a.id, label: String(a.questionNumber) }))}
        firstColumn="問題数"
        categories={categories.data ?? []}
        onPatch={patchAnswer}
        onError={setError}
      />
    </div>
  )
}

/** 回答の表の 1 行。 */
interface AnswerRow {
  key: number
  id: number
  label: string
  area: string
  response: string
  elapsedSeconds: number
  correct: boolean | null
  correctAnswer: string | null
  note: string | null
}

/** 採点済み回答の表（学習履歴の詳細・正誤履歴で共通）。 */
function AnswerTable({
  rows,
  firstColumn,
  categories,
  onPatch,
  onError,
}: {
  rows: AnswerRow[]
  firstColumn: string
  categories: Category[]
  onPatch: (id: number, patch: AnswerPatch) => void
  onError: (message: string) => void
}) {
  if (rows.length === 0) return <Empty>採点済みの回答がありません。</Empty>
  return (
    <table className="table">
      <thead>
        <tr>
          <th>{firstColumn}</th>
          <th className="num">回答時間</th>
          <th>カテゴリ</th>
          <th>回答</th>
          <th>正誤</th>
          <th>正解・メモ</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.key}>
            <td>{row.label}</td>
            <td className="num">{formatSeconds(row.elapsedSeconds)}</td>
            <td>
              <CategorySelect
                answerId={row.id}
                area={row.area}
                categories={categories}
                onChanged={(area) => onPatch(row.id, { area })}
                onError={onError}
              />
            </td>
            <td>
              <MultilineText text={row.response} />
            </td>
            <td>
              <ResultBadge correct={row.correct} />
            </td>
            <td>
              <MemoEditor
                answerId={row.id}
                correctAnswer={row.correctAnswer}
                note={row.note}
                onSaved={(correctAnswer, note) => onPatch(row.id, { correctAnswer, note })}
                onError={onError}
              />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

/** 正誤履歴の画面の URL。 */
function historyUrl(examId: number, q: QuestionItem): string {
  return `/exams/${examId}/history?title=${encodeURIComponent(q.title)}&questionNumber=${q.questionNumber}`
}

/** 問題を解き直す（問題名称・問題数・カテゴリを入れた状態で学習を始める）画面の URL。 */
function retryUrl(examId: number, q: QuestionItem): string {
  return `/exams/${examId}/start?title=${encodeURIComponent(q.title)}&number=${q.questionNumber}&area=${encodeURIComponent(q.area)}`
}

/**
 * 問題一覧・復習・カテゴリ別の履歴（F-13〜F-15、今日の復習）。
 *
 * 復習画面（`review`）には 2 つの表示がある。
 * - 今日の復習（既定）：忘却曲線にもとづく復習日が来た問題。復習日を過ぎた日数が長い順
 * - 最新が不正解（`?view=wrong`）：元のシステムの復習一覧（F-14）
 */
export function QuestionsPage({ review = false }: { review?: boolean }) {
  const examId = useExamId()
  const navigate = useNavigate()
  const location = useLocation()
  const [params] = useSearchParams()
  const categoryId = Number(params.get('categoryId')) || null
  const wrongOnly = review && params.get('view') === 'wrong'
  const filter: QuestionFilter = review
    ? wrongOnly
      ? { kind: 'review' }
      : { kind: 'due' }
    : categoryId
      ? { kind: 'category', categoryId }
      : { kind: 'all' }
  const questions = useLoad(() => reviewApi.questions(examId, filter), [examId, review, wrongOnly, categoryId])
  const categories = useLoad(() => examApi.categories(examId), [examId])
  const categoryName = categories.data?.find((c) => c.id === categoryId)?.name
  const today = todayString()
  const { setActivePracticeId } = useApp()
  const [sort, setSort] = useState<QuestionSort>('default')
  const [startError, setStartError] = useState<string | null>(null)
  const [starting, setStarting] = useState(false)
  const sorted = useMemo(() => sortQuestions(questions.data ?? [], sort), [questions.data, sort])

  /** 今日の復習の問題を 1 回の学習でまとめて解く（回答画面で 1 問ずつ自動で入力する）。 */
  const startQueue = async () => {
    // 画面に表示している順（並べ替えを反映）で、上から順に解く。
    const items = sorted.slice(0, REVIEW_QUEUE_MAX).map((q) => ({
      title: q.title,
      questionNumber: q.questionNumber,
      area: q.area,
    }))
    if (items.length === 0) return
    setStarting(true)
    setStartError(null)
    try {
      const practice = await practiceApi.start(examId, `今日の復習 ${today}`)
      saveSessionStart(practice.id, items[0].questionNumber, items[0].area)
      saveReviewQueue(practice.id, items)
      setActivePracticeId(practice.id)
      navigate('/session')
    } catch (err) {
      setStartError(errorMessage(err))
    } finally {
      setStarting(false)
    }
  }

  const title = review ? '復習' : categoryId ? `カテゴリ別の履歴：${categoryName ?? ''}` : '問題一覧'
  const sub = review
    ? wrongOnly
      ? '最新の結果が不正解の問題です。正解すると一覧から外れます。'
      : '忘れかけた頃に解き直すと記憶に残ります。正解するたびに次の復習までの間隔が延びます（1→3→7→14→30→60日）。'
    : '問題ごとの最新の結果です。選ぶと正誤履歴を表示します。'
  const emptyText = review
    ? wrongOnly
      ? '最新の結果が不正解の問題はありません。'
      : '今日復習する問題はありません。新しい問題に進みましょう。'
    : '該当する問題がありません。'

  return (
    <div>
      <PageHeader
        title={title}
        sub={sub}
        actions={
          !review && (
            <label className="inline-select">
              <span>カテゴリ</span>
              <select
                value={categoryId ?? ''}
                onChange={(e) =>
                  navigate(`/exams/${examId}/questions${e.target.value ? `?categoryId=${e.target.value}` : ''}`)
                }
              >
                <option value="">すべて</option>
                {categories.data?.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </label>
          )
        }
      />
      {review && (
        <nav className="tabs" aria-label="復習の表示">
          <Link className={`tab${wrongOnly ? '' : ' is-active'}`} to={`/exams/${examId}/review`} aria-current={wrongOnly ? undefined : 'page'}>
            今日の復習
          </Link>
          <Link
            className={`tab${wrongOnly ? ' is-active' : ''}`}
            to={`/exams/${examId}/review?view=wrong`}
            aria-current={wrongOnly ? 'page' : undefined}
          >
            最新が不正解
          </Link>
        </nav>
      )}
      {review && !wrongOnly && !!questions.data?.length && (
        <section className="queue-start card">
          <div>
            <strong>今日の復習 {questions.data.length} 問</strong>
            <p className="muted small">
              {questions.data.length > REVIEW_QUEUE_MAX
                ? `上から ${REVIEW_QUEUE_MAX} 問を、1 回の学習で順番に解きます。`
                : 'すべての問題を、1 回の学習で順番に解きます。'}
              問題名称・問題数・カテゴリは自動で入ります。
            </p>
          </div>
          <button type="button" className="button primary" onClick={startQueue} disabled={starting}>
            まとめて復習する
          </button>
          {startError && (
            <div className="queue-start-error">
              <Notice>{startError}</Notice>
            </div>
          )}
        </section>
      )}
      {questions.loading && !questions.data ? (
        <Loading />
      ) : questions.error ? (
        <Notice>{questions.error}</Notice>
      ) : !questions.data?.length ? (
        <Empty>{emptyText}</Empty>
      ) : (
        <>
        <div className="table-tools">
          <label className="inline-select">
            <span>並べ替え</span>
            <select value={sort} onChange={(e) => setSort(e.target.value as QuestionSort)}>
              {QUESTION_SORTS.map((s) => (
                <option key={s.value} value={s.value}>
                  {review && !wrongOnly && s.value === 'default' ? '復習を過ぎた日数が長い順' : s.label}
                </option>
              ))}
            </select>
          </label>
          <span className="muted small">{questions.data.length} 問</span>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>問題名称</th>
              <th className="num">問題数</th>
              <th>カテゴリ</th>
              <th>最終回答日時</th>
              <th className="num">回答回数</th>
              <th className="num">正答率</th>
              <th className="num">平均時間</th>
              <th>最新の正誤</th>
              <th>習熟度</th>
              <th>次の復習</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {sorted.map((q) => (
              <tr
                key={`${q.title}-${q.questionNumber}`}
                className="clickable"
                onClick={() => navigate(historyUrl(examId, q), { state: { back: location.pathname + location.search } })}
              >
                <td>
                  <Link
                    to={historyUrl(examId, q)}
                    state={{ back: location.pathname + location.search }}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {q.title}
                  </Link>
                </td>
                <td className="num">{q.questionNumber}</td>
                <td>{q.area}</td>
                <td>{formatDateTime(q.answeredAt)}</td>
                <td className="num">{q.attemptCount}</td>
                <td className="num">{formatRate(q.correctCount, q.attemptCount)}</td>
                <td className="num nowrap">{formatSeconds(q.averageSeconds)}</td>
                <td>
                  <ResultBadge correct={q.correct} />
                </td>
                <td>
                  <MasteryMeter level={q.masteryLevel} />
                </td>
                <td className={`nowrap ${q.nextReviewOn < today ? 'bad-text' : q.nextReviewOn === today ? 'accent-text' : ''}`}>
                  {reviewDueText(q.nextReviewOn, today)}
                </td>
                <td className="actions">
                  <Link className="button small" to={retryUrl(examId, q)} onClick={(e) => e.stopPropagation()}>
                    解き直す
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        </>
      )}
    </div>
  )
}

/** 問題ごとの正誤履歴（F-16, F-17）。 */
export function HistoryPage() {
  const examId = useExamId()
  const location = useLocation()
  const [params] = useSearchParams()
  const title = params.get('title') ?? ''
  const questionNumber = Number(params.get('questionNumber'))
  const history = useLoad(() => reviewApi.history(examId, title, questionNumber), [examId, title, questionNumber])
  const categories = useLoad(() => examApi.categories(examId), [examId])
  const [error, setError] = useState<string | null>(null)
  const back = (location.state as { back?: string } | null)?.back ?? `/exams/${examId}/questions`

  const patch = (id: number, change: AnswerPatch) => {
    if (!history.data) return
    history.setData(history.data.map((a) => (a.id === id ? { ...a, ...change } : a)))
  }

  return (
    <div>
      <PageHeader
        title={`${title} 問${questionNumber}`}
        sub="この問題の過去の回答（新しい順）"
        actions={
          <Link className="button" to={back}>
            一覧へ戻る
          </Link>
        }
      />
      <Notice>{error}</Notice>
      {history.loading && !history.data ? (
        <Loading />
      ) : history.error ? (
        <Notice>{history.error}</Notice>
      ) : (
        <AnswerTable
          rows={(history.data ?? []).map((a) => ({ ...a, key: a.id, label: formatDateTime(a.answeredAt) }))}
          firstColumn="回答日時"
          categories={categories.data ?? []}
          onPatch={patch}
          onError={setError}
        />
      )}
    </div>
  )
}
