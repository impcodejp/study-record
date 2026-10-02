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
  /** 試験日（YYYY-MM-DD）。未設定は null。 */
  examDate: string | null
  /** 1 日の目標問題数。未設定は null。 */
  dailyGoal: number | null
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
  /** 習熟度（0〜5。最新から数えて連続で正解した回数）。 */
  masteryLevel: number
  /** 次に復習する日（YYYY-MM-DD）。 */
  nextReviewOn: string
  /** 平均回答時間（秒）。 */
  averageSeconds: number
}

/** 見直しノートの 1 行（採点時に正解・メモを残した回答）。 */
export interface NoteItem {
  answerId: number
  practiceId: number
  title: string
  questionNumber: number
  area: string
  response: string
  correct: boolean
  correctAnswer: string | null
  note: string | null
  answeredAt: string
}

/** 習熟度の上限（この値で「習得済み」。backend の domain/study_plan.rs と同じ）。 */
export const MAX_MASTERY_LEVEL = 5

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
  /** 今日の学習の状況。 */
  today: TodayPlan
  /** 学習カレンダー用の日別件数（26 週分、古い日から順）。 */
  heatmap: DailyCount[]
  /** 試験の準備度とペースの目安。 */
  readiness: Readiness
}

/** 試験の準備度とペースの目安。 */
export interface Readiness {
  /** 全体の準備度（0〜100）。解いたことのある問題の習熟度の平均。 */
  percent: number
  /** カテゴリ別の準備度。 */
  areas: AreaReadiness[]
  /** 直近 14 日の 1 日あたりの平均解答数。 */
  recentDailyAverage: number
  /** すべての問題を習得済みにするまでに必要な正解の回数。 */
  remainingCorrectAnswers: number
  /** 試験日までに習得済みにするための 1 日あたりの問題数。試験日が未設定・当日以降なら null。 */
  requiredDaily: number | null
  /** 準備度の推移（8 週分。古い順で、最後が今日）。 */
  history: ReadinessPoint[]
}

/** カテゴリ別の準備度。 */
export interface AreaReadiness {
  area: string
  percent: number
  questionCount: number
  masteredCount: number
}

/** 今日の学習の状況（ダッシュボードの「今日やること」）。 */
export interface TodayPlan {
  /** 今日（YYYY-MM-DD）。 */
  date: string
  examDate: string | null
  /** 試験日までの日数（当日は 0、過ぎていれば負）。試験日が未設定なら null。 */
  daysUntilExam: number | null
  dailyGoal: number | null
  /** 今日採点した回答の件数。 */
  answeredCount: number
  correctCount: number
  /** 今の連続学習日数（今日まだでも昨日まで続いていれば数える）。 */
  currentStreak: number
  longestStreak: number
  /** 今日までに復習の時期が来ている問題の数。 */
  dueCount: number
  /** 解いたことのある問題の数。 */
  questionCount: number
  /** 習得済みの問題の数。 */
  masteredCount: number
}

/** ある日の終わり時点の準備度（準備度の推移）。 */
export interface ReadinessPoint {
  /** 日付（YYYY-MM-DD）。 */
  date: string
  /** 準備度（0〜100）。 */
  percent: number
  /** その日までに解いたことのある問題の数。 */
  questionCount: number
}

/** 試験のテンプレート（よく受験される資格試験と、その出題分野のカテゴリ）。 */
export interface ExamTemplate {
  key: string
  name: string
  /** 分類（IT・会計・金融など）。 */
  group: string
  categories: string[]
}
