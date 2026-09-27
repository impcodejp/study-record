/**
 * API の呼び出し関数。画面からはこのファイルの関数だけを使う。
 *
 * URL とバックエンドの対応は backend/src/presentation/router.rs を参照。
 */

import { query, request } from './client'
import type {
  Attempt,
  Category,
  Dashboard,
  Exam,
  Practice,
  PracticeSummary,
  QuestionItem,
  SessionInfo,
  User,
} from './types'

/** 回答の記録・修正の入力。 */
export interface AnswerInput {
  title: string
  questionNumber: number
  area: string
  response: string
  elapsedSeconds: number
}

/** 採点の入力。 */
export interface GradeInput {
  correct: boolean
  correctAnswer?: string
  note?: string
}

/** 問題一覧の絞り込み。 */
export type QuestionFilter = { kind: 'all' } | { kind: 'review' } | { kind: 'category'; categoryId: number }

/** メール送信系 API の応答。 */
interface MessageResponse {
  message: string
}

/** アカウント関連の API。 */
export const authApi = {
  me: () => request<SessionInfo>('/api/auth/me', { silentUnauthorized: true }),
  login: (email: string, password: string) =>
    request<SessionInfo>('/api/auth/login', {
      method: 'POST',
      body: { email, password },
      silentUnauthorized: true,
    }),
  logout: () => request<void>('/api/auth/logout', { method: 'POST' }),
  register: (name: string, email: string) =>
    request<MessageResponse>('/api/auth/register', { method: 'POST', body: { name, email } }),
  resendVerification: (email: string) =>
    request<MessageResponse>('/api/auth/resend-verification', { method: 'POST', body: { email } }),
  verifyEmail: (token: string, password: string) =>
    request<User>('/api/auth/verify-email', { method: 'POST', body: { token, password } }),
  forgotPassword: (email: string) =>
    request<MessageResponse>('/api/auth/forgot-password', { method: 'POST', body: { email } }),
  resetPassword: (token: string, password: string) =>
    request<void>('/api/auth/reset-password', { method: 'POST', body: { token, password } }),
  changePassword: (currentPassword: string, newPassword: string) =>
    request<void>('/api/auth/password', { method: 'POST', body: { currentPassword, newPassword } }),
}

/** 試験・カテゴリの API。 */
export const examApi = {
  list: () => request<Exam[]>('/api/exams'),
  create: (name: string) => request<Exam[]>('/api/exams', { method: 'POST', body: { name } }),
  rename: (examId: number, name: string) =>
    request<Exam[]>(`/api/exams/${examId}`, { method: 'PUT', body: { name } }),
  remove: (examId: number) => request<Exam[]>(`/api/exams/${examId}`, { method: 'DELETE' }),

  categories: (examId: number) => request<Category[]>(`/api/exams/${examId}/categories`),
  addCategory: (examId: number, name: string) =>
    request<Category[]>(`/api/exams/${examId}/categories`, { method: 'POST', body: { name } }),
  renameCategory: (examId: number, categoryId: number, name: string) =>
    request<Category[]>(`/api/exams/${examId}/categories/${categoryId}`, { method: 'PUT', body: { name } }),
  removeCategory: (examId: number, categoryId: number) =>
    request<Category[]>(`/api/exams/${examId}/categories/${categoryId}`, { method: 'DELETE' }),
  reorderCategories: (examId: number, categoryIds: number[]) =>
    request<Category[]>(`/api/exams/${examId}/category-order`, { method: 'PUT', body: { categoryIds } }),
}

/** 学習・回答・採点の API。 */
export const practiceApi = {
  active: () => request<Practice | null>('/api/active'),
  list: (examId: number) => request<PracticeSummary[]>(`/api/exams/${examId}/practices`),
  start: (examId: number, title: string) =>
    request<Practice>(`/api/exams/${examId}/practices`, { method: 'POST', body: { title } }),
  get: (practiceId: number) => request<Practice>(`/api/practices/${practiceId}`),
  recordAnswer: (practiceId: number, input: AnswerInput) =>
    request<Practice>(`/api/practices/${practiceId}/answers`, { method: 'POST', body: input }),
  updateAnswer: (practiceId: number, answerId: number, input: AnswerInput) =>
    request<Practice>(`/api/practices/${practiceId}/answers/${answerId}`, { method: 'PUT', body: input }),
  deleteAnswer: (practiceId: number, answerId: number) =>
    request<Practice>(`/api/practices/${practiceId}/answers/${answerId}`, { method: 'DELETE' }),
  grade: (practiceId: number, answerId: number, input: GradeInput) =>
    request<Practice>(`/api/practices/${practiceId}/answers/${answerId}/grade`, { method: 'POST', body: input }),
  finish: (practiceId: number) => request<Practice>(`/api/practices/${practiceId}/finish`, { method: 'POST' }),
  abort: (practiceId: number) =>
    request<{ practice: Practice | null }>(`/api/practices/${practiceId}/abort`, { method: 'POST' }),
}

/** 集計・振り返りの API。 */
export const reviewApi = {
  dashboard: (examId: number) => request<Dashboard>(`/api/exams/${examId}/dashboard`),
  questions: (examId: number, filter: QuestionFilter) => {
    const params =
      filter.kind === 'review'
        ? { filter: 'review' }
        : filter.kind === 'category'
          ? { categoryId: filter.categoryId }
          : {}
    return request<QuestionItem[]>(`/api/exams/${examId}/questions${query(params)}`)
  },
  history: (examId: number, title: string, questionNumber: number) =>
    request<Attempt[]>(`/api/exams/${examId}/history${query({ title, questionNumber })}`),
  exportUrl: (examId: number) => `/api/exams/${examId}/export`,
  changeCategory: (answerId: number, categoryId: number) =>
    request<void>(`/api/answers/${answerId}/category`, { method: 'PUT', body: { categoryId } }),
  updateMemo: (answerId: number, correctAnswer: string, note: string) =>
    request<void>(`/api/answers/${answerId}/memo`, { method: 'PUT', body: { correctAnswer, note } }),
}
