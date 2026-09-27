@echo off
rem =====================================================================
rem 証明書の更新後に nginx へ新しい証明書を読み込ませる（Windows 用）
rem
rem win-acme の「更新後に実行するスクリプト」に指定して使う。
rem NGINX_HOME は nginx.exe のあるフォルダに書き換える。
rem 結果は logs\cert-renew.log に追記する。
rem =====================================================================
setlocal
set NGINX_HOME=C:\nginx

cd /d "%NGINX_HOME%" || exit /b 1
echo %date% %time% 証明書の更新を受けて nginx を再読み込みします >> logs\cert-renew.log

rem 設定に誤りがある場合は再読み込みせずに終わる（稼働中の nginx は止めない）
nginx.exe -t >> logs\cert-renew.log 2>&1 || (
  echo %date% %time% nginx の設定に誤りがあるため再読み込みを中止しました >> logs\cert-renew.log
  exit /b 1
)
nginx.exe -s reload >> logs\cert-renew.log 2>&1
echo %date% %time% 再読み込みが完了しました >> logs\cert-renew.log
endlocal
