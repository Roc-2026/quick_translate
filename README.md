# QuickTrans · 划词翻译（DeepSeek）

选中任意软件里的文本 → 按快捷键 → 鼠标旁弹出浮窗，用 **DeepSeek** 逐字流式翻译。

一个常驻托盘的轻量 Windows 工具，基于 **Tauri v2 + Rust + Vue 3**，产物是单个原生 exe，内存占用很小。

> 选中即翻译、流式输出、可视化设置（填 Key / 改快捷键 / 换目标语言）、原剪贴板自动还原。

---

## ✨ 特性

- **划词翻译**：任意程序里选中文字，按快捷键（默认 `Ctrl+Alt+T`）即时翻译。
- **流式输出**：译文逐字蹦出，不用干等。
- **精致浮窗**：毛玻璃暗色 UI，出现在鼠标旁，失焦自动隐藏，`Esc` 关闭。
- **可视化设置**：托盘菜单里图形化填写 API Key、目标语言、快捷键，保存即时生效、无需重启。
- **不打扰剪贴板**：取词时会备份并还原你原有的剪贴板内容。
- **省心**：常驻系统托盘，原生 Rust，资源占用低。

---

## 🧩 技术栈

| 层 | 用到的东西 |
|----|-----------|
| 框架 | Tauri v2 |
| 后端 | Rust：`global-shortcut`(全局热键) · `enigo`(模拟 Ctrl+C) · `arboard`(剪贴板) · `reqwest`(流式请求) |
| 前端 | Vue 3 + TypeScript + Vite |
| 翻译 | DeepSeek `chat/completions`（OpenAI 兼容接口，流式） |

---

## 🚀 快速开始

> ⚠️ **本工具是 Windows 原生程序，必须在 Windows 上构建运行**（要用到 Windows 的全局热键、剪贴板、托盘）。若你用 WSL/Linux 管理代码，请把项目放到 **Windows 本地盘**再构建——不要在 `\\wsl.localhost\...` 网络路径上跑，`cmd.exe` 不支持 UNC 工作目录。

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

## 📦 打包成安装包

```powershell
npm run tauri build
```

产物在 `src-tauri/target/release/`（若设了 `CARGO_TARGET_DIR` 则在对应目录）：

- 安装包（NSIS）：`bundle/nsis/QuickTrans_x.y.z_x64-setup.exe`
- 免安装 exe：`quicktrans.exe`

> 未做代码签名的 exe，别人首次运行会被 Windows SmartScreen 提示，点「更多信息 → 仍要运行」即可；要消除警告需自备代码签名证书。

---

## ⚙️ 配置项

设置窗对应的配置文件在 `%APPDATA%\com.quicktrans.app\config.json`：

| 字段 | 含义 | 默认 |
|------|------|------|
| `api_key` | DeepSeek API Key | （必填） |
| `model` | 模型 | `deepseek-chat` |
| `target_lang` | 目标语言 | `中文` |
| `base_url` | 接口地址 | `https://api.deepseek.com` |
| `hotkey` | 全局快捷键 | `Ctrl+Alt+T` |

快捷键格式：修饰键 `Ctrl` / `Alt` / `Shift` / `Super` + 主键 `a-z` / `0-9` / `F1-F12` / `Space`，如 `Ctrl+Alt+T`、`Alt+F2`。
也可用环境变量 `DEEPSEEK_API_KEY` 覆盖 `api_key`。

---

## ❓ 常见问题

- **提示"没有选中文本"**：个别程序（部分游戏、受保护 PDF）不响应模拟 `Ctrl+C`；换个软件或先手动确认能复制。
- **热键没反应**：可能与其它软件全局热键冲突，去设置里换一个。
- **一直转圈 / 401**：`api_key` 没填对或未充值。
- **浮窗显示成黑块**：装 WebView2 运行时。
- **`enigo` 编译报错**：如版本 API 变动，调整 `src-tauri/Cargo.toml` 里的 `enigo` 版本并按其文档微调取词逻辑。

---

## 📁 目录结构

```
quick_translate/
├─ src/                     # Vue 前端
│  ├─ main.ts               # 按窗口 label 分发浮窗 / 设置界面
│  ├─ App.vue               # 翻译浮窗
│  └─ Settings.vue          # 设置界面
└─ src-tauri/               # Rust 后端
   └─ src/
      ├─ lib.rs             # 热键 / 取词 / 浮窗 / 托盘 / 命令
      ├─ config.rs          # 读写 config.json
      └─ deepseek.rs        # 流式翻译
```

---

## 📄 License

MIT
