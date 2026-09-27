/**
 * API のレスポンスの型定義。
 *
 * バックエンド（Rust）の domain/models.rs と対応させること。
 * 日時はすべて日本時間の `YYYY-MM-DD HH:MM:SS` 文字列。
 */

/** ログイン中のユーザー。 */
export interface User {
  id: number
  name: string
  email: string
}

/** ログイン情報（ユーザーと CSRF トークン）。 */
export interface SessionInfo {
  user: User
  csrfToken: string
}

/** 資格試験。 */
export interface Exam {
  id: number
  name: string
  sortOrder: number
  createdAt: string
  /** 学習の件数。 */
  practiceCount: number
  /** 採点済み回答の件数。 */
  answerCount: number
}

/** カテゴリ。 */
export interface Category {
  id: number
  examId: number
  name: string
  sortOrder: number
  /** このカテゴリを使っている回答の件数。 */
  answerCount: number
}

/** 1 問分の回答。 */
export interface Answer {
  id: number
  title: string
  questionNumber: number
  area: string
  response: string
  elapsedSeconds: number
  answeredAt: string
  /** 正誤。null は未採点。 */
  correct: boolean | null
  correctAnswer: string | null
  note: string | null
}

/** 1 回分の学習。 */
export interface Practice {
  id: number
  examId: number
  examName: string
  title: string
  createdAt: string
  completedAt: string | null
  questionCount: number
  gradedCount: number
  correctCount: number
  answers: Answer[]
}

/** 学習履歴一覧の 1 行（学習 × 問題名称）。 */
export interface PracticeSummary {
  id: number
  title: string
  createdAt: string
  completedAt: string | null
  questionCount: number
  correctCount: number
}

/** 問題一覧の 1 行。 */
export interface QuestionItem {
  title: string
  questionNumber: number
  area: string
  response: string
  answeredAt: string
  correct: boolean
  attemptCount: number
  correctCount: number
}

/** 問題ごとの正誤履歴の 1 行。 */
export interface Attempt {
  id: number
  practiceId: number
  area: string
  response: string
  elapsedSeconds: number
  answeredAt: string
  correct: boolean
  correctAnswer: string | null
  note: string | null
}

/** 日別の件数。 */
export interface DailyCount {
  date: string
  count: number
  correctCount: number
}

/** カテゴリ別の集計。 */
export interface AreaStat {
  area: string
  totalCount: number
  correctCount: number
  inMaster: boolean
}

/** ダッシュボードの集計。 */
export interface Dashboard {
  examId: number
  totalCount: number
  correctCount: number
  averageSeconds: number
  dailyCounts: DailyCount[]
  areaStats: AreaStat[]
}
