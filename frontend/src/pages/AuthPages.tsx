/**
 * アカウント関連の画面（F-22〜F-26）。
 *
 * ログイン／新規登録／確認メールの再送／登録の完了／パスワード再設定／パスワード変更。
 */

import { useState, type FormEvent, type ReactNode } from 'react'
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router-dom'

import { errorMessage } from '../api/client'
import { authApi } from '../api/endpoints'
import { Notice, PageHeader } from '../components/ui'
import { useApp } from '../hooks/useApp'

/** パスワードの最小文字数（サーバー側の検証と合わせる）。 */
const PASSWORD_MIN = 8

/** ログイン前の画面の枠。 */
function AuthCard({ title, children, footer }: { title: string; children: ReactNode; footer?: ReactNode }) {
  return (
    <div className="auth-page">
      <div className="auth-card">
        <p className="brand">学習記録</p>
        <h1>{title}</h1>
        {children}
        {footer && <div className="auth-footer">{footer}</div>}
      </div>
    </div>
  )
}

/** 送信中の状態とエラー・お知らせをまとめて扱う。 */
function useSubmit() {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const run = async (action: () => Promise<string | void>) => {
    setBusy(true)
    setError(null)
    setMessage(null)
    try {
      const result = await action()
      if (result) setMessage(result)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }
  return { busy, error, message, run, setError }
}

/** 新しいパスワードと確認用の入力欄。 */
function NewPasswordFields({
  password,
  confirm,
  onPassword,
  onConfirm,
  label = 'パスワード',
}: {
  password: string
  confirm: string
  onPassword: (v: string) => void
  onConfirm: (v: string) => void
  label?: string
}) {
  return (
    <>
      <label className="field">
        <span>{label}（{PASSWORD_MIN}文字以上）</span>
        <input
          type="password"
          autoComplete="new-password"
          value={password}
          minLength={PASSWORD_MIN}
          required
          onChange={(e) => onPassword(e.target.value)}
        />
      </label>
      <label className="field">
        <span>{label}（確認）</span>
        <input
          type="password"
          autoComplete="new-password"
          value={confirm}
          required
          onChange={(e) => onConfirm(e.target.value)}
        />
      </label>
    </>
  )
}

/** 新しいパスワードの入力を確認する。問題があればメッセージを返す。 */
function checkNewPassword(password: string, confirm: string): string | null {
  if (password.length < PASSWORD_MIN) return `パスワードは${PASSWORD_MIN}文字以上で入力してください。`
  if (password !== confirm) return 'パスワード（確認）が一致しません。'
  return null
}

/** ログイン画面（F-24）。 */
export function LoginPage() {
  const { login } = useApp()
  const navigate = useNavigate()
  const location = useLocation()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const { busy, error, run } = useSubmit()
  const notice = (location.state as { notice?: string } | null)?.notice

  const submit = (e: FormEvent) => {
    e.preventDefault()
    run(async () => {
      await login(email, password)
      const from = (location.state as { from?: string } | null)?.from
      navigate(from && from !== '/login' ? from : '/', { replace: true })
    })
  }

  return (
    <AuthCard
      title="ログイン"
      footer={
        <>
          <Link to="/register">新規登録</Link>
          <Link to="/forgot-password">パスワードを忘れた場合</Link>
        </>
      }
    >
      <Notice kind="success">{notice}</Notice>
      <Notice>{error}</Notice>
      <form onSubmit={submit} className="stack">
        <label className="field">
          <span>メールアドレス</span>
          <input type="email" autoComplete="email" value={email} required onChange={(e) => setEmail(e.target.value)} />
        </label>
        <label className="field">
          <span>パスワード</span>
          <input
            type="password"
            autoComplete="current-password"
            value={password}
            required
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <button type="submit" className="button primary" disabled={busy}>
          {busy ? 'ログイン中…' : 'ログイン'}
        </button>
      </form>
    </AuthCard>
  )
}

/** 新規登録画面（F-22）と確認メールの再送（F-23）。 */
export function RegisterPage() {
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [sent, setSent] = useState(false)
  const { busy, error, message, run } = useSubmit()

  const submit = (e: FormEvent) => {
    e.preventDefault()
    run(async () => {
      const result = await authApi.register(name, email)
      setSent(true)
      return result.message
    })
  }

  const resend = () =>
    run(async () => {
      const result = await authApi.resendVerification(email)
      return result.message
    })

  return (
    <AuthCard title="新規登録" footer={<Link to="/login">ログイン画面へ戻る</Link>}>
      <Notice kind="success">{message}</Notice>
      <Notice>{error}</Notice>
      {sent ? (
        <div className="stack">
          <p>
            メールに記載されたリンクを開いてパスワードを設定すると、登録が完了します。
            確認が終わるまでアカウントは作られません。
          </p>
          <button type="button" className="button" onClick={resend} disabled={busy}>
            確認メールを再送する
          </button>
        </div>
      ) : (
        <form onSubmit={submit} className="stack">
          <label className="field">
            <span>ユーザー名（50文字以内）</span>
            <input value={name} maxLength={50} required autoComplete="name" onChange={(e) => setName(e.target.value)} />
          </label>
          <label className="field">
            <span>メールアドレス</span>
            <input type="email" value={email} required autoComplete="email" onChange={(e) => setEmail(e.target.value)} />
          </label>
          <button type="submit" className="button primary" disabled={busy}>
            確認メールを送る
          </button>
          <p className="muted small">
            以前に登録手続きをした方は、メールアドレスを入力して
            <button type="button" className="link-button" onClick={resend} disabled={busy || !email}>
              確認メールを再送
            </button>
            できます。
          </p>
        </form>
      )}
    </AuthCard>
  )
}

/** 確認リンクから開く、パスワード設定による登録完了の画面（F-22）。 */
export function VerifyEmailPage() {
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const token = params.get('token') ?? ''
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const { busy, error, run, setError } = useSubmit()

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const problem = checkNewPassword(password, confirm)
    if (problem) return setError(problem)
    run(async () => {
      await authApi.verifyEmail(token, password)
      navigate('/login', { replace: true, state: { notice: '登録が完了しました。ログインしてください。' } })
    })
  }

  return (
    <AuthCard title="パスワードの設定" footer={<Link to="/login">ログイン画面へ</Link>}>
      {!token ? (
        <Notice>リンクが正しくありません。メールのリンクをもう一度開いてください。</Notice>
      ) : (
        <>
          <Notice>{error}</Notice>
          <form onSubmit={submit} className="stack">
            <NewPasswordFields password={password} confirm={confirm} onPassword={setPassword} onConfirm={setConfirm} />
            <button type="submit" className="button primary" disabled={busy}>
              登録を完了する
            </button>
          </form>
        </>
      )}
    </AuthCard>
  )
}

