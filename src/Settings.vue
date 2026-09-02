<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface Config {
  api_key: string;
  model: string;
  target_lang: string;
  base_url: string;
  hotkey: string;
  ask_hotkey: string;
  ask_include_selection: boolean;
  ask_thinking: boolean;
}

interface PlatformInfo {
  os: string;
  accessibility_ok: boolean;
}

/** 两把全局热键的注册结果，空串代表正常 */
interface HotkeyStatus {
  translate_err: string;
  ask_err: string;
}

const api_key = ref("");
const model = ref("deepseek-v4-flash");
const target_lang = ref("中文");
const base_url = ref("https://api.deepseek.com");
const hotkey = ref("Ctrl+Alt+T");
const ask_hotkey = ref("Cmd+B");
const ask_include_selection = ref(true);
const ask_thinking = ref(false);

const showKey = ref(false);
const saving = ref(false);
const saved = ref(false);
const errorMsg = ref("");

const isMac = ref(false);
const axOk = ref(true);
const axChecking = ref(false);
const hk = ref<HotkeyStatus>({ translate_err: "", ask_err: "" });

const LANGS = ["中文", "English", "日本語", "한국어", "Français", "Deutsch"];

// DeepSeek 现在只有这两个模型；deepseek-chat / deepseek-reasoner 已于 2026-07-24 下线
const MODELS = [
  { id: "deepseek-v4-flash", desc: "快、便宜。翻译和日常问答用它就够" },
  { id: "deepseek-v4-pro", desc: "推理更强，复杂问题更稳，价格约 3 倍" },
];

/** 下拉框选中项。不在预设里就落到「自定义」，让下面的输入框接管 */
const modelPreset = computed({
  get: () => (MODELS.some((m) => m.id === model.value) ? model.value : "custom"),
  set: (v: string) => {
    model.value = v === "custom" ? "" : v;
  },
});

async function refreshPlatform() {
  try {
    const p = await invoke<PlatformInfo>("platform_info");
    isMac.value = p.os === "macos";
    axOk.value = p.accessibility_ok;
  } catch {
    // 拿不到就当无需授权，不打扰用户
  }
}

async function refreshHotkeys() {
  try {
    hk.value = await invoke<HotkeyStatus>("hotkey_status");
  } catch {
    // 同上，读不到就不提示
  }
}

async function grantAccessibility() {
  await invoke("open_accessibility_settings");
}

/** 系统设置里勾选后不会通知回来，只能让用户手动点一下重新检测 */
async function recheckAccessibility() {
  axChecking.value = true;
  await refreshPlatform();
  setTimeout(() => (axChecking.value = false), 400);
}

onMounted(async () => {
  await refreshPlatform();
  try {
    const c = await invoke<Config>("get_config");
    api_key.value = c.api_key ?? "";
    model.value = c.model || "deepseek-v4-flash";
    target_lang.value = c.target_lang || "中文";
    base_url.value = c.base_url || "https://api.deepseek.com";
    hotkey.value = c.hotkey || "Ctrl+Alt+T";
    ask_hotkey.value = c.ask_hotkey || (isMac.value ? "Cmd+B" : "Ctrl+Alt+B");
    ask_include_selection.value = c.ask_include_selection ?? true;
    ask_thinking.value = c.ask_thinking ?? false;
  } catch (e) {
    errorMsg.value = String(e);
  }
  await refreshHotkeys();
});

