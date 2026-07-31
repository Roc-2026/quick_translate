#!/usr/bin/env bash
# =====================================================================
# QuickTrans - verify toolchain + start dev mode (macOS)
#   bash mac-run.sh            # 开发模式
#   bash mac-run.sh build      # 打包出 .app / .dmg
#
# 注意：这里刻意不设 CARGO_TARGET_DIR。win-run.ps1 里设它是为了绕开
# WSL 网络路径导致的龟速构建，macOS 上没这个问题，用默认的
# src-tauri/target 即可（已在 .gitignore 里）。
# =====================================================================
set -euo pipefail

cyan() { printf '\033[36m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

cd "$(dirname "${BASH_SOURCE[0]}")"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "这个脚本只能在 macOS 上跑。Windows 请用 win-run.ps1。" >&2
  exit 1
fi

cyan "==> 检查工具链..."
for c in rustc cargo node npm; do
  if ! command -v "$c" >/dev/null 2>&1; then
    echo "  [MISSING] $c —— 先跑 bash mac-setup.sh，然后重开终端" >&2
    exit 1
  fi
  green "  [OK] $c: $("$c" --version | head -1)"
done

rustup default stable >/dev/null 2>&1 || true

cyan "==> 安装前端依赖 (npm install)..."
npm install

if [[ "${1:-dev}" == "build" ]]; then
  cyan "==> 打包中（首次 Rust 构建很慢，耐心等）..."
  npm run tauri build
  green ""
  green "产物在 src-tauri/target/release/bundle/ 下："
  green "  dmg/QuickTrans_*.dmg   —— 拖进 Applications 安装"
  green "  macos/QuickTrans.app   —— 也可以直接双击运行"
else
  yellow "提示：首次运行会要求「辅助功能」授权，"
  yellow "      在 系统设置 → 隐私与安全性 → 辅助功能 里勾上 QuickTrans 再试快捷键。"
  cyan "==> 启动开发模式（首次 Rust 构建很慢，耐心等）..."
  npm run tauri dev
fi
