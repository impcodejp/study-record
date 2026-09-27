# 資格勉強 回答正誤記録アプリ

資格試験の問題集を解きながら回答を記録し、自己採点して学習状況を振り返る Web アプリです。
複数の資格試験を登録でき、学習・カテゴリ・集計は試験ごとに分かれます。

| 構成 | 内容 |
|---|---|
| フロントエンド | React 19 + TypeScript + Vite（`frontend/`） |
| API サーバー | Rust + Axum（`backend/`） |
| データベース | SQLite（`data/study_record.sqlite3`） |
| Web サーバー | nginx（`nginx/study-record.conf`） |
| ログ | `logs/SystemRunningLog.log`（約 2MB で `SystemRunningLog1.log` に切り替え） |

詳しい仕様・設計判断は [docs/仕様書.md](docs/仕様書.md) を参照してください。

## フォルダ構成

```text
ProjectRoot/
├─ backend/                 API サーバー（Rust）
│  ├─ src/domain/           業務ルール・モデル（他の層に依存しない）
│  ├─ src/application/      ユースケース（トランザクションの範囲を決める）
│  ├─ src/infrastructure/   DB・メール・設定・ログ
│  ├─ src/presentation/     HTTP API（入出力の変換だけ）
│  ├─ src/bin/              管理ツール（create_user / import_legacy）
│  └─ tests/                API の結合テスト
├─ frontend/                画面（React）
│  └─ src/{api,hooks,components,pages,utils}
├─ config/app.ini.example   設定ファイルの見本
├─ nginx/study-record.conf  nginx の設定の見本
└─ docs/仕様書.md
```

## 必要なもの

- Rust 1.85 以上（`rustup` でインストール）
- Node.js 20 以上
- （本番）nginx

## 初回セットアップ

```powershell
cd ProjectRoot
copy config\app.ini.example config\app.ini   # 必要に応じて編集
cd frontend; npm install; cd ..
```

`config/app.ini` には認証情報（SMTP のパスワードなど）を書くため、Git には登録しません。
パスワードは ini に書かず、環境変数 `APP_SMTP_PASSWORD` で渡す運用を推奨します
（すべての設定項目は `APP_<セクション>_<キー>` の環境変数で上書きできます）。

## 開発時の起動

API サーバーと画面の開発サーバーを、それぞれ `ProjectRoot` から起動します。

```powershell
# 1つ目のターミナル：API サーバー（http://127.0.0.1:8080）
cargo run --manifest-path backend/Cargo.toml

# 2つ目のターミナル：画面（http://localhost:5173、/api は API サーバーへ転送）
cd frontend; npm run dev
```

### 最初のユーザーを作る

新規登録にはメール送信（SMTP）の設定が必要です。SMTP を用意できない場合や最初の利用者は、
管理ツールで直接作れます（パスワードは入力を求められます）。

```powershell
cargo run --manifest-path backend/Cargo.toml --bin create_user -- user@example.com "ユーザー名"
```

## テスト・チェック

```powershell
cargo test  --manifest-path backend/Cargo.toml          # 単体テスト + API の結合テスト
cargo clippy --manifest-path backend/Cargo.toml --all-targets
cd frontend; npm run build; npm run lint
```

## 本番環境への配置（インターネット公開・HTTPS）

ブラウザとの暗号化（HTTPS）は nginx が受け持ち、API サーバーは PC の内部（127.0.0.1）だけで待ち受けます。
証明書は Let's Encrypt（無料、90日ごとに自動更新）を使います。

```text
ブラウザ ──HTTPS(443)──> nginx ──HTTP(PC内部のみ)──> API サーバー(127.0.0.1:8080)
```

### 事前に用意するもの

- 独自ドメイン（例：`study.example.com`）と、そのドメインをサーバーのグローバル IP に向ける DNS の設定（A レコード）
- ルーター・ファイアウォールで **80 番と 443 番だけ** をサーバーへ通す設定
  （80 番は証明書の取得・更新と https への転送に必要。**8080 番は外部に開けない**）
- 社内ネットワークから公開する場合は、情報システム部門の許可

### 1. ビルドする

```powershell
cargo build --release --manifest-path backend/Cargo.toml
cd frontend; npm ci; npm run build; cd ..
```

### 2. `config/app.ini` を本番用にする

```ini
[server]
bind = 127.0.0.1:8080
public_url = https://study.example.com
cookie_secure = true
trust_proxy = true
```

`[smtp]` を設定し、パスワードは環境変数 `APP_SMTP_PASSWORD` で渡します。

### 3. 証明書を取得する

#### Windows（win-acme）

