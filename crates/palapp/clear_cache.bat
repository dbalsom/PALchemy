@echo off
echo Clearing Tauri WebView cache...
if exist "%LOCALAPPDATA%\com.palchemy.dev\EBWebView" (
    rmdir /s /q "%LOCALAPPDATA%\com.palchemy.dev\EBWebView"
    echo Cache cleared successfully.
) else (
    echo Cache directory does not exist or is already clear.
)
pause
