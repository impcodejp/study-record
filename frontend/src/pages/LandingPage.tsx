/**
 * サービスの紹介ページ（ログイン前のトップ）。
 *
 * 初めて訪れた人に、何ができて、なぜ合格に近づくのかを伝え、新規登録へ案内する。
 * 画面の見本は実際の画面と同じ部品の見た目で、数値は「表示例」と明記する。
 * 対応している試験の一覧はテンプレートの API から取得する（取得できなくてもページは表示する）。
 *
 * 検索エンジン向けに、ビルド時にこのページを HTML として書き出す（src/prerender.tsx）。
 * そのときは API を使えないため、試験名を `initialExamNames` で受け取る。
 */

import { Link } from 'react-router-dom'

import { examApi } from '../api/endpoints'
import { FEATURES, LANDING_TITLE, STEPS, faqItems } from '../content/landing'
import { useDocumentTitle } from '../hooks/useDocumentTitle'
import { useLoad } from '../hooks/useLoad'

/** サービスの紹介ページ。 */
export function LandingPage({ initialExamNames = [] }: { initialExamNames?: string[] }) {
  useDocumentTitle(LANDING_TITLE, { raw: true })
  const templates = useLoad(() => examApi.templates(), [])
  const examNames = templates.data?.map((t) => t.name) ?? initialExamNames
  const faq = faqItems(examNames)

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

      {examNames.length > 0 && (
        <section className="landing-section" aria-labelledby="templates-heading">
          <h2 id="templates-heading">テンプレートがある試験</h2>
          <p className="muted">出題分野のカテゴリがそろった状態で始められます。ほかの試験も自由に登録できます。</p>
          <ul className="chip-list">
            {examNames.map((name) => (
              <li key={name} className="chip">
                {name}
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="landing-section" aria-labelledby="faq-heading">
        <h2 id="faq-heading">よくある質問</h2>
        <div className="faq">
          {faq.map((item) => (
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
