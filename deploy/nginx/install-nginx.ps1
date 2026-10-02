<#
.SYNOPSIS
    【nginx 用】画面と nginx の設定を nginx のフォルダへ配置し、nginx に読み込ませる（サーバー PC で、管理者として実行する）。

.DESCRIPTION
    release\nginx フォルダの中で実行する。次の順に処理する。

      1. html フォルダ（画面）を <NginxHome>\html\study-record へコピーする（前の版は消して入れ替える）
      2. http 版または HTTPS 版の設定を <NginxHome>\conf\study-record.conf としてコピーする
         （どちらを選んでも同じファイル名になるため、両方が読み込まれることはない）
         その際、設定の中の「C:/nginx/」と「study.example.com」を実際の値に書き換える
      3. conf\nginx.conf に「include study-record.conf;」が無ければ、足す場所を案内して終了する
      4. nginx -t で設定を確認する
      5. 初回だけ、Windows の起動時に nginx を起動するタスク「StudyRecordNginx」を登録する
         （nginx の Windows 版はサービスとして登録する機能を持たないため、タスク スケジューラを使う）
      6. nginx が動いていれば再読み込みし、動いていなければ起動する

    実行の記録は API サーバーと同じ SystemRunningLog.log に追記する（-LogPath で変更できる）。

.PARAMETER NginxHome
    nginx.exe のあるフォルダ。省略すると C:\nginx。

.PARAMETER Https
    指定すると HTTPS 版（study-record-https.conf）を使う。省略すると http 版（study-record-http.conf）。

.PARAMETER ServerName
    公開するドメイン名。-Https のときは必須。設定ファイルの study.example.com と、
    検索エンジン向けのファイル（紹介ページの正規の URL、robots.txt、sitemap.xml）のドメインを置き換える。

.PARAMETER LogPath
    ログファイル。省略すると C:\study-record\logs\SystemRunningLog.log（API サーバーのログと同じ）。

.EXAMPLE
    .\install-nginx.ps1
.EXAMPLE
    .\install-nginx.ps1 -Https -ServerName study.example.com
