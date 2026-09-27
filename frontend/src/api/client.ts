/**
 * API サーバーとの通信の共通処理。
 *
 * - セッションは Cookie（HttpOnly）で自動的に送られる
 * - 状態を変える要求には `X-Requested-With` と `X-CSRF-Token` ヘッダーを付ける
 * - エラー時はサーバーの `{ "error": "..." }` を ApiError として投げる
 */

/** API のエラー。 */
export class ApiError extends Error {
  /** HTTP ステータス。通信自体に失敗した場合は 0。 */
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

/** ログイン後に受け取った CSRF トークン（メモリ上だけに保持する）。 */
let csrfToken: string | null = null

/** 401 を受け取ったときに呼ぶ処理（ログイン画面への切り替え）。 */
let onUnauthorized: (() => void) | null = null

/** CSRF トークンを設定する（ログイン時・ユーザー情報取得時）。 */
export function setCsrfToken(token: string | null): void {
  csrfToken = token
}

/** 401 を受け取ったときの処理を登録する。 */
export function setUnauthorizedHandler(handler: (() => void) | null): void {
  onUnauthorized = handler
}

/** 要求のオプション。 */
interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  body?: unknown
  /** true のとき 401 でもログイン画面へ切り替えない（ログイン状態の確認用）。 */
  silentUnauthorized?: boolean
}

/**
 * API を呼び出し、JSON の結果を返す。204 の場合は undefined を返す。
 */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const method = options.method ?? 'GET'
  const headers: Record<string, string> = { Accept: 'application/json' }
  if (method !== 'GET') {
    headers['X-Requested-With'] = 'fetch'
    if (csrfToken) headers['X-CSRF-Token'] = csrfToken
  }
  if (options.body !== undefined) headers['Content-Type'] = 'application/json'

  let response: Response
  try {
    response = await fetch(path, {
      method,
      headers,
      credentials: 'same-origin',
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    })
  } catch {
    throw new ApiError(0, 'サーバーに接続できません。ネットワークを確認してください。')
  }

  if (response.status === 401 && !options.silentUnauthorized) onUnauthorized?.()
  if (response.status === 204) return undefined as T

  const text = await response.text()
  const data: unknown = text ? safeParse(text) : undefined
  if (!response.ok) {
    const message =
      data && typeof data === 'object' && 'error' in data && typeof data.error === 'string'
        ? data.error
        : `エラーが発生しました（${response.status}）。`
    throw new ApiError(response.status, message)
  }
  return data as T
}

/** JSON として解析できなければ文字列のまま返す。 */
function safeParse(text: string): unknown {
  try {
    return JSON.parse(text)
  } catch {
    return text
  }
}

/** エラーを画面表示用のメッセージに変換する。 */
export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.message
  if (error instanceof Error) return error.message
  return '予期しないエラーが発生しました。'
}

/** クエリ文字列を組み立てる。 */
export function query(params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== '') search.set(key, String(value))
  }
  const text = search.toString()
  return text ? `?${text}` : ''
}
