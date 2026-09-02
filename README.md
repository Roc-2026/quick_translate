# QuickTrans · 划词翻译 + 快捷问答（DeepSeek）

选中任意软件里的文本 → 按快捷键 → 鼠标旁弹出浮窗，用 **DeepSeek** 逐字流式翻译。
再按一下问答快捷键，就地追问刚选中的那段内容。

一个常驻托盘 / 菜单栏的轻量工具，基于 **Tauri v2 + Rust + Vue 3**，产物是单个原生可执行文件，内存占用很小。**支持 Windows 与 macOS**。

> 选中即翻译、流式输出、多轮问答、可视化设置（填 Key / 改快捷键 / 换目标语言）、原剪贴板自动还原。

---

## ✨ 特性

- **划词翻译**：任意程序里选中文字，按快捷键（默认 `Ctrl+Alt+T`，macOS 上即 `⌃ Control + ⌥ Option + T`）即时翻译。
- **快捷问答**：按 `Cmd+B`（Windows 上 `Ctrl+Alt+B`）唤起问答窗，选中的文本自动带进去当上下文，可以连续追问。
- **流式输出**：译文和回答都逐字蹦出，不用干等。
- **精致浮窗**：毛玻璃暗色 UI，出现在鼠标旁，失焦自动隐藏，`Esc` 关闭；多屏 / 屏幕边缘会自动避让。
- **盖得住全屏应用**（macOS）：别的 App 在原生全屏（比如全屏读 PDF）时，浮窗和问答窗照样能就地弹出，不会把你踢回另一个 Space。
- **可视化设置**：托盘菜单里图形化填写 API Key、目标语言、快捷键，保存即时生效、无需重启。
- **不打扰剪贴板**：取词时会备份并还原你原有的剪贴板内容。
- **省心**：常驻托盘（macOS 上是菜单栏，且不占 Dock），原生 Rust，资源占用低。

---

## 🧩 技术栈

| 层 | 用到的东西 |
|----|-----------|
| 框架 | Tauri v2 |
| 后端 | Rust：`global-shortcut`(全局热键) · `enigo`(模拟 Ctrl+C / ⌘C) · `arboard`(剪贴板) · `reqwest`(流式请求) · `objc2`(macOS 全屏浮窗) |
| 前端 | Vue 3 + TypeScript + Vite |
| 模型 | DeepSeek `chat/completions`（OpenAI 兼容接口，流式），默认 `deepseek-v4-flash` |

平台差异（全局热键、剪贴板、托盘、坐标系）都在同一份代码里用 `#[cfg(target_os = ...)]` 分支处理，没有分叉的第二套代码。

---

## 📥 直接下载（不想自己编译的话）

