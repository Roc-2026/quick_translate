#!/usr/bin/env bash
# =====================================================================
# QuickTrans - one-time environment setup for macOS
#   bash mac-setup.sh
#
# 装完后请「重开一个终端」再跑 mac-run.sh，否则 PATH 里没有 cargo。
# =====================================================================
set -euo pipefail

cyan() { printf '\033[36m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "这个脚本只能在 macOS 上跑。Windows 请用 win-setup.ps1。" >&2
  exit 1
fi

# 1) Xcode Command Line Tools —— 提供 clang/ld，Rust 链接必需（等价于 Windows 的 VS C++ 生成工具）
cyan "==> 检查 Xcode Command Line Tools..."
if xcode-select -p >/dev/null 2>&1; then
  green "  [OK] $(xcode-select -p)"
else
  yellow "  未安装，正在唤起安装向导（会弹一个系统窗口，装完再回来重跑本脚本）"
  xcode-select --install || true
  exit 1
fi

# 2) Rust
cyan "==> 检查 Rust..."
if command -v cargo >/dev/null 2>&1; then
  green "  [OK] $(rustc --version)"
else
  yellow "  未安装，正在通过 rustup 安装..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
  green "  [OK] $(rustc --version)"
fi

# 3) Node.js
cyan "==> 检查 Node.js..."
if command -v node >/dev/null 2>&1; then
  green "  [OK] node $(node --version)"
else
  if command -v brew >/dev/null 2>&1; then
    yellow "  未安装，正在 brew install node..."
    brew install node
  else
    echo "  [MISSING] 没有 node，也没有 Homebrew。" >&2
    echo "  先装 Homebrew：/bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"" >&2
    echo "  或直接从 https://nodejs.org 下载 LTS 安装包（需 >= 18）。" >&2
    exit 1
  fi
fi

# macOS 用系统自带的 WKWebView，不需要像 Windows 那样单独装 WebView2 运行时。
green ""
green "环境就绪。请「关闭并重新打开终端」让 PATH 生效，然后："
green "  bash mac-run.sh"
