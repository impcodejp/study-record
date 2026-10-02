# 本番環境への配置手順（Windows サービス + nginx）

サーバー PC（Windows 11）に、このアプリを常時起動の状態で配置する手順です。
**このファイルの手順を上から順に行えば配置できます。**

## 全体像

本番では 2 つのプログラムが動きます。どちらも Windows の起動時に自動で起動します。

```text
ブラウザ ──http(80) / https(443)──> nginx ──http(PC内部のみ)──> API サーバー（127.0.0.1:8080）
                                     │                           │
                                     └─ 画面（html）を配信        └─ DB（SQLite）・ログ
```

| | API サーバー | nginx |
|---|---|---|
| 役割 | 回答の記録などの処理（`/api/...`） | 画面の配信、`/api` の転送、HTTPS の暗号化 |
| 置き場所（既定） | `C:\study-record\` | `C:\nginx\` |
| 常時起動のしかた | **Windows サービス**「StudyRecordServer」 | **タスク スケジューラ**「StudyRecordNginx」（nginx の Windows 版はサービス登録の機能を持たないため） |
| 実行アカウント | LocalService（権限の低い組み込みアカウント） | LocalService |
| 配置するスクリプト | `windows-service\install-service.ps1` | `nginx\install-nginx.ps1` |
| ログ | `C:\study-record\logs\SystemRunningLog.log` | `C:\nginx\logs\study-record.*.log` |

## このフォルダ（deploy）の中身

ファイルは **どこで使うか** でフォルダを分けています。

```text
deploy/
├─ README.md                    この手順書
├─ build-release.ps1            【開発 PC】ビルドして release フォルダを作る
├─ windows-service/             【Windows サービス用】API サーバーの登録・更新・解除
│  ├─ install-service.ps1         初回の登録と開始
│  ├─ update-service.ps1          新しい版への入れ替え
│  └─ uninstall-service.ps1       登録の解除
├─ nginx/                       【nginx 用】nginx で使うものはすべてここ
│  ├─ install-nginx.ps1           画面と設定を nginx へ配置し、自動起動を登録する
│  ├─ study-record-http.conf      nginx の設定（http 版・社内利用向け）
│  ├─ study-record-https.conf     nginx の設定（HTTPS 版・インターネット公開向け）
│  └─ reload-nginx.bat            証明書の更新後に nginx を再読み込みする（win-acme から呼ばれる）
└─ ubuntu/
   └─ study-record.service      将来 Ubuntu へ移行するときの systemd 設定の見本（Windows では使わない）
```

`build-release.ps1` を実行すると、サーバー PC へ持っていくものが同じ分け方でまとまります。

```text
release/
├─ windows-service/   → install-service.ps1 が C:\study-record\ へコピーする
│  ├─ study-record-server.exe / create_user.exe / import_legacy.exe
│  ├─ config\app.ini.example
│  └─ install-service.ps1 / update-service.ps1 / uninstall-service.ps1
└─ nginx/             → install-nginx.ps1 が C:\nginx\ へコピーする
   ├─ html\                       画面（frontend\dist）   → C:\nginx\html\study-record\
   ├─ conf\                       nginx の設定            → C:\nginx\conf\study-record.conf
   ├─ reload-nginx.bat                                    → C:\nginx\reload-nginx.bat
   └─ install-nginx.ps1
```

## 手順

### 1. 【開発 PC】ビルドする

Rust と Node.js が入っている PC で、`ProjectRoot` から実行します。

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\build-release.ps1
```

`ProjectRoot\release` ができます。**`release` フォルダをまるごと** サーバー PC へコピーしてください（例：`C:\work\release`）。

> サーバー PC には Rust・Node.js・Visual C++ 再頒布可能パッケージのどれも不要です（exe は単体で動きます）。

### 2. 【サーバー PC】nginx を用意する

