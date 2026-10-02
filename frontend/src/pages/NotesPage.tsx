/**
 * 見直しノート：採点時に残した「正解」「メモ・解説」を横断して読み返す画面。
 *
 * 試験の直前に、間違えた問題の要点だけをまとめて見直すために使う。
 * 文字（問題名称・回答・正解・メモ）・カテゴリ・正誤で絞り込める。メモの編集は正誤履歴の画面で行う。
 */

import { useMemo, useState } from 'react'
import { Link, useLocation } from 'react-router-dom'

import { examApi, reviewApi } from '../api/endpoints'
import type { NoteItem } from '../api/types'
import { Empty, Loading, MultilineText, Notice, PageHeader, ResultBadge } from '../components/ui'
import { useExamId } from '../hooks/useExamId'
import { useLoad } from '../hooks/useLoad'
import { formatDateTime } from '../utils/format'

/** 検索の対象にする項目をつなげた文字列（大文字・小文字を区別しない）。 */
function searchText(item: NoteItem): string {
  return [item.title, item.response, item.correctAnswer ?? '', item.note ?? '', `問${item.questionNumber}`]
    .join('\n')
    .toLowerCase()
}

/** 見直しノートの画面。 */
export function NotesPage() {
  const examId = useExamId()
  const location = useLocation()
  const notes = useLoad(() => reviewApi.notes(examId), [examId])
  const categories = useLoad(() => examApi.categories(examId), [examId])
  const [keyword, setKeyword] = useState('')
  const [area, setArea] = useState('')
  const [wrongOnly, setWrongOnly] = useState(true)

  const filtered = useMemo(() => {
    const words = keyword.trim().toLowerCase().split(/\s+/).filter(Boolean)
    return (notes.data ?? []).filter(
      (item) =>
        (!wrongOnly || !item.correct) &&
        (!area || item.area === area) &&
        words.every((word) => searchText(item).includes(word)),
    )
  }, [notes.data, keyword, area, wrongOnly])

  return (
    <div>
      <PageHeader
        title="見直しノート"
        sub="採点のときに残した正解とメモを、まとめて読み返せます。試験の直前の見直しに使ってください。"
      />
      <section className="card notes-filter">
        <label className="field notes-search">
          <span>キーワード（スペース区切りで絞り込み）</span>
          <input
            type="search"
            value={keyword}
            placeholder="例：境界値 ループ"
            onChange={(e) => setKeyword(e.target.value)}
          />
        </label>
        <label className="field">
          <span>カテゴリ</span>
          <select value={area} onChange={(e) => setArea(e.target.value)}>
            <option value="">すべて</option>
            {categories.data?.map((c) => (
              <option key={c.id} value={c.name}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <label className="check">
          <input type="checkbox" checked={wrongOnly} onChange={(e) => setWrongOnly(e.target.checked)} />
          不正解だった回答だけ
        </label>
      </section>

      {notes.loading && !notes.data ? (
        <Loading />
      ) : notes.error ? (
        <Notice>{notes.error}</Notice>
      ) : !notes.data?.length ? (
        <Empty>まだメモがありません。採点のときに「正解・メモを残す」に書くと、ここに集まります。</Empty>
      ) : filtered.length === 0 ? (
        <Empty>条件に合うメモがありません。</Empty>
      ) : (
        <>
          <p className="muted small">{filtered.length} 件</p>
          <ul className="note-list">
            {filtered.map((item) => {
              const historyUrl = `/exams/${examId}/history?title=${encodeURIComponent(item.title)}&questionNumber=${item.questionNumber}`
              const retryUrl = `/exams/${examId}/start?title=${encodeURIComponent(item.title)}&number=${item.questionNumber}&area=${encodeURIComponent(item.area)}`
              return (
                <li key={item.answerId} className="note card">
                  <div className="note-head">
                    <div>
                      <strong>
                        {item.title} 問{item.questionNumber}
                      </strong>
                      <span className="muted small">
                        {' '}
                        {item.area}・{formatDateTime(item.answeredAt)}
                      </span>
                    </div>
                    <ResultBadge correct={item.correct} />
                  </div>
                  <dl className="note-body">
                    <div>
                      <dt>自分の回答</dt>
                      <dd>
                        <MultilineText text={item.response} />
                      </dd>
                    </div>
                    <div>
                      <dt>正解</dt>
                      <dd>
                        <MultilineText text={item.correctAnswer} />
                      </dd>
                    </div>
                    <div className="note-memo">
                      <dt>メモ・解説</dt>
                      <dd>
                        <MultilineText text={item.note} />
                      </dd>
                    </div>
                  </dl>
                  <div className="note-actions">
                    <Link className="button small" to={historyUrl} state={{ back: location.pathname }}>
                      正誤履歴・メモの編集
                    </Link>
                    <Link className="button small primary" to={retryUrl}>
                      解き直す
                    </Link>
                  </div>
                </li>
              )
            })}
          </ul>
        </>
      )}
    </div>
  )
}
