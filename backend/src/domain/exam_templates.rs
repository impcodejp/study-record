//! 試験のテンプレート（よく受験される資格試験と、その出題分野をもとにしたカテゴリ）。
//!
//! 新しく使い始めた利用者が、試験を選ぶだけでカテゴリをそろえて学習を始められるようにする。
//! カテゴリは各試験の公式の出題範囲（分野・科目）の大きな区分にそろえている。
//! 試験の制度が変わったときは、この一覧を更新する（作成済みの試験のカテゴリは変わらない）。

use serde::Serialize;

/// 試験のテンプレート。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamTemplate {
    /// テンプレートを指定するためのキー（API の入力に使う）。
    pub key: &'static str,
    /// 試験名（試験を作るときの初期値）。
    pub name: &'static str,
    /// 分類（画面でまとめて表示するため）。
    pub group: &'static str,
    /// 作成するカテゴリ（この順に並べる。「未分類」は別に必ず作る）。
    pub categories: &'static [&'static str],
}

/// 試験のテンプレートの一覧（画面に表示する順）。
pub const EXAM_TEMPLATES: &[ExamTemplate] = &[
    ExamTemplate {
        key: "it-passport",
        name: "ITパスポート試験",
        group: "IT",
        categories: &["ストラテジ系", "マネジメント系", "テクノロジ系"],
    },
    ExamTemplate {
        key: "fe-a",
        name: "基本情報技術者試験 科目A",
        group: "IT",
        categories: &[
            "基礎理論",
            "コンピュータシステム",
            "技術要素",
            "開発技術",
            "プロジェクトマネジメント",
            "サービスマネジメント",
            "システム戦略",
            "経営戦略",
            "企業と法務",
        ],
    },
    ExamTemplate {
        key: "fe-b",
        name: "基本情報技術者試験 科目B",
        group: "IT",
        categories: &["アルゴリズムとプログラミング", "情報セキュリティ"],
    },
    ExamTemplate {
        key: "ap-am",
        name: "応用情報技術者試験 午前",
        group: "IT",
        categories: &[
            "基礎理論",
            "コンピュータシステム",
            "技術要素",
            "開発技術",
            "プロジェクトマネジメント",
            "サービスマネジメント",
            "システム戦略",
            "経営戦略",
            "企業と法務",
        ],
    },
    ExamTemplate {
        key: "ap-pm",
        name: "応用情報技術者試験 午後",
        group: "IT",
        categories: &[
            "経営戦略",
            "情報戦略",
            "情報セキュリティ",
            "システムアーキテクチャ",
            "ネットワーク",
            "データベース",
            "組込みシステム開発",
            "情報システム開発",
            "プログラミング",
            "プロジェクトマネジメント",
            "サービスマネジメント",
            "システム監査",
        ],
    },
    ExamTemplate {
        key: "boki-2",
        name: "日商簿記検定 2級",
        group: "会計・金融",
        categories: &["商業簿記（仕訳）", "商業簿記（決算・財務諸表）", "連結会計", "工業簿記"],
    },
    ExamTemplate {
        key: "boki-3",
        name: "日商簿記検定 3級",
        group: "会計・金融",
        categories: &["仕訳", "勘定記入・補助簿", "決算整理・精算表", "財務諸表"],
    },
    ExamTemplate {
        key: "fp-2",
        name: "FP技能検定 2級",
        group: "会計・金融",
        categories: &[
            "ライフプランニングと資金計画",
            "リスク管理",
            "金融資産運用",
            "タックスプランニング",
            "不動産",
            "相続・事業承継",
        ],
    },
    ExamTemplate {
        key: "takken",
        name: "宅地建物取引士",
        group: "法律・不動産",
        categories: &["権利関係", "法令上の制限", "宅建業法", "税・その他"],
    },
    ExamTemplate {
        key: "toeic-lr",
        name: "TOEIC Listening & Reading",
        group: "語学",
        categories: &["Part 1", "Part 2", "Part 3", "Part 4", "Part 5", "Part 6", "Part 7"],
    },
];

/// キーからテンプレートを探す。
pub fn find_template(key: &str) -> Option<&'static ExamTemplate> {
    EXAM_TEMPLATES.iter().find(|t| t.key == key)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::domain::validation::{validate_category_name, validate_exam_name};

    #[test]
    fn templates_are_valid_and_unique() {
        let mut keys = HashSet::new();
        for template in EXAM_TEMPLATES {
            assert!(keys.insert(template.key), "キーが重複: {}", template.key);
            validate_exam_name(template.name).unwrap();
            let mut names = HashSet::new();
            for category in template.categories {
                validate_category_name(category).unwrap();
                // カテゴリ名は大文字・小文字を区別せず一意（DB の制約と同じ）。「未分類」は別に作る
                assert!(names.insert(category.to_lowercase()), "カテゴリが重複: {category}");
                assert_ne!(*category, "未分類");
            }
        }
        assert_eq!(find_template("takken").unwrap().name, "宅地建物取引士");
        assert!(find_template("none").is_none());
    }
}