1. [win-acme](https://www.win-acme.com/) をダウンロードし、`C:\win-acme` などに展開する
2. 証明書の確認用フォルダ（例：`C:\nginx\acme`）と、証明書の保存先（例：`C:\nginx\certs`）を作る
3. `nginx/study-record-https.conf` のドメイン名とパスを書き換え、**いったん 443 番の server ブロックをコメントにして** nginx を起動する
   （証明書がまだ無いと nginx が起動できないため。80 番だけで確認用ファイルに応答させる）
4. 管理者権限の PowerShell で証明書を取得する
   ```powershell
   C:\win-acme\wacs.exe --source manual --host study.example.com `
     --validation filesystem --webroot C:\nginx\acme `
     --store pemfiles --pemfilespath C:\nginx\certs `
     --installation script --script "C:\path\to\ProjectRoot\nginx\reload-nginx.bat"
   ```
   win-acme は更新用のタスクを Windows のタスク スケジューラに自動で登録します。
   更新のたびに `nginx\reload-nginx.bat` が nginx に新しい証明書を読み込ませます（中の `NGINX_HOME` を書き換えておく）。
   オプション名は win-acme のバージョンで変わることがあるため、`wacs.exe --help` でも確認してください。
5. `C:\nginx\certs` にできたファイル名を確認し、`ssl_certificate`（`*-chain.pem`）と
   `ssl_certificate_key`（`*-key.pem`）に指定する。443 番の server ブロックを元に戻し、`nginx -t` で確認してから `nginx -s reload`

#### Ubuntu（certbot）

```bash
sudo apt install nginx certbot libssl-dev pkg-config
sudo mkdir -p /var/www/acme
# study-record-https.conf のパスを Linux 用に書き換えて /etc/nginx/conf.d/ に置く
#   root（acme-challenge）: /var/www/acme
#   ssl_certificate     : /etc/letsencrypt/live/study.example.com/fullchain.pem
#   ssl_certificate_key : /etc/letsencrypt/live/study.example.com/privkey.pem
# （Windows と同じく、初回は 443 番の server ブロックをコメントにしてから起動する）
sudo certbot certonly --webroot -w /var/www/acme -d study.example.com \
  --deploy-hook "systemctl reload nginx"
```

certbot は自動更新のタイマーを登録します。`sudo certbot renew --dry-run` で更新を試せます。

### 4. API サーバーを常駐させる

- Windows：`ProjectRoot` をカレントフォルダにして `backend\target\release\study-record-server.exe` を起動する。
  常駐させる場合は NSSM やタスク スケジューラ（起動時に実行）を使う
- Ubuntu：systemd のサービスとして登録する（`WorkingDirectory` を `ProjectRoot` にする）

停止は Ctrl+C（Linux では SIGTERM）。処理中の要求を終えてから止まります。

### 5. 公開後の確認

- `http://study.example.com` を開くと `https://` に転送される
- `https://study.example.com/api/health` が `ok` を返す
- `https://study.example.com:8080` など、nginx 以外のポートには外部から接続できない
- [SSL Labs の診断](https://www.ssllabs.com/ssltest/) で評価が A 以上になる
- ログイン後、ブラウザの開発者ツールで Cookie `sr_session` に `Secure` `HttpOnly` `SameSite=Strict` が付いている

### 社内だけで使う場合

`nginx/study-record.conf`（http 版）を使うか、社内の認証局の証明書を `study-record-https.conf` に指定します。
2 つの設定ファイルは、どちらか一方だけを include してください（両方を読み込むと nginx が起動しません）。

## 旧システム（科目B対策アプリ）のデータ取り込み

旧 DB（`practice.sqlite3` / `practice.sqlite3.user-<ID>.sqlite3`）を、ファイルごとに取り込めます。
先に取り込み先のユーザーを作っておいてください。

```powershell
cargo run --manifest-path backend/Cargo.toml --bin import_legacy -- C:\old\practice.sqlite3 user@example.com
# 試験名を指定する場合（省略時は「基本情報技術者試験 科目B」）
cargo run --manifest-path backend/Cargo.toml --bin import_legacy -- C:\old\practice.sqlite3 user@example.com "基本情報 科目B"
```

採点済みの回答だけを取り込み、旧形式（回答に問題名称の列が無い DB）も自動で変換します。
取り込みは 1 つのトランザクションで行うため、途中で失敗しても何も変更されません。

## バックアップ

`data/` フォルダの SQLite ファイル（`-wal` / `-shm` を含む）をコピーしてください。
サーバーの動作中にコピーする場合は、SQLite の `.backup` コマンドを使うと安全です。