/** パスワード再設定メールの送信画面（F-25）。 */
export function ForgotPasswordPage() {
  const [email, setEmail] = useState('')
  const { busy, error, message, run } = useSubmit()

  const submit = (e: FormEvent) => {
    e.preventDefault()
    run(async () => (await authApi.forgotPassword(email)).message)
  }

  return (
    <AuthCard title="パスワードの再設定" footer={<Link to="/login">ログイン画面へ戻る</Link>}>
      <Notice kind="success">{message}</Notice>
      <Notice>{error}</Notice>
      <form onSubmit={submit} className="stack">
        <p>登録したメールアドレスに、パスワード再設定用のリンクを送ります。</p>
        <label className="field">
          <span>メールアドレス</span>
          <input type="email" value={email} required autoComplete="email" onChange={(e) => setEmail(e.target.value)} />
        </label>
        <button type="submit" className="button primary" disabled={busy}>
          再設定メールを送る
        </button>
      </form>
    </AuthCard>
  )
}

/** 再設定リンクから開く、新しいパスワードの設定画面（F-25）。 */
export function ResetPasswordPage() {
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const token = params.get('token') ?? ''
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const { busy, error, run, setError } = useSubmit()

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const problem = checkNewPassword(password, confirm)
    if (problem) return setError(problem)
    run(async () => {
      await authApi.resetPassword(token, password)
      navigate('/login', {
        replace: true,
        state: { notice: 'パスワードを再設定しました。新しいパスワードでログインしてください。' },
      })
    })
  }

  return (
    <AuthCard title="新しいパスワードの設定" footer={<Link to="/login">ログイン画面へ</Link>}>
      {!token ? (
        <Notice>リンクが正しくありません。メールのリンクをもう一度開いてください。</Notice>
      ) : (
        <>
          <Notice>{error}</Notice>
          <form onSubmit={submit} className="stack">
            <NewPasswordFields
              label="新しいパスワード"
              password={password}
              confirm={confirm}
              onPassword={setPassword}
              onConfirm={setConfirm}
            />
            <button type="submit" className="button primary" disabled={busy}>
              パスワードを設定する
            </button>
          </form>
        </>
      )}
    </AuthCard>
  )
}

/** アカウント画面（F-26 パスワード変更、F-27 ユーザー情報）。 */
export function AccountPage() {
  const { user } = useApp()
  const [current, setCurrent] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const { busy, error, message, run, setError } = useSubmit()

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const problem = checkNewPassword(password, confirm)
    if (problem) return setError(problem)
    run(async () => {
      await authApi.changePassword(current, password)
      setCurrent('')
      setPassword('')
      setConfirm('')
      return 'パスワードを変更しました。ほかの端末ではログアウトされます。'
    })
  }

  return (
    <div className="narrow">
      <PageHeader title="アカウント" />
      <section className="card">
        <h2>ユーザー情報</h2>
        <dl className="details">
          <dt>ユーザー名</dt>
          <dd>{user?.name}</dd>
          <dt>メールアドレス</dt>
          <dd>{user?.email}</dd>
        </dl>
      </section>
      <section className="card">
        <h2>パスワードの変更</h2>
        <Notice kind="success">{message}</Notice>
        <Notice>{error}</Notice>
        <form onSubmit={submit} className="stack">
          <label className="field">
            <span>現在のパスワード</span>
            <input
              type="password"
              autoComplete="current-password"
              value={current}
              required
              onChange={(e) => setCurrent(e.target.value)}
            />
          </label>
          <NewPasswordFields
            label="新しいパスワード"
            password={password}
            confirm={confirm}
            onPassword={setPassword}
            onConfirm={setConfirm}
          />
          <button type="submit" className="button primary" disabled={busy}>
            変更する
          </button>
        </form>
      </section>
    </div>
  )
}