#>
[CmdletBinding()]
param(
    [string]$NginxHome = 'C:\nginx',
    [switch]$Https,
    [string]$ServerName,
    [string]$LogPath = 'C:\study-record\logs\SystemRunningLog.log'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# nginx の conf フォルダに置く設定ファイルの名前（http 版・HTTPS 版で共通）
$ConfName = 'study-record.conf'
# 設定の見本に書かれている、置き換え前の値
$PlaceholderHome = 'C:/nginx/'
$PlaceholderServerName = 'study.example.com'
# nginx を Windows の起動時に起動するタスクの名前
$TaskName = 'StudyRecordNginx'
# LocalService アカウントの SID（言語設定によらず icacls で指定できる）
$LocalServiceSid = '*S-1-5-19'

<#
.SYNOPSIS
    画面とログファイル（SystemRunningLog.log）の両方にメッセージを出す。
#>
function Write-Log {
    param(
        [Parameter(Mandatory)][string]$Message,
        [ValidateSet('INFO', 'WARN', 'ERROR')][string]$Level = 'INFO'
    )
    $line = '{0} {1} install-nginx: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
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
    nginx.exe を nginx のフォルダを基準にして実行し、出力をログに残す。終了コードを返す。
#>
function Invoke-Nginx {
    param([Parameter(Mandatory)][string[]]$Arguments)
    Push-Location $NginxHome
    try {
        # nginx は確認結果を標準エラー出力に書くため、まとめて受け取る
        $output = & cmd /c "nginx.exe $($Arguments -join ' ') 2>&1"
        foreach ($text in $output) { Write-Log "nginx: $text" }
        return $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '管理者として開いた PowerShell で実行してください。' -ForegroundColor Red
    exit 1
}

try {
    $mode = if ($Https) { 'HTTPS 版' } else { 'http 版' }
    Write-Log "nginx への配置を開始します（nginx: $NginxHome、設定: $mode）"

    if (-not (Test-Path (Join-Path $NginxHome 'nginx.exe'))) {
        throw "$NginxHome に nginx.exe が見つかりません。-NginxHome を指定してください"
    }
    if ($Https -and -not $ServerName) {
        throw '-Https のときは -ServerName（公開するドメイン名）を指定してください'
    }

    # 1. 画面を入れ替える
    $htmlDest = Join-Path $NginxHome 'html\study-record'
    if (Test-Path $htmlDest) { Remove-Item -Recurse -Force $htmlDest }
    Copy-Item (Join-Path $PSScriptRoot 'html') $htmlDest -Recurse
    Write-Log "画面をコピーしました: $htmlDest"

    # 検索エンジン向けのファイル（紹介ページの正規の URL・SNS 向けの情報、robots.txt、sitemap.xml）は、
    # ビルド時に仮のドメインで作られているため、公開するドメインに書き換える。
    $seoFiles = 'welcome\index.html', 'robots.txt', 'sitemap.xml'
    if ($ServerName) {
        $scheme = if ($Https) { 'https' } else { 'http' }
        foreach ($file in $seoFiles) {
            $target = Join-Path $htmlDest $file
            if (-not (Test-Path $target)) { continue }
            $text = [System.IO.File]::ReadAllText($target, [System.Text.Encoding]::UTF8)
            $text = $text.Replace("https://$PlaceholderServerName", "${scheme}://$ServerName")
            [System.IO.File]::WriteAllText($target, $text, (New-Object System.Text.UTF8Encoding $false))
        }
        Write-Log "検索エンジン向けのファイルの URL を ${scheme}://$ServerName にしました"
    }
    else {
        Write-Log "-ServerName が無いため、検索エンジン向けのファイルの URL は仮のドメイン（$PlaceholderServerName）のままです（社内だけで使う場合は問題ありません）" 'WARN'
    }

    # 2. 設定ファイルを置き換えてコピーする
    $source = if ($Https) { 'study-record-https.conf' } else { 'study-record-http.conf' }
    $content = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot "conf\$source"))
    $homeForNginx = (($NginxHome -replace '\\', '/').TrimEnd('/')) + '/'
    $content = $content.Replace($PlaceholderHome, $homeForNginx)
    if ($ServerName) { $content = $content.Replace($PlaceholderServerName, $ServerName) }

    if ($Https) {
        # 証明書が無いと nginx が起動できないため、設定を書き換える前に確認する
        $certPattern = '(?m)^\s*ssl_certificate(?:_key)?\s+([^;]+);'
        foreach ($match in [regex]::Matches($content, $certPattern)) {
            $cert = $match.Groups[1].Value.Trim()
            if (-not (Test-Path $cert)) {
                throw "証明書のファイル $cert がありません。先に http 版で配置して証明書を取得するか、conf\$source の ssl_certificate のファイル名を win-acme が作ったものに合わせてください"
            }
        }
    }

    # 確認（nginx -t）に失敗したときに戻せるよう、今の設定を退避してから書き込む
    $confDest = Join-Path $NginxHome "conf\$ConfName"
    $confBackup = "$confDest.bak"
    $hadConf = Test-Path $confDest
    if ($hadConf) { Copy-Item $confDest $confBackup -Force }
    [System.IO.File]::WriteAllText($confDest, $content, (New-Object System.Text.UTF8Encoding $false))
    Write-Log "設定ファイルをコピーしました: $confDest（元: $source）"

    # 証明書の確認用フォルダ（win-acme の --webroot に指定する）と保存先を用意しておく
    # （http 版のうちに証明書を取得し、その後 HTTPS 版へ切り替えるため、どちらの版でも作る）
    foreach ($dir in 'acme', 'certs') {
        New-Item -ItemType Directory -Force -Path (Join-Path $NginxHome $dir) | Out-Null
    }
    # 証明書の更新後に win-acme から呼ばれるスクリプト
    Copy-Item (Join-Path $PSScriptRoot 'reload-nginx.bat') $NginxHome -Force

    # 3. nginx.conf から読み込まれているか確認する（nginx.conf は利用者の設定なので自動では書き換えない）
    $mainConf = Join-Path $NginxHome 'conf\nginx.conf'
    if (-not (Select-String -Path $mainConf -Pattern "^\s*include\s+$([regex]::Escape($ConfName))\s*;" -Quiet)) {
        Write-Log "$mainConf の http { } の中に、次の 1 行を足してからもう一度実行してください: include $ConfName;" 'WARN'
        Write-Log '（nginx.conf に最初からある「listen 80」の server { } ブロックは、ポートが重なるため削除するかコメントにしてください）' 'WARN'
        exit 0
    }

    # 4. 設定を確認する（誤りがあれば、動いている nginx には触れずに止める）
    if ((Invoke-Nginx -Arguments '-t') -ne 0) {
        # 次回の起動で失敗しないよう、設定を元に戻す
        if ($hadConf) { Move-Item $confBackup $confDest -Force } else { Remove-Item $confDest }
        throw 'nginx の設定に誤りがあります（上の nginx: の行を確認してください）。study-record.conf は元に戻し、動いている nginx はそのままです'
    }

    # 5. Windows の起動時に nginx が自動で起動するようにする
    #    nginx（Windows 版）はサービスとして登録する機能を持たないため、タスク スケジューラで起動する。
    #    権限の低い LocalService で動かし、書き込みは logs / temp だけに許す。
    if (-not (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue)) {
        foreach ($dir in 'logs', 'temp') {
            New-Item -ItemType Directory -Force -Path (Join-Path $NginxHome $dir) | Out-Null
        }
        icacls $NginxHome /grant "${LocalServiceSid}:(OI)(CI)RX" | Out-Null
        icacls (Join-Path $NginxHome 'logs') /grant "${LocalServiceSid}:(OI)(CI)M" /T | Out-Null
        icacls (Join-Path $NginxHome 'temp') /grant "${LocalServiceSid}:(OI)(CI)M" /T | Out-Null
        $action = New-ScheduledTaskAction -Execute (Join-Path $NginxHome 'nginx.exe') -WorkingDirectory $NginxHome
        $trigger = New-ScheduledTaskTrigger -AtStartup
        $taskPrincipal = New-ScheduledTaskPrincipal -UserId 'NT AUTHORITY\LOCALSERVICE' -LogonType ServiceAccount
        # 既定の「3 日で停止」を無効にし、異常終了したら 1 分後に再起動する（3 回まで）
        $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) `
            -RestartInterval (New-TimeSpan -Minutes 1) -RestartCount 3 -StartWhenAvailable
        Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger `
            -Principal $taskPrincipal -Settings $settings `
            -Description '資格勉強 回答正誤記録アプリの nginx（Windows の起動時に起動する）' | Out-Null
        Write-Log "nginx を Windows の起動時に起動するタスク「$TaskName」を登録しました（LocalService で実行）"
    }

    # 6. 動いていれば再読み込みし、動いていなければ起動する
    if (Get-Process -Name nginx -ErrorAction SilentlyContinue) {
        if ((Invoke-Nginx -Arguments '-s', 'reload') -ne 0) { throw 'nginx の再読み込みに失敗しました' }
        Write-Log 'nginx に新しい設定と画面を読み込ませました'
    }
    else {
        Start-ScheduledTask -TaskName $TaskName
        Start-Sleep -Seconds 2
        if (-not (Get-Process -Name nginx -ErrorAction SilentlyContinue)) {
            throw "nginx を起動できませんでした。$NginxHome\logs\error.log を確認してください"
        }
        Write-Log 'nginx を起動しました'
    }
    Write-Log 'nginx への配置が完了しました'
}
catch {
    Write-Log "nginx への配置に失敗しました: $($_.Exception.Message)" 'ERROR'
    exit 1
}
