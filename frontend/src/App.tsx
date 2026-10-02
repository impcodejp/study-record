/**
 * 画面の構成（URL と画面の対応）。
 *
 * 画面遷移の要点（機能一覧 4 章）：
 * - ダッシュボード → 学習開始 → 回答記録 ⇄ 採点 →（全件採点）→ 採点結果
 * - 問題一覧・復習・カテゴリ別 → 正誤履歴 →（戻る）→ 元の一覧
 * - 学習履歴一覧 → 学習履歴の詳細
 */

import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { GuestOnly, RequireAuth } from './components/Guards'
import { Layout } from './components/Layout'
import { AppProvider } from './hooks/AppProvider'
import {
  AccountPage,
  ForgotPasswordPage,
  LoginPage,
  RegisterPage,
  ResetPasswordPage,
  VerifyEmailPage,
} from './pages/AuthPages'
import { DashboardPage } from './pages/DashboardPage'
import { CategoriesPage, ExamsPage, HomeRedirect } from './pages/MasterPages'
import { LandingPage } from './pages/LandingPage'
import { NotesPage } from './pages/NotesPage'
import { SetupPage } from './pages/SetupPage'
import { HistoryPage, PracticeDetailPage, PracticeListPage, QuestionsPage, ResultPage } from './pages/ReviewPages'
import { SessionPage } from './pages/SessionPage'
import { StartPage } from './pages/StartPage'

/** アプリの本体。 */
export default function App() {
  return (
    <AppProvider>
      <BrowserRouter>
        <Routes>
          {/* ログイン前の画面 */}
          <Route path="/welcome" element={<GuestOnly><LandingPage /></GuestOnly>} />
          <Route path="/login" element={<GuestOnly><LoginPage /></GuestOnly>} />
          <Route path="/register" element={<GuestOnly><RegisterPage /></GuestOnly>} />
          <Route path="/forgot-password" element={<GuestOnly><ForgotPasswordPage /></GuestOnly>} />
          {/* メールのリンクから開く画面（ログイン状態に関係なく表示する） */}
          <Route path="/verify-email" element={<VerifyEmailPage />} />
          <Route path="/reset-password" element={<ResetPasswordPage />} />

          {/* ログイン後の画面 */}
          <Route element={<RequireAuth><Layout /></RequireAuth>}>
            <Route index element={<HomeRedirect />} />
            <Route path="setup" element={<SetupPage />} />
            <Route path="exams" element={<ExamsPage />} />
            <Route path="exams/:examId">
              <Route index element={<DashboardPage />} />
              <Route path="start" element={<StartPage />} />
              <Route path="practices" element={<PracticeListPage />} />
              <Route path="practices/:practiceId" element={<PracticeDetailPage />} />
              <Route path="questions" element={<QuestionsPage />} />
              <Route path="review" element={<QuestionsPage review />} />
              <Route path="history" element={<HistoryPage />} />
              <Route path="notes" element={<NotesPage />} />
              <Route path="categories" element={<CategoriesPage />} />
            </Route>
            <Route path="session" element={<SessionPage />} />
            <Route path="result/:practiceId" element={<ResultPage />} />
            <Route path="account" element={<AccountPage />} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </AppProvider>
  )
}
