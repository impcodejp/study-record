/**
 * ダッシュボード（F-08〜F-10、今日やること、合格準備度、学習カレンダー、苦手なカテゴリ、実績）。
 */

import { Link, useNavigate } from 'react-router-dom'

import { examApi, reviewApi, type GoalInput } from '../api/endpoints'
import { AreaChart, DailyChart } from '../components/Charts'
import { AchievementsCard, ReadinessCard, StudyHeatmap, TodayPanel, WeakAreas } from '../components/StudyPlan'
import { Loading, Notice, PageHeader } from '../components/ui'
import { useExamId } from '../hooks/useExamId'
import { useLoad } from '../hooks/useLoad'
import { formatCount, formatRate } from '../utils/format'

/** ダッシュボード画面。 */
export function DashboardPage() {
  const examId = useExamId()
  const navigate = useNavigate()
  const dashboard = useLoad(() => reviewApi.dashboard(examId), [examId])
  const categories = useLoad(() => examApi.categories(examId), [examId])

  if (dashboard.loading && !dashboard.data) return <Loading />
  if (dashboard.error) return <Notice>{dashboard.error}</Notice>
  const d = dashboard.data
  if (!d) return null

  /** カテゴリを選んだら、そのカテゴリの履歴（F-15）へ。 */
  const openCategory = (area: string) => {
    const category = categories.data?.find((c) => c.name === area)
    if (category) navigate(`/exams/${examId}/questions?categoryId=${category.id}`)
  }

  /** 学習目標を保存し、集計を読み込み直す（失敗時は呼び出し元のフォームがエラーを表示する）。 */
  const saveGoal = async (goal: GoalInput) => {
    await examApi.updateGoal(examId, goal)
    dashboard.reload()
  }

  return (
    <div>
      <PageHeader
        title="ダッシュボード"
        actions={
          <>
            <a className="button" href={reviewApi.exportUrl(examId)} download>
              CSV で書き出す
            </a>
            <Link className="button primary" to={`/exams/${examId}/start`}>
              新しい学習を始める
            </Link>
          </>
        }
      />
      <TodayPanel examId={examId} today={d.today} onSaveGoal={saveGoal} />
      <ReadinessCard readiness={d.readiness} daysUntilExam={d.today.daysUntilExam} />
      <section className="stats" aria-label="累計">
        <div className="stat hero">
          <span className="stat-label">正答率</span>
          <span className="stat-value">{formatRate(d.correctCount, d.totalCount)}</span>
        </div>
        <div className="stat">
          <span className="stat-label">解いた問題数</span>
          <span className="stat-value">{formatCount(d.totalCount)}</span>
        </div>
        <div className="stat">
          <span className="stat-label">正解数</span>
          <span className="stat-value">{formatCount(d.correctCount)}</span>
        </div>
        <div className="stat">
          <span className="stat-label">平均回答時間</span>
          <span className="stat-value">
            {d.averageSeconds.toFixed(1)}
            <small>秒</small>
          </span>
        </div>
      </section>
      <div className="chart-grid">
        <section className="card">
          <DailyChart data={d.dailyCounts} />
        </section>
        <section className="card">
          <AreaChart data={d.areaStats} onSelect={(stat) => openCategory(stat.area)} />
        </section>
      </div>
      <div className="chart-grid chart-grid-2">
        <section className="card">
          <StudyHeatmap data={d.heatmap} dailyGoal={d.today.dailyGoal} />
        </section>
        <section className="card stack-card">
          <WeakAreas stats={d.areaStats} onSelect={(stat) => openCategory(stat.area)} />
          <AchievementsCard dashboard={d} />
        </section>
      </div>
    </div>
  )
}
