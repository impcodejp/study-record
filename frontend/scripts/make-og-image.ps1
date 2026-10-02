<#
.SYNOPSIS
    SNS 共有用の画像（frontend\public\og-image.png、1200×630）を og-image.html から作り直す（開発 PC で実行する）。

.DESCRIPTION
    Windows に入っている Microsoft Edge を画面なしで起動し、og-image.html を撮影する。
    文言やデザインを変えたときだけ実行すればよい（作った画像は Git に登録する）。
    実行の記録は ProjectRoot\logs\SystemRunningLog.log に追記する。
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot
$Output = Join-Path (Split-Path -Parent $ScriptDir) 'public\og-image.png'
$LogPath = Join-Path (Split-Path -Parent (Split-Path -Parent $ScriptDir)) 'logs\SystemRunningLog.log'
$EdgeCandidates = @(
    'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe',
    'C:\Program Files\Microsoft\Edge\Application\msedge.exe'
)

<#
.SYNOPSIS
    画面とログファイル（SystemRunningLog.log）の両方にメッセージを出す。
#>
function Write-Log {
    param(
        [Parameter(Mandatory)][string]$Message,
        [ValidateSet('INFO', 'WARN', 'ERROR')][string]$Level = 'INFO'
    )
    $line = '{0} {1} make-og-image: {2}' -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fffzzz'), $Level, $Message
    Write-Host $line
    $dir = Split-Path -Parent $LogPath
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    # サーバーなどが書き込み中でも追記できるよう、共有を許可して開く
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

try {
    $edge = $EdgeCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $edge) { throw 'Microsoft Edge が見つかりません' }
    $profileDir = Join-Path ([System.IO.Path]::GetTempPath()) ('og-image-' + [guid]::NewGuid())
    $source = 'file:///' + ((Join-Path $ScriptDir 'og-image.html') -replace '\\', '/')
    $edgeArgs = @('--headless=new', '--disable-gpu', '--no-first-run', '--hide-scrollbars', "--user-data-dir=$profileDir",
        '--window-size=1200,630', "--screenshot=$Output", $source)
    $process = Start-Process -FilePath $edge -ArgumentList $edgeArgs -PassThru -Wait
    if ($process.ExitCode -ne 0 -or -not (Test-Path $Output)) { throw "画像を作れませんでした（終了コード $($process.ExitCode)）" }
    Remove-Item -Recurse -Force $profileDir -ErrorAction SilentlyContinue
    Write-Log "SNS 共有用の画像を作りました: $Output"
}
catch {
    Write-Log "SNS 共有用の画像を作れませんでした: $($_.Exception.Message)" 'ERROR'
    exit 1
}