async function save(closeAfter: boolean) {
  saving.value = true;
  errorMsg.value = "";
  saved.value = false;
  try {
    await invoke("save_config", {
      args: {
        api_key: api_key.value,
        model: model.value,
        target_lang: target_lang.value,
        base_url: base_url.value,
        hotkey: hotkey.value,
        ask_hotkey: ask_hotkey.value,
        ask_include_selection: ask_include_selection.value,
        ask_thinking: ask_thinking.value,
      },
    });
    // 保存会重注册热键，注册成没成得回读一次才知道
    await refreshHotkeys();
    saved.value = true;
    if (closeAfter && !hk.value.translate_err && !hk.value.ask_err) {
      await invoke("hide_window");
    } else {
      setTimeout(() => (saved.value = false), 1500);
    }
  } catch (e) {
    errorMsg.value = "保存失败：" + String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="wrap">
    <h1>QuickTrans 设置</h1>
    <p class="sub">配置你的 DeepSeek 账号与快捷键，保存后立即生效。</p>

    <div v-if="isMac" class="perm" :class="{ bad: !axOk }">
      <div class="perm-head">
        <span class="perm-icon">{{ axOk ? "✓" : "⚠" }}</span>
        <span class="perm-title">
          辅助功能权限{{ axOk ? "已授予" : "未授予" }}
        </span>
      </div>
      <p class="perm-desc">
        <template v-if="axOk">
          QuickTrans 可以正常读取其他 App 里选中的文本。
        </template>
        <template v-else>
          macOS 会丢弃未授权 App 发出的复制指令，取词会一直失败。请在
          系统设置 → 隐私与安全性 → 辅助功能 中勾选 QuickTrans，然后回来点「重新检测」。
        </template>
      </p>
      <div v-if="!axOk" class="perm-actions">
        <button class="ghost" type="button" @click="grantAccessibility">
          打开授权面板
        </button>
        <button class="ghost" type="button" :disabled="axChecking" @click="recheckAccessibility">
          {{ axChecking ? "检测中…" : "重新检测" }}
        </button>
      </div>
    </div>

    <label class="field">
      <span class="lab">DeepSeek API Key <em>必填</em></span>
      <div class="key-row">
        <input
          :type="showKey ? 'text' : 'password'"
          v-model="api_key"
          placeholder="sk-..."
          spellcheck="false"
          autocomplete="off"
        />
        <button class="ghost" type="button" @click="showKey = !showKey">
          {{ showKey ? "隐藏" : "显示" }}
        </button>
      </div>
      <span class="tip">
        没有 Key？去
        <a href="https://platform.deepseek.com" target="_blank">platform.deepseek.com</a>
        注册并创建。
      </span>
    </label>

    <label class="field">
      <span class="lab">目标语言</span>
      <input v-model="target_lang" list="langs" placeholder="中文" />
      <datalist id="langs">
        <option v-for="l in LANGS" :key="l" :value="l" />
      </datalist>
    </label>

    <label class="field">
      <span class="lab">划词翻译快捷键</span>
      <input v-model="hotkey" placeholder="Ctrl+Alt+T" spellcheck="false" />
      <span v-if="hk.translate_err" class="warn">⚠ {{ hk.translate_err }}</span>
      <span v-if="isMac" class="tip">
        格式如 <code>Ctrl+Alt+T</code>、<code>Cmd+Shift+E</code>。修饰键
        Ctrl（⌃）/ Alt（⌥ Option）/ Shift（⇧）/ Cmd（⌘），主键 a-z、0-9、F1-F12、Space。
      </span>
      <span v-else class="tip">
        格式如 <code>Ctrl+Alt+T</code>、<code>Alt+F2</code>。修饰键 Ctrl / Alt / Shift / Super，
        主键 a-z、0-9、F1-F12、Space。
      </span>
    </label>

    <label class="field">
      <span class="lab">问答窗快捷键</span>
      <input v-model="ask_hotkey" placeholder="Cmd+B" spellcheck="false" />
      <span v-if="hk.ask_err" class="warn">⚠ {{ hk.ask_err }}</span>
      <span class="tip">
        全局快捷键是系统级抢占的：注册
        <code>{{ isMac ? "Cmd+B" : "Ctrl+B" }}</code>
        之后，所有 App 里的「加粗」都会失效。嫌碍事就换成
        <code>{{ isMac ? "Cmd+Shift+B" : "Ctrl+Alt+B" }}</code>。
        <template v-if="isMac">
          <br />⌘Space 被 Spotlight 占着，填了会注册失败。
        </template>
      </span>
    </label>

    <label class="check">
      <input type="checkbox" v-model="ask_include_selection" />
      <span>
        唤起问答窗时带上选中的文本
        <em>关掉可以秒开，开着会多花约 0.4 秒取词</em>
      </span>
    </label>

    <details class="adv">
      <summary>高级</summary>
      <label class="field">
        <span class="lab">模型</span>
        <select v-model="modelPreset">
          <option v-for="m in MODELS" :key="m.id" :value="m.id">{{ m.id }}</option>
          <option value="custom">自定义…</option>
        </select>
        <input
          v-if="modelPreset === 'custom'"
          v-model="model"
          class="mt"
          placeholder="deepseek-v4-flash"
          spellcheck="false"
        />
        <span class="tip">
          <template v-for="m in MODELS" :key="m.id">
            <template v-if="m.id === modelPreset">{{ m.desc }}</template>
          </template>
          <template v-if="modelPreset === 'custom'">
            自定义模型名。注意 <code>deepseek-chat</code> 与
            <code>deepseek-reasoner</code> 已于 2026-07-24 下线，填了会直接报错。
          </template>
        </span>
      </label>
      <label class="check">
        <input type="checkbox" v-model="ask_thinking" />
        <span>
          问答开启思考模式
          <em>回答更靠谱，但首字要等好几秒。翻译始终不用思考模式</em>
        </span>
      </label>
      <label class="field">
        <span class="lab">接口地址</span>
        <input v-model="base_url" placeholder="https://api.deepseek.com" spellcheck="false" />
      </label>
    </details>

    <p v-if="errorMsg" class="err">{{ errorMsg }}</p>

    <div class="actions">
      <span v-if="saved" class="ok">已保存 ✓</span>
      <button class="ghost" type="button" :disabled="saving" @click="save(false)">
        应用
      </button>
      <button class="primary" type="button" :disabled="saving" @click="save(true)">
        {{ saving ? "保存中…" : "保存并关闭" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  min-height: 100vh;
  box-sizing: border-box;
  padding: 22px 24px 18px;
  background: #16171b;
  color: #e7e9ee;
  user-select: none;
}

h1 {
  font-size: 18px;
  font-weight: 700;
}
.sub {
  margin: 4px 0 18px;
  font-size: 12px;
  color: #8a8f99;
}

.field {
  display: block;
  margin-bottom: 16px;
}

/* macOS 辅助功能授权状态卡片 */
.perm {
  margin-bottom: 18px;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px solid rgba(52, 211, 153, 0.35);
  background: rgba(52, 211, 153, 0.08);
}
.perm.bad {
  border-color: rgba(245, 158, 11, 0.4);
  background: rgba(245, 158, 11, 0.1);
}
.perm-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.perm-icon {
  color: #34d399;
  font-size: 13px;
}
.perm.bad .perm-icon {
  color: #f59e0b;
}
.perm-title {
  font-size: 12px;
  font-weight: 600;
  color: #c3c7d0;
}
.perm-desc {
  margin-top: 6px;
  font-size: 11px;
  line-height: 1.6;
  color: #8a8f99;
}
.perm-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}
.perm-actions button {
  padding: 6px 12px;
  font-size: 12px;
}
.lab {
  display: block;
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 6px;
  color: #c3c7d0;
}
.lab em {
  color: #f59e0b;
  font-style: normal;
  font-size: 11px;
  margin-left: 4px;
}

input,
select {
  width: 100%;
  box-sizing: border-box;
  padding: 9px 11px;
  border-radius: 9px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: #101114;
  color: #e7e9ee;
  font-size: 13px;
  outline: none;
  user-select: text;
  transition: border-color 0.15s;
}
input:focus,
select:focus {
  border-color: #6366f1;
}
.mt {
  margin-top: 8px;
}

.key-row {
  display: flex;
  gap: 8px;
}
.key-row input {
  flex: 1;
}

/* 复选框那一行：勾选框不能被上面的 width:100% 撑满 */
.check {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  margin-bottom: 16px;
  font-size: 12px;
  color: #c3c7d0;
  cursor: pointer;
  line-height: 1.5;
}
.check input[type="checkbox"] {
  width: auto;
  flex: none;
  margin-top: 2px;
  accent-color: #6366f1;
}
.check em {
  display: block;
  margin-top: 3px;
  font-style: normal;
  font-size: 11px;
  color: #767b85;
}

.tip {
  display: block;
  margin-top: 6px;
  font-size: 11px;
  color: #767b85;
  line-height: 1.5;
}
.tip a {
  color: #7c9cff;
}
.warn {
  display: block;
  margin-top: 6px;
  font-size: 11px;
  line-height: 1.5;
  color: #f59e0b;
}
code {
  background: rgba(255, 255, 255, 0.08);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 11px;
}

.adv {
  margin-bottom: 12px;
}
.adv summary {
  cursor: pointer;
  font-size: 12px;
  color: #8a8f99;
  margin-bottom: 12px;
  user-select: none;
}

.err {
  color: #f87171;
  font-size: 12px;
  margin-bottom: 10px;
}

.actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}
.ok {
  color: #34d399;
  font-size: 12px;
  margin-right: auto;
}

button {
  border: none;
  border-radius: 9px;
  padding: 9px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.ghost {
  background: rgba(255, 255, 255, 0.08);
  color: #d5d8df;
}
.ghost:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.14);
}
.primary {
  background: linear-gradient(135deg, #6366f1, #38bdf8);
  color: #fff;
}
.primary:hover:not(:disabled) {
  filter: brightness(1.06);
}
</style>
