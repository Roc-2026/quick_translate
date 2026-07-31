# QuickTrans · 划词翻译（DeepSeek）

选中任意软件里的文本 → 按快捷键 → 鼠标旁弹出浮窗，用 **DeepSeek** 逐字流式翻译。

一个常驻托盘 / 菜单栏的轻量工具，基于 **Tauri v2 + Rust + Vue 3**，产物是单个原生可执行文件，内存占用很小。**支持 Windows 与 macOS**。

> 选中即翻译、流式输出、可视化设置（填 Key / 改快捷键 / 换目标语言）、原剪贴板自动还原。

---

## ✨ 特性

- **划词翻译**：任意程序里选中文字，按快捷键（默认 `Ctrl+Alt+T`，macOS 上即 `⌃ Control + ⌥ Option + T`）即时翻译。
- **流式输出**：译文逐字蹦出，不用干等。
- **精致浮窗**：毛玻璃暗色 UI，出现在鼠标旁，失焦自动隐藏，`Esc` 关闭；多屏 / 屏幕边缘会自动避让。
- **可视化设置**：托盘菜单里图形化填写 API Key、目标语言、快捷键，保存即时生效、无需重启。
- **不打扰剪贴板**：取词时会备份并还原你原有的剪贴板内容。
- **省心**：常驻托盘（macOS 上是菜单栏，且不占 Dock），原生 Rust，资源占用低。

---

## 🧩 技术栈

| 层 | 用到的东西 |
|----|-----------|
| 框架 | Tauri v2 |
| 后端 | Rust：`global-shortcut`(全局热键) · `enigo`(模拟 Ctrl+C / ⌘C) · `arboard`(剪贴板) · `reqwest`(流式请求) |
| 前端 | Vue 3 + TypeScript + Vite |
| 翻译 | DeepSeek `chat/completions`（OpenAI 兼容接口，流式） |

平台差异（全局热键、剪贴板、托盘、坐标系）都在同一份代码里用 `#[cfg(target_os = ...)]` 分支处理，没有分叉的第二套代码。

---

## 🚀 快速开始（Windows）

> ⚠️ **Windows 版必须在 Windows 上构建运行**。若你用 WSL/Linux 管理代码，请把项目放到 **Windows 本地盘**再构建——不要在 `\\wsl.localhost\...` 网络路径上跑，`cmd.exe` 不支持 UNC 工作目录。

### 1. 装环境（Windows，一次性）

- **Rust**：<https://rustup.rs>
- **Node.js ≥ 18**：<https://nodejs.org>
- **Visual Studio C++ 生成工具**：安装时勾选「使用 C++ 的桌面开发」（Rust 链接需要）
- **WebView2 运行时**：Win10/11 多半已自带，缺了就装 Evergreen 版

仓库里也附了 `win-setup.ps1`，可用 `winget` 一键安装上述环境：
```powershell
powershell -ExecutionPolicy Bypass -File win-setup.ps1
```

### 2. 拿 DeepSeek API Key

到 <https://platform.deepseek.com> 注册 → API Keys 创建 `sk-...` → 充少量额度。

### 3. 克隆并运行

```powershell
git clone https://github.com/Roc-2026/quick_translate.git
cd quick_translate
npm install
npm run tauri dev
```

首次会编译 Rust 依赖，稍慢；之后很快。启动后主窗口默认隐藏，看**任务栏右下角托盘**的「译」图标。

> 若 PowerShell 报「禁止运行脚本」，执行一次 `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` 即可。

### 4. 配置 Key

首次启动没 Key 时会**自动弹出设置窗**；也可随时右键托盘「译」图标 →「**设置**」，填入 API Key，保存即用。

### 5. 使用

任意软件里选中文字 → 按 `Ctrl+Alt+T` → 浮窗出译文。

---

## 🍎 快速开始（macOS）

> macOS 版同样必须**在 Mac 上构建**（要用系统的全局热键、剪贴板、菜单栏）。不能在 Windows/Linux 上交叉编译。

### 1. 装环境（一次性）

```bash
bash mac-setup.sh      # Xcode Command Line Tools + Rust + Node
```

装完**关掉终端重开**，让 `PATH` 里有 `cargo`。手动装也行：

- **Xcode Command Line Tools**：`xcode-select --install`（提供 clang/ld，相当于 Windows 的 VS C++ 生成工具）
- **Rust**：<https://rustup.rs>
- **Node.js ≥ 18**：`brew install node` 或 <https://nodejs.org>

macOS 用系统自带的 WKWebView，**不需要**像 Windows 那样装 WebView2 运行时。

### 2. 跑起来

```bash
git clone https://github.com/Roc-2026/quick_translate.git
cd quick_translate
bash mac-run.sh        # = npm install && npm run tauri dev
```

启动后**看菜单栏右上角**的「译」图标（单色 template 图，会跟随浅色/深色菜单栏自动反色）。按设计**不会**在 Dock 里出现图标，也不进 `⌘Tab` 切换列表。

### 3. 授予「辅助功能」权限 ← 必做

QuickTrans 靠合成一次 `⌘C` 来取走你选中的文本。**未授权时 macOS 会静默丢弃这个按键**，取词永远为空。

1. 系统设置 → 隐私与安全性 → **辅助功能**
2. 打开 QuickTrans 的开关（`tauri dev` 模式下条目名可能显示为 `quicktrans`）
3. 回到设置窗点「重新检测」

首次启动若检测到没授权，会自动弹出设置窗，里面有「打开授权面板」按钮直达。