1. [nginx の公式サイト](https://nginx.org/en/download.html) から Windows 版（Stable version）をダウンロードし、`C:\nginx` に展開する
   （`C:\nginx\nginx.exe` がある状態にする）
2. `C:\nginx\conf\nginx.conf` を編集する
   - `http { }` の中（閉じかっこ `}` の直前）に、次の 1 行を足す
     ```nginx
     include study-record.conf;
     ```
   - 最初から書かれている `server { listen 80; ... }` のブロックは、ポートが重なるため削除するかコメント（`#`）にする

### 3. 【サーバー PC】API サーバーを Windows サービスとして登録する

**管理者として開いた** PowerShell で実行します。

```powershell
cd C:\work\release\windows-service
powershell -ExecutionPolicy Bypass -File .\install-service.ps1
```

1 回目は `C:\study-record\config\app.ini` を作って止まります。メモ帳を **管理者として** 開いて編集してください
（認証情報を書くファイルのため、管理者とサービスだけが読めるようにしてあります）。

```ini
[server]
bind = 127.0.0.1:8080
; 利用者がブラウザで開く URL（メールのリンクに使う）
public_url = https://study.example.com
; HTTPS で運用するなら true（http 版のままなら false）
cookie_secure = true
; nginx の背後に置くので true
trust_proxy = true
```

`[smtp]` も設定します（新規登録・パスワード再設定のメールに使う。不要なら空のままで可）。
編集したら、**もう一度同じコマンドを実行** します。サービスが登録・開始され、`/api/health` の確認まで自動で行います。

登録されるサービスの内容：

- サービス名 `StudyRecordServer`、Windows の起動時に自動で開始する
- 権限の低い組み込みアカウント **LocalService** で動かす（書き込み権限は `data` と `logs` だけ）
- 異常終了したら 10 秒後に自動で再起動する（3 回まで。24 時間で回数を数え直す）

### 4. 【サーバー PC】最初のユーザーを作る

```powershell
cd C:\study-record
.\create_user.exe user@example.com "ユーザー名"
```

パスワードの入力を求められます。旧システムのデータを取り込む場合は、ここで `import_legacy.exe` も実行します（ProjectRoot の README を参照）。

### 5. 【サーバー PC】nginx へ配置する

**管理者として開いた** PowerShell で実行します。

```powershell
cd C:\work\release\nginx
powershell -ExecutionPolicy Bypass -File .\install-nginx.ps1
```

画面と設定（http 版）をコピーし、設定を確認（`nginx -t`）してから、
Windows の起動時に nginx を起動するタスク `StudyRecordNginx` を登録して nginx を起動します。
nginx のフォルダが `C:\nginx` でない場合は `-NginxHome D:\nginx` のように指定します。

ここで `http://localhost/` を開いて画面が出れば、社内利用の場合は完了です。
インターネットに公開する場合は、続けて手順 6 を行います。

### 6. 【インターネット公開のときだけ】HTTPS にする

#### 事前に用意するもの

- 独自ドメイン（例：`study.example.com`）と、そのドメインをサーバーのグローバル IP に向ける DNS の設定（A レコード）
- ルーター・ファイアウォールで **80 番と 443 番だけ** をサーバーへ通す設定
  （80 番は証明書の取得・更新と https への転送に必要。**8080 番は外部に開けない**）
- 社内ネットワークから公開する場合は、情報システム部門の許可

#### 証明書を取得する（win-acme）

証明書は Let's Encrypt（無料、90日ごとに自動更新）を使います。手順 5 の http 版のまま取得します。

1. [win-acme](https://www.win-acme.com/) をダウンロードし、`C:\win-acme` に展開する
2. 管理者権限の PowerShell で取得する
   ```powershell
   C:\win-acme\wacs.exe --source manual --host study.example.com `
     --validation filesystem --webroot C:\nginx\acme `
     --store pemfiles --pemfilespath C:\nginx\certs `
     --installation script --script "C:\nginx\reload-nginx.bat"
   ```
   win-acme は更新用のタスクをタスク スケジューラに自動で登録し、更新のたびに `reload-nginx.bat` で nginx に新しい証明書を読み込ませます。
   オプション名は win-acme のバージョンで変わることがあるため、`wacs.exe --help` でも確認してください。
3. `C:\nginx\certs` に `study.example.com-chain.pem` と `study.example.com-key.pem` ができたことを確認する
   （名前が違う場合は、`release\nginx\conf\study-record-https.conf` の `ssl_certificate` / `ssl_certificate_key` を合わせる）

#### HTTPS 版に切り替える

```powershell
cd C:\work\release\nginx
powershell -ExecutionPolicy Bypass -File .\install-nginx.ps1 -Https -ServerName study.example.com
```

`-ServerName` は、検索エンジン向けのファイル（紹介ページの正規の URL・SNS 共有の情報・`robots.txt`・`sitemap.xml`）のドメインにも使われます。公開後は [Google Search Console](https://search.google.com/search-console) にドメインを登録し、`https://study.example.com/sitemap.xml` を送信すると、検索結果に早く載ります。

`app.ini` の `public_url = https://...` と `cookie_secure = true` も確認し、変えた場合はサービスを再起動します（`Restart-Service StudyRecordServer`）。

### 7. 公開後の確認

- `https://study.example.com/api/health` が `ok` を返す
- `http://study.example.com` を開くと `https://` に転送される
- `https://study.example.com:8080` など、nginx 以外のポートには外部から接続できない
- [SSL Labs の診断](https://www.ssllabs.com/ssltest/) で評価が A 以上になる
- ログイン後、ブラウザの開発者ツールで Cookie `sr_session` に `Secure` `HttpOnly` `SameSite=Strict` が付いている

## 日常の運用

| やりたいこと | 方法 |
|---|---|
| 新しい版に入れ替える | 開発 PC で手順 1 → サーバー PC の新しい `release` で、管理者の PowerShell から `windows-service\update-service.ps1`（停止 → 旧版を `backup\` へ退避 → 入れ替え → 開始）と `nginx\install-nginx.ps1`（HTTPS なら `-Https -ServerName ...` も付ける） |
| API サーバーの停止・開始・状態確認 | `Stop-Service StudyRecordServer` / `Start-Service StudyRecordServer` / `Get-Service StudyRecordServer`（「サービス」画面 services.msc からも可） |
| nginx の再起動 | `C:\nginx` で `.\nginx.exe -s stop` の後、`Start-ScheduledTask StudyRecordNginx` |
| 動かないときに見るログ | API サーバー：`C:\study-record\logs\SystemRunningLog.log`（配置スクリプトの実行記録も入る） / nginx：`C:\nginx\logs\error.log`、`study-record.error.log` |
| バックアップ | `C:\study-record\data\` の SQLite ファイル（`-wal` / `-shm` を含む）をコピーする。動作中にコピーする場合は SQLite の `.backup` コマンドを使う |
| 登録を解除する | `windows-service\uninstall-service.ps1`（データとログは残る）。nginx の自動起動は `Unregister-ScheduledTask StudyRecordNginx` |

### SMTP のパスワードを app.ini に書きたくない場合

サービス専用の環境変数として登録できます（すべての設定項目は `APP_<セクション>_<キー>` の環境変数で上書きできます）。

```powershell
reg add HKLM\SYSTEM\CurrentControlSet\Services\StudyRecordServer /v Environment /t REG_MULTI_SZ /d "APP_SMTP_PASSWORD=ここにパスワード" /f
Restart-Service StudyRecordServer
```

## スクリプトを使わずに登録する場合

スクリプトは、exe 自身が持つ次のコマンドを呼んでいるだけです（中身は `backend/src/service_host.rs`）。

```powershell
cd C:\study-record
.\study-record-server.exe install                       # 登録（ホームフォルダ＝exe のあるフォルダ）
.\study-record-server.exe install --home D:\study-data  # ホームフォルダを別にする場合
.\study-record-server.exe uninstall                     # 解除
.\study-record-server.exe                               # サービスにせず、コンソールで起動する（Ctrl+C で停止）
```

## 付録：Ubuntu へ移行する場合

`ubuntu/study-record.service` を使います（手順はファイルの先頭に記載）。
ホームフォルダは `/opt/study-record`、SMTP のパスワードは `/etc/study-record/env`（root だけが読めるファイル）に
`APP_SMTP_PASSWORD=...` の形で書きます。停止時は SIGTERM を受けて、処理中の要求を終えてから止まります。

nginx の設定は `nginx/study-record-https.conf` を `/etc/nginx/conf.d/` に置き、パスを Linux 用に書き換えます。
証明書は certbot で取得します。

```bash
sudo apt install nginx certbot
sudo mkdir -p /var/www/acme
#   root（acme-challenge）: /var/www/acme
#   ssl_certificate     : /etc/letsencrypt/live/study.example.com/fullchain.pem
#   ssl_certificate_key : /etc/letsencrypt/live/study.example.com/privkey.pem
# （初回は 443 番の server ブロックをコメントにしてから起動する）
sudo certbot certonly --webroot -w /var/www/acme -d study.example.com \
  --deploy-hook "systemctl reload nginx"
```
