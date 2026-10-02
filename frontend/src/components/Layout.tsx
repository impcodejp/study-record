/**
 * ログイン後の画面の枠（ヘッダー・試験の切り替え・メニュー）。
 *
 * 学習中（回答・採点の途中）は他の画面へ移動できないよう、メニューを隠す（F-04）。
 */

import { useEffect } from 'react'
import { NavLink, Outlet, useMatch, useNavigate } from 'react-router-dom'

import { examApi } from '../api/endpoints'
import { useApp } from '../hooks/useApp'
import { useLoad } from '../hooks/useLoad'
import { saveLastExamId } from '../utils/storage'

/** 試験ごとのメニュー。 */
const EXAM_MENU = [
  { to: '', label: 'ダッシュボード', end: true },
  { to: 'start', label: '学習を始める' },
  { to: 'practices', label: '学習履歴' },
  { to: 'questions', label: '問題一覧' },
  { to: 'review', label: '復習' },
  { to: 'notes', label: '見直しノート' },
  { to: 'categories', label: 'カテゴリ' },
]

/** ログイン後の画面の枠。 */
export function Layout() {
  const { user, logout, activePracticeId } = useApp()
  const navigate = useNavigate()
  const examMatch = useMatch('/exams/:examId/*')
  const examId = examMatch ? Number(examMatch.params.examId) : null
  const inSession = activePracticeId !== null
  const exams = useLoad(() => examApi.list(), [examId])

  useEffect(() => {
    if (examId) saveLastExamId(examId)
  }, [examId])

  const handleLogout = async () => {
    await logout()
    navigate('/login')
  }

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-inner">
          <span className="brand">学習記録</span>
          {!inSession && exams.data && exams.data.length > 0 && (
            <label className="exam-switch">
              <span className="visually-hidden">試験の切り替え</span>
              <select
                value={examId ?? ''}
                onChange={(e) => e.target.value && navigate(`/exams/${e.target.value}`)}
              >
                {examId === null && <option value="">試験を選択</option>}
                {exams.data.map((exam) => (
                  <option key={exam.id} value={exam.id}>
                    {exam.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          <div className="header-spacer" />
          {!inSession && (
            <nav className="header-links" aria-label="アカウント">
              <NavLink to="/exams">試験の管理</NavLink>
              <NavLink to="/account">{user?.name ?? 'アカウント'}</NavLink>
              <button type="button" className="link-button" onClick={handleLogout}>
                ログアウト
              </button>
            </nav>
          )}
        </div>
        {!inSession && examId && (
          <nav className="exam-menu" aria-label="試験のメニュー">
            <div className="header-inner">
              {EXAM_MENU.map((item) => (
                <NavLink key={item.to} to={`/exams/${examId}${item.to ? `/${item.to}` : ''}`} end={item.end}>
                  {item.label}
                </NavLink>
              ))}
            </div>
          </nav>
        )}
      </header>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  )
}
