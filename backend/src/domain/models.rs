//! 業務データのモデル。
//!
//! API のレスポンスとしてそのまま JSON に変換できるよう、
//! フィールド名は camelCase でシリアライズする。
//! 日時はすべて日本時間の `YYYY-MM-DD HH:MM:SS` 文字列で持つ。

use serde::Serialize;

/// ログイン中のユーザー（F-27）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// ユーザー ID。
    pub id: i64,
    /// ユーザー名。
    pub name: String,
    /// メールアドレス。
    pub email: String,
}

/// 資格試験（試験マスタ）。学習・カテゴリ・集計はすべて試験ごとに分かれる。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    /// 試験 ID。
    pub id: i64,
    /// 試験名（例：基本情報技術者試験 科目B）。
    pub name: String,
    /// 並び順。
    pub sort_order: i64,
    /// 登録日時（日本時間）。
    pub created_at: String,
    /// この試験の学習の件数。
    pub practice_count: i64,
    /// この試験の採点済み回答の件数。
    pub answer_count: i64,
    /// 試験日（`YYYY-MM-DD`）。未設定は `None`。
    pub exam_date: Option<String>,
    /// 1 日の目標問題数。未設定は `None`。
    pub daily_goal: Option<i64>,
}

/// カテゴリ（試験ごとのカテゴリマスタ）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    /// カテゴリ ID。
    pub id: i64,
    /// 所属する試験 ID。
    pub exam_id: i64,
    /// カテゴリ名。
    pub name: String,
    /// 並び順。
    pub sort_order: i64,
    /// このカテゴリを使っている回答の件数（未採点を含む）。
    pub answer_count: i64,
}

/// 1 問分の回答。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    /// 回答 ID（記録順）。
    pub id: i64,
    /// 問題名称。
    pub title: String,
    /// 問題数（問題番号）。
    pub question_number: i64,
    /// カテゴリ名。
    pub area: String,
    /// 記録した回答（自由記述）。
    pub response: String,
    /// 回答時間（秒）。
    pub elapsed_seconds: i64,
    /// 記録日時（日本時間）。
    pub answered_at: String,
    /// 正誤。`None` は未採点。
    pub correct: Option<bool>,
    /// 採点時に記録した正解（任意）。
    pub correct_answer: Option<String>,
    /// 採点時に記録したメモ・解説（任意）。
    pub note: Option<String>,
}

/// 1 回分の学習（回答を含む）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Practice {
    /// 学習 ID。
    pub id: i64,
    /// 試験 ID。
    pub exam_id: i64,
    /// 試験名。
    pub exam_name: String,
    /// 開始時の問題名称。
    pub title: String,
    /// 開始日時（日本時間）。
    pub created_at: String,
    /// 完了日時（日本時間）。`None` は進行中。
    pub completed_at: Option<String>,
    /// 回答件数。
    pub question_count: i64,
    /// 採点済み件数。
    pub graded_count: i64,
    /// 正解数。
    pub correct_count: i64,
    /// 回答（記録順）。
    pub answers: Vec<Answer>,
}

/// 学習履歴一覧の 1 行（学習 × 問題名称の単位、F-11）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSummary {
    /// 学習 ID。
    pub id: i64,
    /// 問題名称。
    pub title: String,
    /// 開始日時（日本時間）。
    pub created_at: String,
    /// 完了日時（日本時間）。
    pub completed_at: Option<String>,
    /// 問題数（採点済み回答の件数）。
    pub question_count: i64,
    /// 正解数。
    pub correct_count: i64,
}

/// 問題一覧の 1 行（問題名称＋問題数ごとの最新結果、F-13）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionItem {
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// 最新回答のカテゴリ名。
    pub area: String,
    /// 最新の回答。
    pub response: String,
    /// 最終回答日時（日本時間）。
    pub answered_at: String,
    /// 最新の正誤。
    pub correct: bool,
    /// 回答回数。
    pub attempt_count: i64,
    /// 正解した回数。
    pub correct_count: i64,
    /// 習熟度（0〜5。最新から数えて連続で正解した回数。domain::study_plan を参照）。
    pub mastery_level: u8,
    /// 次に復習する日（日本時間、`YYYY-MM-DD`）。
    pub next_review_on: String,
    /// 平均回答時間（秒）。
    pub average_seconds: f64,
}

/// 見直しノートの 1 行（採点時に正解・メモを残した回答）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteItem {
    /// 回答 ID。
    pub answer_id: i64,
    /// 学習 ID。
    pub practice_id: i64,
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// カテゴリ名。
    pub area: String,
    /// 記録した回答。
    pub response: String,
    /// 正誤。
    pub correct: bool,
    /// 正解（任意）。
    pub correct_answer: Option<String>,
    /// メモ・解説（任意）。
    pub note: Option<String>,
    /// 回答日時（日本時間）。
    pub answered_at: String,
}

