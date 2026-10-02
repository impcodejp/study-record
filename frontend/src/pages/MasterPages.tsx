/**
 * マスタ管理の画面（試験マスタ、カテゴリマスタ F-18〜F-21）と、トップの振り分け。
 */

import { useState, type FormEvent } from 'react'
import { Link, Navigate } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { examApi } from '../api/endpoints'
import type { Category, Exam } from '../api/types'
import { Empty, Loading, Notice, PageHeader } from '../components/ui'
import { useExamId } from '../hooks/useExamId'
import { useLoad } from '../hooks/useLoad'
import { formatDateTime } from '../utils/format'
import { loadLastExamId } from '../utils/storage'

/** トップ（/）：最後に開いた試験、なければ最初の試験、試験が無ければ試験の管理へ。 */
export function HomeRedirect() {
  const exams = useLoad(() => examApi.list(), [])
  if (exams.loading) return <Loading />
  if (exams.error) return <Notice>{exams.error}</Notice>
  const list = exams.data ?? []
  // 試験が無い（使い始めたばかりの）利用者は、はじめての設定へ案内する。
  if (list.length === 0) return <Navigate to="/setup" replace />
  const last = loadLastExamId()
  const target = list.find((e) => e.id === last) ?? list[0]
  return <Navigate to={`/exams/${target.id}`} replace />
}

/** 名前の追加フォーム（試験・カテゴリ共通）。 */
function AddForm({
  label,
  maxLength,
  onAdd,
}: {
  label: string
  maxLength: number
  onAdd: (name: string) => Promise<void>
}) {
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const submit = async (e: FormEvent) => {
    e.preventDefault()
    setBusy(true)
    try {
      await onAdd(name)
      setName('')
    } catch {
      // エラーは親の画面が表示する。入力内容は残して再入力できるようにする。
    } finally {
      setBusy(false)
    }
  }
  return (
    <form className="inline-form" onSubmit={submit}>
      <input
        aria-label={label}
        placeholder={label}
        value={name}
        maxLength={maxLength}
        required
        onChange={(e) => setName(e.target.value)}
      />
      <button type="submit" className="button primary" disabled={busy}>
        追加
      </button>
    </form>
  )
}

/** 名前をその場で編集する行の部品。 */
function EditableName({
  name,
  maxLength,
  onSave,
}: {
  name: string
  maxLength: number
  onSave: (name: string) => Promise<void>
}) {
  const [editing, setEditing] = useState(false)
  const [value, setValue] = useState(name)
  if (!editing) {
    return (
      <span className="editable">
        <span>{name}</span>
        <button
          type="button"
          className="link-button"
          onClick={() => {
            setValue(name)
            setEditing(true)
          }}
        >
          名称変更
        </button>
      </span>
    )
  }
  const save = async (e: FormEvent) => {
    e.preventDefault()
    try {
      await onSave(value)
      setEditing(false)
    } catch {
      // エラーは親の画面が表示する。編集中のまま残す。
    }
  }
  return (
    <form className="inline-form" onSubmit={save}>
      <input aria-label="新しい名称" value={value} maxLength={maxLength} required autoFocus onChange={(e) => setValue(e.target.value)} />
      <button type="submit" className="button primary small">
        保存
      </button>
      <button type="button" className="button small" onClick={() => setEditing(false)}>
        取消
      </button>
    </form>
  )
}

