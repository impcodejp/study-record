/**
 * 学習の開始画面（F-01）。
 */

import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { practiceApi } from '../api/endpoints'
import { Notice, PageHeader } from '../components/ui'
import { useApp } from '../hooks/useApp'
import { useExamId } from '../hooks/useExamId'
import { saveSessionStart } from '../utils/session'

/** 学習の開始画面。 */
export function StartPage() {
  const examId = useExamId()
  const navigate = useNavigate()
  const { setActivePracticeId } = useApp()
  const [title, setTitle] = useState('')
  const [firstNumber, setFirstNumber] = useState('1')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    // 最初の問題数は画面側だけでチェックし、サーバーには送らない（F-01）。
    const number = Number(firstNumber)
    if (!title.trim()) return setError('問題名称を入力してください。')
    if (title.trim().length > 100) return setError('問題名称は1～100文字で入力してください。')
    if (!Number.isInteger(number) || number < 1) return setError('問題数は1以上の数字で入力してください。')
    setBusy(true)
    setError(null)
    try {
      const practice = await practiceApi.start(examId, title)
      saveSessionStart(practice.id, number)
      setActivePracticeId(practice.id)
      navigate('/session')
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="narrow">
      <PageHeader title="学習を始める" sub="問題集の名称と、最初に解く問題の番号を入力してください。" />
      <section className="card">
        <Notice>{error}</Notice>
        <form className="stack" onSubmit={submit}>
          <label className="field">
            <span>問題名称（100文字以内）</span>
            <input value={title} maxLength={100} required autoFocus onChange={(e) => setTitle(e.target.value)} />
          </label>
          <label className="field narrow-field">
            <span>最初の問題数</span>
            <input
              type="number"
              min={1}
              step={1}
              inputMode="numeric"
              value={firstNumber}
              required
              onChange={(e) => setFirstNumber(e.target.value)}
            />
          </label>
          <button type="submit" className="button primary" disabled={busy}>
            開始する
          </button>
        </form>
      </section>
    </div>
  )
}
