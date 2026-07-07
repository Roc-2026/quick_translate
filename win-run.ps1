# =====================================================================
# QuickTrans - verify toolchain + start dev mode
# Run in a NEWLY OPENED Windows PowerShell (not WSL):
#   powershell -ExecutionPolicy Bypass -File win-run.ps1
# =====================================================================

$ErrorActionPreference = "Stop"

Write-Host "==> Checking toolchain..." -ForegroundColor Cyan
foreach ($c in @("rustc", "cargo", "node", "npm")) {
    $v = (& $c --version) 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $v) {
        Write-Host "  [MISSING] $c -- install it and REOPEN PowerShell" -ForegroundColor Red
        exit 1
    }
    Write-Host ("  [OK] {0}: {1}" -f $c, $v) -ForegroundColor Green
}

# Ensure a stable rust toolchain exists
rustup default stable | Out-Null

Write-Host "==> Setting cargo target dir to local disk (faster builds)..." -ForegroundColor Cyan
setx CARGO_TARGET_DIR "C:\quicktrans-target" | Out-Null
$env:CARGO_TARGET_DIR = "C:\quicktrans-target"   # effective in this session
Write-Host "  CARGO_TARGET_DIR = C:\quicktrans-target"

Write-Host "==> Installing frontend deps (npm install)..." -ForegroundColor Cyan
npm install

Write-Host "==> Starting dev mode (first Rust build is slow, please wait)..." -ForegroundColor Cyan
npm run tauri dev