到 **[Releases](https://github.com/Roc-2026/quick_translate/releases/latest)** 下载现成的安装包：

| 平台 | 文件 |
|------|------|
| macOS（Apple Silicon） | `QuickTrans_x.y.z_aarch64.zip` |
| Windows 10/11（64 位） | `QuickTrans_x.y.z_x64-setup.exe` |

**macOS**：解压后把 `QuickTrans.app` 拖进「应用程序」，然后在终端里执行一次
```bash
xattr -dr com.apple.quarantine /Applications/QuickTrans.app
```
这一步不能跳 —— 没有 Apple 代码签名的 App 直接双击会提示「已损坏」，其实只是被打了「从网上下载」的隔离标记。

**Windows**：双击 exe，SmartScreen 提示时点「更多信息 → 仍要运行」。

Intel Mac、或者想改代码，往下看自行构建。

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
想追问就按 `Ctrl+Alt+B` → 问答窗弹出，选中的那段已经在引用条里，直接提问即可。

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
bash mac-run.sh build      # 打包成 .app
cp -R src-tauri/target/release/bundle/macos/QuickTrans.app /Applications/
open /Applications/QuickTrans.app
```

首次 Rust 构建很慢，**十几分钟属正常**。

> 💡 **日常使用请打包成 `.app`，不要用 `bash mac-run.sh`（dev 模式）。**
>
> dev 模式跑的是裸二进制 `src-tauri/target/debug/quicktrans`，没有 bundle ID，
> 在「辅助功能」列表里**不叫 QuickTrans** —— 显示成小写的 `quicktrans`，
> 或者干脆被算到父终端（Terminal / iTerm / VS Code）头上，很难找。
> 而且每次重新编译授权就失效。
>
> `.app` 带 `com.quicktrans.app` 这个 bundle ID，授权窗会正常弹、名字正常显示，
> 只要不重新编译就一直有效。改代码时才用 `bash mac-run.sh` 跑 dev。

启动后**看菜单栏右上角**的「译」图标（单色 template 图，会跟随浅色/深色菜单栏自动反色）。按设计**不会**在 Dock 里出现图标，也不进 `⌘Tab` 切换列表 —— 找不到窗口是正常的。

### 3. 授予「辅助功能」权限 ← 必做

QuickTrans 靠合成一次 `⌘C` 来取走你选中的文本。**未授权时 macOS 会静默丢弃这个按键**，取词永远为空。

App 一启动就会调 `CGRequestPostEventAccess()` 主动请求，所以：

1. 首次启动应该会**自动弹出系统授权窗** —— 直接点「打开系统设置」授权即可
2. 系统设置 → 隐私与安全性 → **辅助功能**，打开 QuickTrans 的开关
3. 回到设置窗点「重新检测」

首次启动若检测到没授权，也会自动弹出设置窗，里面的「打开授权面板」按钮同样会先请求权限再跳转。

> 严格来说发合成按键要的是 **PostEvent** 权限，与 Accessibility 是两个独立的 TCC 条目，只是都显示在「辅助功能」面板下。必须**主动 request 才会把 App 注册进那个列表** —— 只做 `AXIsProcessTrusted()` 之类的查询不会注册，列表里就一直看不到 QuickTrans。

> ⚠️ **每次重新编译，二进制变了，授权就会失效**，需要在列表里**用减号删掉旧条目**再重新勾选（光是关掉再打开开关没用）。这也是日常用 `.app`、只在改代码时才跑 dev 的另一个理由。
>
> 卡死时可以重置授权记录后重启 App：
> ```bash
> tccutil reset PostEvent com.quicktrans.app    # 针对 .app
> tccutil reset PostEvent                        # dev 模式的裸二进制没有 bundle ID，只能全量重置
> ```

### 4. 配置 Key + 使用

和 Windows 一样：菜单栏图标 →「设置」填 API Key。然后任意软件里选中文字 → 按 `Ctrl+Alt+T`（即 `⌃⌥T`）→ 浮窗出译文；按 `Cmd+B` → 问答窗弹出，选中的那段已在引用条里，直接追问。

> `Cmd+B` 是全局抢占的，注册之后各家编辑器里的「加粗」都会失效。嫌碍事就去设置里改成 `Cmd+Shift+B` 之类。`⌘Space` 被 Spotlight 占着，填了会注册失败（设置界面会给出告警）。

### 5. 改代码时（dev 模式）

```bash
bash mac-run.sh            # = npm install && npm run tauri dev
```

前端改动热更新；Rust 改动会重新编译。注意上面提过的两个 dev 模式限制：辅助功能列表里显示为小写 `quicktrans`（或归到父终端名下），且每次重编译都要重新授权。

### 6. 出分发包

```bash
bash mac-run.sh build
```

产物在 `src-tauri/target/release/bundle/`：

- `macos/QuickTrans.app` —— 可直接双击运行
- `dmg/QuickTrans_x.y.z_<arch>.dmg` —— 拖进「应用程序」安装

> 💡 **要分发给别人，建议压缩 `.app` 而不是用 dmg。** `bundle_dmg.sh` 依赖 `hdiutil` 挂载临时卷加 `osascript` 驱动 Finder 摆图标，很容易因为残留的挂载卷或终端缺少「自动化 → Finder」权限而失败，而 dmg 除了拖拽动画外没有实际价值。
>
> ```bash
> cd src-tauri/target/release/bundle/macos
> ditto -c -k --keepParent QuickTrans.app QuickTrans_x.y.z_aarch64.zip
> ```
>
> **必须用 `ditto`**，不能用右键压缩或 `zip -r` —— `.app` 里有符号链接和扩展属性，普通 zip 会破坏 bundle 结构，对方解压出来打不开。

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
| `model` | 模型，只能是 `deepseek-v4-flash` 或 `deepseek-v4-pro` | `deepseek-v4-flash` |
| `target_lang` | 目标语言 | `中文` |
| `base_url` | 接口地址 | `https://api.deepseek.com` |
| `hotkey` | 划词翻译快捷键 | `Ctrl+Alt+T` |
| `ask_hotkey` | 问答窗快捷键 | macOS `Cmd+B`；其他平台 `Ctrl+Alt+B` |
| `ask_include_selection` | 唤起问答窗时是否顺带取词做上下文 | `true` |
| `ask_thinking` | 问答是否开启思考模式（更聪明但首字慢好几秒） | `false` |

快捷键格式：修饰键 `Ctrl` / `Alt` / `Shift` / `Super` + 主键 `a-z` / `0-9` / `F1-F12` / `Space`，如 `Ctrl+Alt+T`、`Alt+F2`。
macOS 上 `Alt` = `⌥ Option`，`Super`（也可写 `Cmd`）= `⌘ Command`，例如 `Cmd+Shift+E`。
也可用环境变量 `DEEPSEEK_API_KEY` 覆盖 `api_key`。

> **关于模型**：DeepSeek 已于 2026-07-24 下线 `deepseek-chat` 和 `deepseek-reasoner`，再用这两个名字请求会直接报错。
> 旧版本留下的 `config.json` 在启动时会**自动迁移**到 `deepseek-v4-flash`，不用手动改。
> V4 的**思考模式默认是开的**，翻译路径固定关掉（否则首字要等好几秒，且 `temperature` 会被静默忽略），问答路径由 `ask_thinking` 控制。

---

## ❓ 常见问题

**通用**

- **提示"没有取到选中的文本"**：当前确实没选中，或这个程序不响应模拟复制（部分游戏、受保护 PDF）。换个软件，或先手动 `Ctrl+C` / `⌘C` 确认能复制。
- **译出来的是之前复制过的内容，不是这次选中的**：已修（见下面 macOS 一节）。若在旧版本上遇到，升级即可。
- **热键没反应**：可能与其它软件全局热键冲突，去设置里换一个。注册失败时设置界面会直接给出黄色告警，不用猜。
- **按了 `Cmd+B` 之后各种编辑器不能加粗了**：这是全局热键的固有代价 —— 系统级抢占，所有 App 都收不到这个组合了。去设置里换成 `Cmd+Shift+B` 之类即可。macOS 上 `⌘Space` 被 Spotlight 占着，填了会注册失败。
- **一直转圈 / 401**：`api_key` 没填对或未充值。
- **报「Model Not Exist」**：`deepseek-chat` / `deepseek-reasoner` 已于 2026-07-24 下线。设置 → 高级 → 模型，换成 `deepseek-v4-flash`。
- **回答要停顿好几秒才出字**：思考模式开着。设置 → 高级，关掉「问答开启思考模式」。
- **`enigo` 编译报错**：如版本 API 变动，调整 `src-tauri/Cargo.toml` 里的 `enigo` 版本并按其文档微调取词逻辑。

**Windows**

- **浮窗显示成黑块**：装 WebView2 运行时。

**macOS**

- **下载的 App 提示「已损坏，无法打开」**：不是真的损坏，是没做 Apple 代码签名。执行一次 `xattr -dr com.apple.quarantine /Applications/QuickTrans.app`，或右键 →「打开」→ 再点「打开」。
- **打包 dmg 失败（`bundle_dmg.sh` 报错）**：`.app` 其实已经编译好了（在 `bundle/macos/` 下），失败的只是最后塞进 dmg 那步。两个常见原因：上次失败残留的卷还挂着（`hdiutil detach "/Volumes/QuickTrans" -force`），或终端缺少「自动化 → Finder」权限（系统设置 → 隐私与安全性 → 自动化，勾上你的终端下的 Finder；列表里没有就 `tccutil reset AppleEvents` 后重跑，弹窗时点「好」）。要分发的话直接 `ditto` 压 `.app` 更省事，见上面第 6 步。
- **按快捷键提示「未授予辅助功能权限」**：见上面第 3 步。**重新编译后需要重新授权**。
- **辅助功能列表里找不到 QuickTrans**：先确认你跑的是 `.app` 而不是 dev 模式 —— dev 的裸二进制在列表里显示成小写 `quicktrans`，或被算到父终端（Terminal / iTerm / VS Code）头上，勾那个父终端并**完全退出重开**（⌘Q）往往就能用。必须由 App 主动 `CGRequestPostEventAccess()` 才会注册进列表（启动时会自动调）。仍缺失就用 `tccutil reset PostEvent com.quicktrans.app`（dev 模式没有 bundle ID，用不带参数的全量重置）后重启 App；手动添加则在列表里点 `+`，用 `⌘⇧G` 输入可执行文件的完整路径（默认对话框只显示 `/Applications`）。
- **译出来的永远是上一次复制的内容**：已修。两层成因：① 触发键 `⌃⌥T` 的修饰键用户还按着时，合成的 `⌘C` 会变成 `⌃⌥⌘C`，不是复制快捷键，系统什么都不做 —— 而 enigo 发 Release 事件**清不掉物理按键造成的 flags**，只能用 `CGEventSourceFlagsState()` 轮询真实状态等用户松手；② 原先固定 sleep 后无条件读剪贴板，**无法区分"复制成功"与"复制没生效"**，就把上次的残留当本次选中内容发出去了。现在发 `⌘C` 前先写入哨兵字符串，之后轮询，内容仍是哨兵即判定失败并报错（比对比文本可靠 —— 选中内容与上次复制相同时光比文本区分不了）。见 `lib.rs` 的 `grab_selection`。
- **按快捷键后 App 直接崩溃（`EXC_BREAKPOINT` / `SIGTRAP`）**：已修。成因是 enigo 的 `Key::Unicode` 会反查键盘布局，那条路径调的 Carbon `TISGetInputSourceProperty` 带主线程断言，而取词跑在 tokio blocking 线程上。现在改成直接发 `⌘C` 的 virtual keycode，不再反查。
- **浮窗是不透明黑块，没有毛玻璃**：`tauri.conf.json` 里的 `app.macOSPrivateApi` 必须为 `true`，且 `Cargo.toml` 里 tauri 要开 `macos-private-api` feature —— 两者缺一不可。
- **菜单栏图标是彩色的、很脏**：`src-tauri/icons/tray-mac.png` 应该是纯黑字形 + 透明背景的 template 图；重新跑 `python3 gen_icons.py` 生成。
- **打包报找不到 `icon.icns`**：跑 `python3 gen_icons.py`（纯 Python 生成，不依赖 macOS 的 `iconutil`），或用 `npx tauri icon src-tauri/icons/icon.png`。
- **别的 App 全屏时按快捷键没反应**（比如全屏读 PDF）：已修。macOS 的原生全屏会把那个 App 放进独立的 Space，普通窗口不属于该 Space，`show()` 的结果是系统把你切回 QuickTrans 自己的 Space 再显示，看起来就像没反应。现在三个窗口在启动时都会被设上 `NSWindowCollectionBehaviorCanJoinAllSpaces | FullScreenAuxiliary`（见 `lib.rs` 的 `mac::make_overlay`）——**这两位缺一不可**，tao 的 `set_visible_on_all_workspaces()` 只设了前者，所以没法直接用。
- **浮窗弹出时源程序失去焦点**：目前是普通窗口，show 时会抢焦点（与 Windows 版行为一致）。要做到完全不抢需要换成非激活的 `NSPanel`（`tauri-nspanel` 插件），暂未实现。

---

## 📁 目录结构

```
quick_translate/
├─ gen_icons.py             # 生成全平台图标（含 macOS 的 .icns 与菜单栏 template 图）
├─ win-setup.ps1 / win-run.ps1   # Windows 环境安装 / 开发启动
├─ mac-setup.sh  / mac-run.sh    # macOS  环境安装 / 开发启动、打包
├─ src/                     # Vue 前端
│  ├─ main.ts               # 按窗口 label 分发浮窗 / 问答 / 设置界面
│  ├─ App.vue               # 翻译浮窗
│  ├─ Ask.vue               # 问答窗（多轮对话 + 引用选中文本）
│  └─ Settings.vue          # 设置界面（含 macOS 辅助功能授权卡片、热键冲突提示）
└─ src-tauri/               # Rust 后端
   └─ src/
      ├─ lib.rs             # 热键 / 取词 / 浮窗 / 托盘 / 命令 / 平台分支（含 macOS 全屏浮窗）
      ├─ config.rs          # 读写 config.json（含旧模型名自动迁移）
      └─ deepseek.rs        # 流式请求：翻译与问答共用
```

---

## 📄 License

MIT
