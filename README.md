# 資格勉強 回答正誤記録アプリ

資格試験の問題集を解きながら回答を記録し、自己採点して学習状況を振り返る Web アプリです。
複数の資格試験を登録でき、学習・カテゴリ・集計は試験ごとに分かれます。

記録するだけでなく、**今日なにを解けばよいか** を示します。

- **今日の復習**：問題ごとに習熟度を計算し、忘れかけた頃（1→3→7→14→30→60日後）に復習を促す（間隔反復）
- **まとめて復習**：今日の復習の問題を 1 回の学習で順番に解ける。問題名称・問題数・カテゴリは 1 問ずつ自動で入る
- **今日やること**：試験日までの残り日数・1 日の目標の達成度・連続学習日数をダッシュボードの先頭に表示する
- **合格準備度とペース**：習熟度から試験の準備度（全体・カテゴリ別）を出し、試験日までに必要な 1 日の問題数と今のペースを比べる
- **見直しノート**：採点時に残した正解・メモを、キーワード・カテゴリ・正誤で絞り込んで読み返せる
- **学習カレンダー・苦手なカテゴリ・時間のかかる問題**：学習した日、正答率の低いカテゴリ、平均回答時間の長い問題が一目でわかる
- **解き直す**：問題一覧・復習・見直しノートから、問題名称・問題数・カテゴリを入れた状態ですぐに学習を始められる
- **すぐ始められる**：主な資格（IT パスポート・基本情報・応用情報・簿記・FP・宅建・TOEIC）のテンプレートを選ぶだけで、出題分野のカテゴリがそろう
- **続けたくなる**：準備度の推移グラフ、実績バッジ。スマートフォンのホーム画面に追加してアプリのように使える（PWA）

ログイン前のトップ（`/`）は、サービスの紹介ページです。検索エンジン向けに、ビルド時に HTML として書き出します（`frontend/scripts/prerender.mjs`）。

| 構成 | 内容 |
|---|---|
| フロントエンド | React 19 + TypeScript + Vite（`frontend/`） |
| API サーバー | Rust + Axum（`backend/`） |
| データベース | SQLite（`data/study_record.sqlite3`） |
| Web サーバー | nginx（設定は `deploy/nginx/`） |
| 本番での常時起動 | API サーバーは Windows サービス、nginx はタスク スケジューラ |
| ログ | `logs/SystemRunningLog.log`（約 2MB で `SystemRunningLog1.log` に切り替え） |

- **本番環境への配置（Windows サービス + nginx）は [deploy/README.md](deploy/README.md) を参照してください。**
- 詳しい仕様・設計判断は [docs/仕様書.md](docs/仕様書.md) を参照してください。

このファイルには、開発 PC での準備・起動・テストの手順をまとめています。

## フォルダ構成

```text
ProjectRoot/
├─ backend/                 API サーバー（Rust）
│  ├─ src/domain/           業務ルール・モデル（他の層に依存しない）
│  ├─ src/application/      ユースケース（トランザクションの範囲を決める）
│  ├─ src/infrastructure/   DB・メール・設定・ログ
│  ├─ src/presentation/     HTTP API（入出力の変換だけ）
│  ├─ src/bin/              管理ツール（create_user / import_legacy）
│  ├─ src/service_host.rs   Windows サービスとしての登録・起動・停止
│  └─ tests/                API の結合テスト
├─ frontend/                画面（React）
│  └─ src/{api,hooks,components,pages,utils}
├─ config/app.ini.example   設定ファイルの見本
├─ deploy/                  本番環境への配置に使うもの（手順は deploy/README.md）
│  ├─ build-release.ps1     ビルドして release フォルダを作る（開発 PC で実行）
│  ├─ windows-service/      【Windows サービス用】API サーバーの登録・更新・解除のスクリプト
│  ├─ nginx/                【nginx 用】nginx の設定（http 版・HTTPS 版）と配置スクリプト
│  └─ ubuntu/               将来 Ubuntu へ移行するときの systemd 設定の見本
├─ docs/仕様書.md
└─ release/                 build-release.ps1 の出力（Git には登録しない）
```

## 必要なもの（開発 PC）

- Rust 1.85 以上（`rustup` でインストール）
- Node.js 20 以上

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

## 本番環境への配置

サーバー PC（Windows 11）では、API サーバーを **Windows サービス**、nginx を **タスク スケジューラ** で常時起動します。
手順は [deploy/README.md](deploy/README.md) にまとめています。概要は次のとおりです。

1. 開発 PC で `deploy\build-release.ps1` を実行し、`release` フォルダ（`windows-service\` と `nginx\` に分かれる）を作る
2. `release` をサーバー PC へコピーし、管理者の PowerShell で `windows-service\install-service.ps1` を実行する
3. 同じく `nginx\install-nginx.ps1` を実行する（インターネット公開時は証明書を取得してから `-Https` を付けて再実行）

## 旧システム（科目B対策アプリ）のデータ取り込み

旧 DB（`practice.sqlite3` / `practice.sqlite3.user-<ID>.sqlite3`）を、ファイルごとに取り込めます。
先に取り込み先のユーザーを作っておいてください。
本番のサーバー PC では、`C:\study-record` で `import_legacy.exe` を直接実行します（同じ `config\app.ini` を使うため）。

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
