# =====================================================================
# QuickTrans - one-shot environment installer
# Run in Windows PowerShell (NOT WSL), admin recommended:
#   powershell -ExecutionPolicy Bypass -File win-setup.ps1
# Needs internet. VS Build Tools is large, be patient.
# =====================================================================

Write-Host "==> 1/4 Installing Rust (rustup)..." -ForegroundColor Cyan
winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements

Write-Host "==> 2/4 Installing Node.js LTS..." -ForegroundColor Cyan
winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements

Write-Host "==> 3/4 Installing VS C++ Build Tools (required for Rust linking, large)..." -ForegroundColor Cyan
winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
  --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" `
  --accept-source-agreements --accept-package-agreements

Write-Host "==> 4/4 Installing WebView2 runtime (often preinstalled on Win11)..." -ForegroundColor Cyan
winget install --id Microsoft.EdgeWebView2Runtime -e --accept-source-agreements --accept-package-agreements

Write-Host ""
Write-Host "Done. CLOSE and REOPEN a new PowerShell so PATH takes effect," -ForegroundColor Yellow
Write-Host "then run: powershell -ExecutionPolicy Bypass -File win-run.ps1" -ForegroundColor Yellow