> ⚠️ **每次重新编译，二进制变了，授权就会失效**，需要在列表里删掉旧条目重新勾选。开发阶段这个很烦但绕不开 —— 除非用固定的开发者证书签名。

### 4. 配置 Key + 使用

和 Windows 一样：菜单栏图标 →「设置」填 API Key。然后任意软件里选中文字 → 按 `Ctrl+Alt+T`（即 `⌃⌥T`）→ 浮窗出译文。

### 5. 打包

```bash
bash mac-run.sh build      # = npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`：

- `dmg/QuickTrans_x.y.z_<arch>.dmg` —— 拖进「应用程序」安装
- `macos/QuickTrans.app` —— 也可直接双击运行

想出**同时支持 Intel 和 Apple Silicon** 的通用包：

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri build -- --target universal-apple-darwin
```

> 未做代码签名/公证的 App，别人拿到会被 Gatekeeper 拦下（「已损坏，无法打开」）。让对方执行一次：
> ```bash
> xattr -dr com.apple.quarantine /Applications/QuickTrans.app
> ```
> 或右键 →「打开」→ 再点「打开」。要彻底消除警告需要 Apple 开发者账号做签名 + 公证。

---

## 📦 打包成安装包（Windows）

```powershell
npm run tauri build
```

产物在 `src-tauri/target/release/`（若设了 `CARGO_TARGET_DIR` 则在对应目录）：

- 安装包（NSIS）：`bundle/nsis/QuickTrans_x.y.z_x64-setup.exe`
- 免安装 exe：`quicktrans.exe`

> 未做代码签名的 exe，别人首次运行会被 Windows SmartScreen 提示，点「更多信息 → 仍要运行」即可；要消除警告需自备代码签名证书。

macOS 的打包见上面「快速开始（macOS）」第 5 步。

---

## ⚙️ 配置项

设置窗对应的配置文件：

- Windows：`%APPDATA%\com.quicktrans.app\config.json`
- macOS：`~/Library/Application Support/com.quicktrans.app/config.json`

（也可以直接用托盘/菜单栏菜单里的「打开配置目录」。）

| 字段 | 含义 | 默认 |
|------|------|------|
| `api_key` | DeepSeek API Key | （必填） |
| `model` | 模型 | `deepseek-chat` |
| `target_lang` | 目标语言 | `中文` |
| `base_url` | 接口地址 | `https://api.deepseek.com` |
| `hotkey` | 全局快捷键 | `Ctrl+Alt+T` |

快捷键格式：修饰键 `Ctrl` / `Alt` / `Shift` / `Super` + 主键 `a-z` / `0-9` / `F1-F12` / `Space`，如 `Ctrl+Alt+T`、`Alt+F2`。
macOS 上 `Alt` = `⌥ Option`，`Super`（也可写 `Cmd`）= `⌘ Command`，例如 `Cmd+Shift+E`。
也可用环境变量 `DEEPSEEK_API_KEY` 覆盖 `api_key`。

---

## ❓ 常见问题

**通用**

- **提示"没有选中文本"**：个别程序（部分游戏、受保护 PDF）不响应模拟复制；换个软件或先手动确认能复制。
- **热键没反应**：可能与其它软件全局热键冲突，去设置里换一个。
- **一直转圈 / 401**：`api_key` 没填对或未充值。
- **`enigo` 编译报错**：如版本 API 变动，调整 `src-tauri/Cargo.toml` 里的 `enigo` 版本并按其文档微调取词逻辑。

**Windows**

- **浮窗显示成黑块**：装 WebView2 运行时。

**macOS**

- **按快捷键提示「未授予辅助功能权限」**：见上面第 3 步。**重新编译后需要重新授权**。
- **浮窗是不透明黑块，没有毛玻璃**：`tauri.conf.json` 里的 `app.macOSPrivateApi` 必须为 `true`，且 `Cargo.toml` 里 tauri 要开 `macos-private-api` feature —— 两者缺一不可。
- **菜单栏图标是彩色的、很脏**：`src-tauri/icons/tray-mac.png` 应该是纯黑字形 + 透明背景的 template 图；重新跑 `python3 gen_icons.py` 生成。
- **打包报找不到 `icon.icns`**：跑 `python3 gen_icons.py`（纯 Python 生成，不依赖 macOS 的 `iconutil`），或用 `npx tauri icon src-tauri/icons/icon.png`。
- **浮窗弹出时源程序失去焦点**：目前是普通窗口，show 时会抢焦点（与 Windows 版行为一致）。要做到完全不抢需要换成非激活的 `NSPanel`（`tauri-nspanel` 插件），暂未实现。

---

## 📁 目录结构

```
quick_translate/
├─ gen_icons.py             # 生成全平台图标（含 macOS 的 .icns 与菜单栏 template 图）
├─ win-setup.ps1 / win-run.ps1   # Windows 环境安装 / 开发启动
├─ mac-setup.sh  / mac-run.sh    # macOS  环境安装 / 开发启动、打包
├─ src/                     # Vue 前端
│  ├─ main.ts               # 按窗口 label 分发浮窗 / 设置界面
│  ├─ App.vue               # 翻译浮窗
│  └─ Settings.vue          # 设置界面（含 macOS 辅助功能授权卡片）
└─ src-tauri/               # Rust 后端
   └─ src/
      ├─ lib.rs             # 热键 / 取词 / 浮窗 / 托盘 / 命令 / 平台分支
      ├─ config.rs          # 读写 config.json
      └─ deepseek.rs        # 流式翻译
```

---

## 📄 License

MIT
