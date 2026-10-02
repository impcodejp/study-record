<#
.SYNOPSIS
    登録済みの Windows サービス「StudyRecordServer」の exe を新しい版に入れ替える（サーバー PC で、管理者として実行する）。

.DESCRIPTION
    新しい release\windows-service フォルダの中で実行する。次の順に処理する。

      1. サービスを停止する（処理中の要求を終えてから止まる）
      2. 入れ替える前の exe を backup フォルダへ退避し、新しい exe をホームフォルダへコピーする
      3. サービスを開始し、/api/health が ok を返すか確認する

    config\app.ini・data・logs には触れない。DB の構造の変更は、サービスの起動時に自動で反映される。

.PARAMETER HomeDir
    ホームフォルダ。install-service.ps1 で指定したものと同じにする。省略すると C:\study-record。

.PARAMETER HealthUrl
    起動確認に使う URL。app.ini の [server] bind を変えた場合はそれに合わせる。

.EXAMPLE
    .\update-service.ps1
#>
[CmdletBinding()]
param(
    [string]$HomeDir = 'C:\study-record',
    [string]$HealthUrl = 'http://127.0.0.1:8080/api/health'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# サービス名（study-record-server.exe の service_host.rs と同じ）
$ServiceName = 'StudyRecordServer'
# 入れ替える exe
$ExeFiles = 'study-record-server.exe', 'create_user.exe', 'import_legacy.exe'
$LogPath = Join-Path $HomeDir 'logs\SystemRunningLog.log'

<#
.SYNOPSIS
    画面とログファイル（SystemRunningLog.log）の両方にメッセージを出す。
#>
function Write-Log {
    param(
        [Parameter(Mandatory)][string]$Message,
        [ValidateSet('INFO', 'WARN', 'ERROR')][string]$Level = 'INFO'
    )
    $line = '{0} {1} update-service: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
    switch ($Level) {
        'ERROR' { Write-Host $line -ForegroundColor Red }
        'WARN'  { Write-Host $line -ForegroundColor Yellow }
        default { Write-Host $line }
    }
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
    /api/health が ok を返すまで最大 30 秒待つ。
#>
function Test-Health {
    param([Parameter(Mandatory)][string]$Url)
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2
            if ($response.Content -eq 'ok') { return $true }
        }
        catch {
            # 起動中は接続できないため、少し待って再試行する
        }
        Start-Sleep -Seconds 1
    }
    return $false
}

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '管理者として開いた PowerShell で実行してください。' -ForegroundColor Red
    exit 1
}
if (-not (Test-Path (Join-Path $HomeDir 'logs'))) {
    Write-Host "$HomeDir にサービスが見つかりません。先に install-service.ps1 を実行してください。" -ForegroundColor Red
    exit 1
}

try {
    Write-Log "exe の入れ替えを開始します（ホームフォルダ: $HomeDir）"
    if (-not (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) {
        throw "サービス「$ServiceName」が登録されていません。先に install-service.ps1 を実行してください"
    }

    # 1. 停止する
    Stop-Service -Name $ServiceName
    Write-Log 'サービスを停止しました'

    # 2. 旧版を退避してから新しい exe をコピーする
    $backupDir = Join-Path $HomeDir ('backup\' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
    New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
    foreach ($exe in $ExeFiles) {
        $current = Join-Path $HomeDir $exe
        if (Test-Path $current) { Copy-Item $current $backupDir }
        Copy-Item (Join-Path $PSScriptRoot $exe) $HomeDir -Force
    }
    Copy-Item (Join-Path $PSScriptRoot 'config\app.ini.example') (Join-Path $HomeDir 'config') -Force
    Write-Log "exe を入れ替えました（旧版の退避先: $backupDir）"

    # 3. 開始して稼働を確認する
    Start-Service -Name $ServiceName
    if (Test-Health -Url $HealthUrl) {
        Write-Log "サービスを開始しました（$HealthUrl が ok を返しました）"
    }
    else {
        Write-Log "サービスは開始しましたが、$HealthUrl から応答がありません。$LogPath を確認してください（旧版に戻す場合は $backupDir の exe を戻す）" 'WARN'
    }
}
catch {
    Write-Log "exe の入れ替えに失敗しました: $($_.Exception.Message)" 'ERROR'
    exit 1
}