/** 試験の管理画面。 */
export function ExamsPage() {
  const exams = useLoad(() => examApi.list(), [])
  const [error, setError] = useState<string | null>(null)

  /** 更新系の操作を実行し、結果の一覧で画面を差し替える。失敗時はエラーを表示して投げ直す。 */
  const apply = async (action: () => Promise<Exam[]>) => {
    setError(null)
    try {
      exams.setData(await action())
    } catch (err) {
      setError(errorMessage(err))
      throw err
    }
  }

  return (
    <div className="narrow">
      <PageHeader
        title="試験の管理"
        sub="学習する資格試験を登録します。カテゴリ・学習履歴・集計は試験ごとに分かれます。"
        actions={
          <Link className="button primary" to="/setup">
            テンプレートから追加
          </Link>
        }
      />
      <Notice>{error}</Notice>
      <section className="card">
        <AddForm label="試験名（例：基本情報技術者試験 科目B）" maxLength={100} onAdd={(name) => apply(() => examApi.create(name))} />
      </section>
      {exams.loading && !exams.data ? (
        <Loading />
      ) : exams.error ? (
        <Notice>{exams.error}</Notice>
      ) : !exams.data?.length ? (
        <Empty>まだ試験がありません。上の欄から登録してください。</Empty>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>試験名</th>
              <th className="num">学習回数</th>
              <th className="num">解答数</th>
              <th>登録日時</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {exams.data.map((exam) => (
              <tr key={exam.id}>
                <td>
                  <EditableName name={exam.name} maxLength={100} onSave={(name) => apply(() => examApi.rename(exam.id, name))} />
                </td>
                <td className="num">{exam.practiceCount}</td>
                <td className="num">{exam.answerCount}</td>
                <td>{formatDateTime(exam.createdAt)}</td>
                <td className="actions">
                  <Link className="button small primary" to={`/exams/${exam.id}`}>
                    開く
                  </Link>
                  <button
                    type="button"
                    className="button small danger"
                    disabled={exam.practiceCount > 0}
                    title={exam.practiceCount > 0 ? '学習記録がある試験は削除できません' : undefined}
                    onClick={() => {
                      if (window.confirm(`「${exam.name}」を削除します。よろしいですか？`)) {
                        apply(() => examApi.remove(exam.id)).catch(() => undefined)
                      }
                    }}
                  >
                    削除
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}

/** カテゴリ管理画面（F-18〜F-21 と並び替え）。 */
export function CategoriesPage() {
  const examId = useExamId()
  const categories = useLoad(() => examApi.categories(examId), [examId])
  const [error, setError] = useState<string | null>(null)

  /** 更新系の操作を実行し、結果の一覧で画面を差し替える。失敗時はエラーを表示して投げ直す。 */
  const apply = async (action: () => Promise<Category[]>) => {
    setError(null)
    try {
      categories.setData(await action())
    } catch (err) {
      setError(errorMessage(err))
      throw err
    }
  }

  /** index 番目のカテゴリを delta（-1 / +1）だけ移動する。 */
  const move = (index: number, delta: number) => {
    const list = [...(categories.data ?? [])]
    const target = index + delta
    if (target < 0 || target >= list.length) return
    ;[list[index], list[target]] = [list[target], list[index]]
    apply(() => examApi.reorderCategories(examId, list.map((c) => c.id))).catch(() => undefined)
  }

  return (
    <div className="narrow">
      <PageHeader title="カテゴリ" sub="回答を記録するときに選ぶ分類です。名称を変更すると、記録済みの回答にも反映されます。" />
      <Notice>{error}</Notice>
      <section className="card">
        <AddForm label="カテゴリ名（50文字以内）" maxLength={50} onAdd={(name) => apply(() => examApi.addCategory(examId, name))} />
      </section>
      {categories.loading && !categories.data ? (
        <Loading />
      ) : categories.error ? (
        <Notice>{categories.error}</Notice>
      ) : !categories.data?.length ? (
        <Empty>カテゴリがありません。回答を記録するには、1つ以上登録してください。</Empty>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>並び順</th>
              <th>カテゴリ名</th>
              <th className="num">使用回答数</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {categories.data.map((category, index, list) => (
              <tr key={category.id}>
                <td className="order-buttons">
                  <button type="button" className="button small" aria-label="上へ" disabled={index === 0} onClick={() => move(index, -1)}>
                    ↑
                  </button>
                  <button
                    type="button"
                    className="button small"
                    aria-label="下へ"
                    disabled={index === list.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    ↓
                  </button>
                </td>
                <td>
                  <EditableName
                    name={category.name}
                    maxLength={50}
                    onSave={(name) => apply(() => examApi.renameCategory(examId, category.id, name))}
                  />
                </td>
                <td className="num">{category.answerCount}</td>
                <td className="actions">
                  <button
                    type="button"
                    className="button small danger"
                    disabled={category.answerCount > 0}
                    title={category.answerCount > 0 ? '回答で使用中のカテゴリは削除できません' : undefined}
                    onClick={() => {
                      if (window.confirm(`「${category.name}」を削除します。よろしいですか？`)) {
                        apply(() => examApi.removeCategory(examId, category.id)).catch(() => undefined)
                      }
                    }}
                  >
                    削除
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  )
}