/// 問題ごとの正誤履歴の 1 行（F-16）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    /// 回答 ID。
    pub id: i64,
    /// 学習 ID。
    pub practice_id: i64,
    /// カテゴリ名。
    pub area: String,
    /// 回答。
    pub response: String,
    /// 回答時間（秒）。
    pub elapsed_seconds: i64,
    /// 回答日時（日本時間）。
    pub answered_at: String,
    /// 正誤。
    pub correct: bool,
    /// 正解（任意）。
    pub correct_answer: Option<String>,
    /// メモ（任意）。
    pub note: Option<String>,
}

/// 日別の推移の 1 日分（F-09）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCount {
    /// 日付（日本時間、`YYYY-MM-DD`）。
    pub date: String,
    /// 採点済み回答数。
    pub count: i64,
    /// そのうちの正解数。
    pub correct_count: i64,
}

/// カテゴリ別の集計（F-10）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaStat {
    /// カテゴリ名。
    pub area: String,
    /// 解答数。
    pub total_count: i64,
    /// 正解数。
    pub correct_count: i64,
    /// カテゴリマスタに登録されているか。
    pub in_master: bool,
}

/// ダッシュボードの集計結果（F-08〜F-10）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    /// 試験 ID。
    pub exam_id: i64,
    /// 解いた問題数（採点済み回答の件数）。
    pub total_count: i64,
    /// 正解数。
    pub correct_count: i64,
    /// 平均回答時間（秒）。回答がなければ 0。
    pub average_seconds: f64,
    /// 過去 14 日間の日別件数（古い日から順）。
    pub daily_counts: Vec<DailyCount>,
    /// カテゴリ別の集計（カテゴリマスタの並び順）。
    pub area_stats: Vec<AreaStat>,
    /// 今日の学習の状況（目標・連続日数・復習）。
    pub today: TodayPlan,
    /// 学習カレンダー用の日別件数（[`HEATMAP_DAYS`] 日分、古い日から順）。
    pub heatmap: Vec<DailyCount>,
    /// 試験の準備度とペースの目安。
    pub readiness: Readiness,
}

/// 試験の準備度とペースの目安（domain::study_plan の計算を参照）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    /// 全体の準備度（0〜100%）。解いたことのある問題の習熟度の平均。
    pub percent: f64,
    /// カテゴリ別の準備度（カテゴリマスタの並び順。問題が無いカテゴリは含めない）。
    pub areas: Vec<AreaReadiness>,
    /// 直近の 1 日あたりの平均解答数（日別の推移と同じ期間）。
    pub recent_daily_average: f64,
    /// すべての問題を習得済みにするまでに必要な正解の回数。
    pub remaining_correct_answers: i64,
    /// 試験日までに習得済みにするための 1 日あたりの問題数。試験日が未設定・当日以降なら `None`。
    pub required_daily: Option<i64>,
    /// 準備度の推移（[`READINESS_WEEKS`] 週分。古い順で、最後が今日）。
    pub history: Vec<ReadinessPoint>,
}

/// 準備度の推移を表示する週数。
pub const READINESS_WEEKS: i64 = 8;

/// ある日の終わり時点の準備度。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessPoint {
    /// 日付（日本時間、`YYYY-MM-DD`）。
    pub date: String,
    /// その日までの回答から求めた準備度（0〜100%）。
    pub percent: f64,
    /// その日までに解いたことのある問題の数。
    pub question_count: i64,
}

/// カテゴリ別の準備度。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaReadiness {
    /// カテゴリ名（問題の最新回答のカテゴリ）。
    pub area: String,
    /// 準備度（0〜100%）。
    pub percent: f64,
    /// 解いたことのある問題の数。
    pub question_count: i64,
    /// 習得済みの問題の数。
    pub mastered_count: i64,
}

/// 学習カレンダーに表示する日数（26 週）。
pub const HEATMAP_DAYS: i64 = 26 * 7;

/// 今日の学習の状況（ダッシュボードの「今日やること」）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayPlan {
    /// 今日（日本時間、`YYYY-MM-DD`）。
    pub date: String,
    /// 試験日（`YYYY-MM-DD`）。未設定は `None`。
    pub exam_date: Option<String>,
    /// 試験日までの日数（当日は 0、過ぎていれば負）。試験日が未設定なら `None`。
    pub days_until_exam: Option<i64>,
    /// 1 日の目標問題数。未設定は `None`。
    pub daily_goal: Option<i64>,
    /// 今日採点した回答の件数。
    pub answered_count: i64,
    /// そのうちの正解数。
    pub correct_count: i64,
    /// 今の連続学習日数（今日まだ解いていなくても、昨日まで続いていれば数える）。
    pub current_streak: i64,
    /// これまでの最長の連続学習日数。
    pub longest_streak: i64,
    /// 今日までに復習の時期が来ている問題の数。
    pub due_count: i64,
    /// 解いたことのある問題の数（問題名称＋問題数の単位）。
    pub question_count: i64,
    /// 習得済み（習熟度が上限）の問題の数。
    pub mastered_count: i64,
}
