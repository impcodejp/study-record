<#
.SYNOPSIS
    本番用にビルドし、サーバー PC へ持っていく release フォルダを作る（開発 PC で実行する）。

.DESCRIPTION
    API サーバー（Rust）と画面（React）をビルドし、置き場所ごとに分けて release フォルダへまとめる。

        release\
        ├─ windows-service\   Windows サービス（API サーバー）で使うもの
        │   ├─ study-record-server.exe     サービス本体
        │   ├─ create_user.exe / import_legacy.exe   管理ツール
        │   ├─ config\app.ini.example      設定ファイルの見本
        │   └─ install-service.ps1 など    サービスの登録・更新・解除
        └─ nginx\             nginx で使うもの
            ├─ html\                       画面（nginx が配信する）
            ├─ conf\                       nginx の設定（http 版・HTTPS 版）
            ├─ reload-nginx.bat            証明書の更新後に nginx を再読み込みする
            └─ install-nginx.ps1           nginx への配置

    release フォルダをまるごとサーバー PC へコピーし、
    windows-service\install-service.ps1 と nginx\install-nginx.ps1 を実行する（deploy\README.md を参照）。
    実行の記録は ProjectRoot\logs\SystemRunningLog.log に追記する。

.PARAMETER OutDir
    出力先のフォルダ。省略すると ProjectRoot\release。中身は毎回作り直す。

.EXAMPLE
    .\deploy\build-release.ps1
#>
[CmdletBinding()]
param(
    [string]$OutDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ProjectRoot（このスクリプトの 1 つ上のフォルダ）
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutDir) { $OutDir = Join-Path $ProjectRoot 'release' }
$LogPath = Join-Path $ProjectRoot 'logs\SystemRunningLog.log'

<#
.SYNOPSIS
    画面とログファイル（SystemRunningLog.log）の両方にメッセージを出す。
#>
function Write-Log {
    param(
        [Parameter(Mandatory)][string]$Message,
        [ValidateSet('INFO', 'WARN', 'ERROR')][string]$Level = 'INFO'
    )
    $line = '{0} {1} build-release: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
    switch ($Level) {
        'ERROR' { Write-Host $line -ForegroundColor Red }
        'WARN'  { Write-Host $line -ForegroundColor Yellow }
        default { Write-Host $line }
    }
    $dir = Split-Path -Parent $LogPath
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    # アプリのログと同じく BOM なしの UTF-8 で追記する
    # サービスが書き込み中でも追記できるよう、共有を許可して開く（ローテーションの名前変更も妨げない）
    $share = [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete
    $stream = New-Object System.IO.FileStream($LogPath, [System.IO.FileMode]::Append, [System.IO.FileAccess]::Write, $share)
    try {
        $bytes = (New-Object System.Text.UTF8Encoding $false).GetBytes("$line`n")
        $stream.Write($bytes, 0, $bytes.Length)
    }
    finally {
        $stream.Dispose()
    }
}

<#
.SYNOPSIS
    外部コマンドを実行し、失敗したら例外にする。
#>
function Invoke-Step {
    param(
        [Parameter(Mandatory)][string]$Description,
        [Parameter(Mandatory)][scriptblock]$Command
    )
    Write-Log "$Description を開始します"
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Description に失敗しました（終了コード $LASTEXITCODE）" }
    Write-Log "$Description が完了しました"
}

try {
    Write-Log "release フォルダの作成を開始します（出力先: $OutDir）"

    # 1. API サーバー（exe）をビルドする
    Invoke-Step 'API サーバーのビルド（cargo build --release）' {
        cargo build --release --bins --manifest-path (Join-Path $ProjectRoot 'backend\Cargo.toml')
    }

    # 2. 画面をビルドする（frontend\dist にできる）
    $frontend = Join-Path $ProjectRoot 'frontend'
    Push-Location $frontend
    try {
        Invoke-Step '画面の依存パッケージのインストール（npm ci）' { npm ci }
        Invoke-Step '画面のビルド（npm run build）' { npm run build }
    }
    finally {
        Pop-Location
    }

    # 3. 出力先を作り直す
    if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
    $serviceDir = Join-Path $OutDir 'windows-service'
    $nginxDir = Join-Path $OutDir 'nginx'
    New-Item -ItemType Directory -Force -Path (Join-Path $serviceDir 'config') | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $nginxDir 'conf') | Out-Null

    # 4. Windows サービス用のファイルを集める
    $targetDir = Join-Path $ProjectRoot 'backend\target\release'
    foreach ($exe in 'study-record-server.exe', 'create_user.exe', 'import_legacy.exe') {
        Copy-Item (Join-Path $targetDir $exe) $serviceDir
    }
    Copy-Item (Join-Path $ProjectRoot 'config\app.ini.example') (Join-Path $serviceDir 'config')
    Copy-Item (Join-Path $PSScriptRoot 'windows-service\*.ps1') $serviceDir
    Write-Log "Windows サービス用のファイルをまとめました: $serviceDir"

    # 5. nginx 用のファイルを集める
    Copy-Item (Join-Path $frontend 'dist') (Join-Path $nginxDir 'html') -Recurse
    Copy-Item (Join-Path $PSScriptRoot 'nginx\*.conf') (Join-Path $nginxDir 'conf')
    Copy-Item (Join-Path $PSScriptRoot 'nginx\reload-nginx.bat') $nginxDir
    Copy-Item (Join-Path $PSScriptRoot 'nginx\install-nginx.ps1') $nginxDir
    Write-Log "nginx 用のファイルをまとめました: $nginxDir"

    Write-Log "release フォルダの作成が完了しました。フォルダごとサーバー PC へコピーしてください: $OutDir"
}
catch {
    Write-Log "release フォルダの作成に失敗しました: $($_.Exception.Message)" 'ERROR'
    exit 1
}
