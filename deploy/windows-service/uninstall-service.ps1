<#
.SYNOPSIS
    Windows サービス「StudyRecordServer」を停止して登録を解除する（サーバー PC で、管理者として実行する）。

.DESCRIPTION
    study-record-server.exe uninstall を呼び出す。
    ホームフォルダ（exe・config・data・logs）は消さずに残す。不要になったら手動で削除する。

.PARAMETER HomeDir
    ホームフォルダ。install-service.ps1 で指定したものと同じにする。省略すると C:\study-record。

.EXAMPLE
    .\uninstall-service.ps1
#>
[CmdletBinding()]
param(
    [string]$HomeDir = 'C:\study-record'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

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
    $line = '{0} {1} uninstall-service: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
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

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '管理者として開いた PowerShell で実行してください。' -ForegroundColor Red
    exit 1
}
if (-not (Test-Path (Join-Path $HomeDir 'logs'))) {
    Write-Host "$HomeDir にサービスが見つかりません。-HomeDir を確認してください。" -ForegroundColor Red
    exit 1
}

try {
    Write-Log 'サービスの登録解除を開始します'
    & (Join-Path $HomeDir 'study-record-server.exe') uninstall
    if ($LASTEXITCODE -ne 0) { throw "サービスの登録解除に失敗しました（終了コード $LASTEXITCODE）" }
    Write-Log "サービスの登録を解除しました（$HomeDir のデータとログは残しています）"
}
catch {
    Write-Log "サービスの登録解除に失敗しました: $($_.Exception.Message)" 'ERROR'
    exit 1
}
