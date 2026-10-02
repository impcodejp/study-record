<#
.SYNOPSIS
    API サーバーを Windows サービス「StudyRecordServer」として登録し、開始する（サーバー PC で、管理者として実行する）。

.DESCRIPTION
    release\windows-service フォルダの中で実行する。次の順に処理する。

      1. ホームフォルダ（既定 C:\study-record）を作り、exe と設定ファイルの見本をコピーする
      2. config\app.ini が無ければ見本から作り、編集を促して終了する（編集後にもう一度実行する）
      3. study-record-server.exe install でサービスとして登録する
         （自動起動・異常終了時の自動再起動・LocalService で実行・data / logs だけ書き込み可）
      4. サービスを開始し、/api/health が ok を返すか確認する

    ホームフォルダの中身：
        C:\study-record\
        ├─ study-record-server.exe / create_user.exe / import_legacy.exe
        ├─ config\app.ini            設定（認証情報を含むため管理者とサービスだけが読めるようにする）
        ├─ data\                     SQLite の DB
        └─ logs\SystemRunningLog.log ログ（このスクリプトの実行記録も追記する）

.PARAMETER HomeDir
    ホームフォルダ。省略すると C:\study-record。

.PARAMETER HealthUrl
    起動確認に使う URL。app.ini の [server] bind を変えた場合はそれに合わせる。

.EXAMPLE
    .\install-service.ps1
.EXAMPLE
    .\install-service.ps1 -HomeDir D:\study-record
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
# ホームフォルダへコピーする exe
$ExeFiles = 'study-record-server.exe', 'create_user.exe', 'import_legacy.exe'
# LocalService アカウントの SID（言語設定によらず icacls で指定できる）
$LocalServiceSid = '*S-1-5-19'
# Administrators グループの SID
$AdministratorsSid = '*S-1-5-32-544'
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
    $line = '{0} {1} install-service: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
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

# 管理者として実行されていなければ、何も変更せずに止める（ログフォルダもまだ作らない）
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '管理者として開いた PowerShell で実行してください。' -ForegroundColor Red
    exit 1
}

try {
    Write-Log "サービスの登録を開始します（ホームフォルダ: $HomeDir）"

    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        throw "サービス「$ServiceName」はすでに登録されています。exe を入れ替える場合は update-service.ps1 を使ってください"
    }

    # 1. ホームフォルダへ exe と見本をコピーする
    New-Item -ItemType Directory -Force -Path (Join-Path $HomeDir 'config') | Out-Null
    foreach ($exe in $ExeFiles) {
        Copy-Item (Join-Path $PSScriptRoot $exe) $HomeDir -Force
    }
    Copy-Item (Join-Path $PSScriptRoot 'config\app.ini.example') (Join-Path $HomeDir 'config') -Force
    Write-Log 'exe と設定ファイルの見本をホームフォルダへコピーしました'

    # 2. 設定ファイルが無ければ見本から作り、編集してもらう
    $iniPath = Join-Path $HomeDir 'config\app.ini'
    if (-not (Test-Path $iniPath)) {
        Copy-Item (Join-Path $HomeDir 'config\app.ini.example') $iniPath
        # 認証情報を書くため、管理者（変更可）とサービス（読み取りのみ）以外は読めないようにする
        icacls $iniPath /inheritance:r /grant "${AdministratorsSid}:F" "${LocalServiceSid}:R" | Out-Null
        Write-Log "設定ファイルを作成しました: $iniPath" 'WARN'
        Write-Log '[server] の public_url / cookie_secure / trust_proxy と [smtp] を本番用に編集してから、もう一度このスクリプトを実行してください' 'WARN'
        exit 0
    }

    # 3. サービスとして登録する（data / logs の作成と権限の設定も exe が行う）
    $serverExe = Join-Path $HomeDir 'study-record-server.exe'
    & $serverExe install --home $HomeDir
    if ($LASTEXITCODE -ne 0) { throw "サービスの登録に失敗しました（終了コード $LASTEXITCODE）" }
    # このスクリプトが先に作ったログファイルにも、サービスが書き込めるようにする
    icacls $LogPath /grant "${LocalServiceSid}:M" | Out-Null
    Write-Log "サービス「$ServiceName」を登録しました"

    # 4. 開始して稼働を確認する
    Start-Service -Name $ServiceName
    if (Test-Health -Url $HealthUrl) {
        Write-Log "サービスを開始しました（$HealthUrl が ok を返しました）"
    }
    else {
        Write-Log "サービスは開始しましたが、$HealthUrl から応答がありません。$LogPath を確認してください" 'WARN'
    }
}
catch {
    Write-Log "サービスの登録に失敗しました: $($_.Exception.Message)" 'ERROR'
    exit 1
}
