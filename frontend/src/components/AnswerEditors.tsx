/**
 * 採点済みの回答をあとから直すための部品（F-17 カテゴリ変更、正解・メモの編集）。
 */

import { useState, type FormEvent } from 'react'

import { errorMessage } from '../api/client'
import { reviewApi } from '../api/endpoints'
import type { Category } from '../api/types'
import { MultilineText } from './ui'

/** 回答のカテゴリを選んで即時に変更するセレクトボックス（F-17）。 */
export function CategorySelect({
  answerId,
  area,
  categories,
  onChanged,
  onError,
}: {
  answerId: number
  area: string
  categories: Category[]
  onChanged: (area: string) => void
  onError: (message: string) => void
}) {
  const [busy, setBusy] = useState(false)
  const inMaster = categories.some((c) => c.name === area)

  const change = async (categoryId: number) => {
    const category = categories.find((c) => c.id === categoryId)
    if (!category) return
    setBusy(true)
    try {
      await reviewApi.changeCategory(answerId, categoryId)
      onChanged(category.name)
    } catch (err) {
      onError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <select
      aria-label="カテゴリ"
      value={categories.find((c) => c.name === area)?.id ?? ''}
      disabled={busy}
      onChange={(e) => change(Number(e.target.value))}
    >
      {!inMaster && <option value="">{area}</option>}
      {categories.map((c) => (
        <option key={c.id} value={c.id}>
          {c.name}
        </option>
      ))}
    </select>
  )
}

/** 正解・メモの表示と編集。 */
export function MemoEditor({
  answerId,
  correctAnswer,
  note,
  onSaved,
  onError,
}: {
  answerId: number
  correctAnswer: string | null
  note: string | null
  onSaved: (correctAnswer: string | null, note: string | null) => void
  onError: (message: string) => void
}) {
  const [editing, setEditing] = useState(false)
  const [answerValue, setAnswerValue] = useState(correctAnswer ?? '')
  const [noteValue, setNoteValue] = useState(note ?? '')

  if (!editing) {
    return (
      <div className="memo-view">
        {correctAnswer && (
          <div>
            <span className="muted small">正解</span> <MultilineText text={correctAnswer} />
          </div>
        )}
        {note && (
          <div>
            <span className="muted small">メモ</span> <MultilineText text={note} />
          </div>
        )}
        <button
          type="button"
          className="link-button small"
          onClick={() => {
            setAnswerValue(correctAnswer ?? '')
            setNoteValue(note ?? '')
            setEditing(true)
          }}
        >
          {correctAnswer || note ? '編集' : '正解・メモを追加'}
        </button>
      </div>
    )
  }

  const save = async (e: FormEvent) => {
    e.preventDefault()
    try {
      await reviewApi.updateMemo(answerId, answerValue, noteValue)
      onSaved(answerValue.trim() ? answerValue : null, noteValue.trim() ? noteValue : null)
      setEditing(false)
    } catch (err) {
      onError(errorMessage(err))
    }
  }

  return (
    <form className="stack memo-form" onSubmit={save}>
      <textarea aria-label="正解" placeholder="正解" rows={2} maxLength={255} value={answerValue} onChange={(e) => setAnswerValue(e.target.value)} />
      <textarea aria-label="メモ" placeholder="メモ・解説" rows={3} maxLength={1000} value={noteValue} onChange={(e) => setNoteValue(e.target.value)} />
      <div className="form-actions">
        <button type="submit" className="button primary small">
          保存
        </button>
        <button type="button" className="button small" onClick={() => setEditing(false)}>
          取消
        </button>
      </div>
    </form>
  )
}
