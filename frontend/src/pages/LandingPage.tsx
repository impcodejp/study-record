/**
 * サービスの紹介ページ（ログイン前のトップ）。
 *
 * 初めて訪れた人に、何ができて、なぜ合格に近づくのかを伝え、新規登録へ案内する。
 * 画面の見本は実際の画面と同じ部品の見た目で、数値は「表示例」と明記する。
 * 対応している試験の一覧はテンプレートの API から取得する（取得できなくてもページは表示する）。
 */

import { Link } from 'react-router-dom'

import { examApi } from '../api/endpoints'
import { useLoad } from '../hooks/useLoad'

/** 主な機能。 */
const FEATURES = [
  {
    title: '忘れかけた問題を、毎日教えてくれる',
    body: '正解が続くほど次の復習までの間隔が延びる「間隔反復」で、問題ごとに復習日を自動で計算。今日解くべき問題だけが一覧になります。',
  },
  {
    title: '復習リストを、そのまま解ける',
    body: '「まとめて復習」なら、問題名称・番号・分野が 1 問ずつ自動で入ります。答えを書いて記録するだけで、次の問題へ進みます。',
  },
  {
    title: '合格まであと何問かがわかる',
    body: '習熟度から合格準備度を出し、試験日までに必要な 1 日の問題数と、今のペースを比べます。分野別の準備度で、伸ばすべき分野も一目で。',
  },
  {
    title: '間違いを、自分だけの参考書に',
    body: '採点のときに残した正解と解説は「見直しノート」に集まります。キーワードや分野で絞り込めるので、試験直前の見直しに最適です。',
  },
  {
    title: '続けたくなる記録',
    body: '連続学習日数、学習カレンダー、目標の達成度、実績バッジ。毎日の小さな積み重ねが目に見えます。',
  },
  {
    title: '手持ちの問題集・過去問でそのまま',
    body: '問題の本文は不要。問題集の名前と番号で管理するので、どの教材でも使えます。データは CSV で書き出せます。',
  },
]

/** 使い方の 3 ステップ。 */
const STEPS = [
  { title: '試験を選ぶ', body: '主な資格は、出題分野のカテゴリがそろったテンプレートから始められます。' },
  { title: '解いて、記録する', body: '問題集を解きながら、回答を記録。回答時間も自動で測ります。' },
  { title: '採点するだけ', body: '自己採点すると、習熟度と次の復習日をアプリが計算します。' },
]

/** よくある質問。 */
const FAQ = [
  {
    q: '問題集の本文を入力する必要はありますか？',
    a: 'ありません。問題集の名称と問題番号、回答だけを記録します。お手持ちの問題集・過去問・模試にそのまま使えます。',
  },
  {
    q: 'スマートフォンでも使えますか？',
    a: '使えます。ブラウザのメニューから「ホーム画面に追加」すると、アプリのように起動できます。',
  },
  {
    q: '一覧に無い試験でも使えますか？',
    a: '使えます。試験名を入力して作り、カテゴリ（分野）を自由に登録できます。',
  },
  {
    q: '記録したデータを持ち出せますか？',
    a: '試験ごとに、採点済みの回答を CSV（Excel で開ける形式）で書き出せます。',
  },
]

/** サービスの紹介ページ。 */
export function LandingPage() {
  const templates = useLoad(() => examApi.templates(), [])

  return (
    <div className="landing">
      <header className="landing-nav">
        <span className="brand">学習記録</span>
        <nav aria-label="アカウント">
          <Link to="/login">ログイン</Link>
          <Link className="button primary small" to="/register">
            無料で始める
          </Link>
        </nav>
      </header>

      <section className="landing-hero">
        <div className="hero-text">
          <p className="hero-eyebrow">資格試験の学習記録アプリ</p>
          <h1>
            解いた問題を、
            <br />
            合格に変える。
          </h1>
          <p className="hero-lead">
            問題集の回答を記録して自己採点するだけ。忘れかけた問題を毎日教えてくれて、試験日までに何をどれだけ解けばよいかがわかります。
          </p>
          <div className="hero-actions">
            <Link className="button primary large" to="/register">
              無料で始める
            </Link>
            <Link className="button large" to="/login">
              ログイン
            </Link>
          </div>
        </div>
        <HeroPreview />
      </section>

      <section className="landing-section" aria-labelledby="features-heading">
        <h2 id="features-heading">合格に必要な「次の一問」がわかる</h2>
        <div className="feature-grid">
          {FEATURES.map((f) => (
            <article key={f.title} className="feature card">
              <h3>{f.title}</h3>
              <p>{f.body}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="landing-section" aria-labelledby="steps-heading">
        <h2 id="steps-heading">使い方は 3 ステップ</h2>
        <ol className="landing-steps">
          {STEPS.map((s, i) => (
            <li key={s.title} className="card">
              <span className="step-number" aria-hidden="true">
                {i + 1}
              </span>
              <h3>{s.title}</h3>
              <p>{s.body}</p>
            </li>
          ))}
        </ol>
      </section>

      {!!templates.data?.length && (
        <section className="landing-section" aria-labelledby="templates-heading">
          <h2 id="templates-heading">テンプレートがある試験</h2>
          <p className="muted">出題分野のカテゴリがそろった状態で始められます。ほかの試験も自由に登録できます。</p>
          <ul className="chip-list">
            {templates.data.map((t) => (
              <li key={t.key} className="chip">
                {t.name}
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="landing-section" aria-labelledby="faq-heading">
        <h2 id="faq-heading">よくある質問</h2>
        <div className="faq">
          {FAQ.map((item) => (
            <details key={item.q} className="card">
              <summary>{item.q}</summary>
              <p>{item.a}</p>
            </details>
          ))}
        </div>
      </section>

      <section className="landing-cta">
        <h2>今日の一問から、合格までの道のりを見える化しよう</h2>
        <Link className="button primary large" to="/register">
          無料で始める
        </Link>
      </section>

      <footer className="landing-footer muted small">
        <span>学習記録</span>
        <Link to="/login">ログイン</Link>
      </footer>
    </div>
  )
}

/** ヒーローの画面見本（実際の「今日やること」と同じ見た目。数値は表示例）。 */
function HeroPreview() {
  return (
    <figure className="hero-preview" aria-label="画面の表示例">
      <div className="today card">
        <div className="today-head">
          <h2>今日やること</h2>
          <span className="muted small">表示例</span>
        </div>
        <div className="today-grid preview-grid">
          <div className="today-item">
            <span className="today-label">試験まであと</span>
            <span className="today-value">
              24<small>日</small>
            </span>
          </div>
          <div className="today-item">
            <span className="today-label">今日の目標</span>
            <span className="today-value">
              12<small>/ 20問</small>
            </span>
            <span className="goal-track" aria-hidden="true">
              <span className="goal-fill" style={{ width: '60%' }} />
            </span>
          </div>
          <div className="today-item">
            <span className="today-label">連続学習</span>
            <span className="today-value">
              7<small>日</small>
            </span>
          </div>
          <div className="today-item today-review">
            <span className="today-label">今日の復習</span>
            <span className="today-value">
              9<small>問</small>
            </span>
            <span className="button small primary" aria-hidden="true">
              まとめて復習
            </span>
          </div>
        </div>
        <div className="preview-readiness">
          <span className="today-label">合格準備度</span>
          <span className="readiness-track" aria-hidden="true">
            <span className="readiness-fill" style={{ width: '62%' }} />
          </span>
          <strong>62%</strong>
        </div>
      </div>
      <figcaption className="muted small">数値は表示例です</figcaption>
    </figure>
  )
}
