<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface Config {
  api_key: string;
  model: string;
  target_lang: string;
  base_url: string;
  hotkey: string;
}

const api_key = ref("");
const model = ref("deepseek-chat");
const target_lang = ref("中文");
const base_url = ref("https://api.deepseek.com");
const hotkey = ref("Ctrl+Alt+T");

const showKey = ref(false);
const saving = ref(false);
const saved = ref(false);
const errorMsg = ref("");

const LANGS = ["中文", "English", "日本語", "한국어", "Français", "Deutsch"];

onMounted(async () => {
  try {
    const c = await invoke<Config>("get_config");
    api_key.value = c.api_key ?? "";
    model.value = c.model || "deepseek-chat";
    target_lang.value = c.target_lang || "中文";
    base_url.value = c.base_url || "https://api.deepseek.com";
    hotkey.value = c.hotkey || "Ctrl+Alt+T";
  } catch (e) {
    errorMsg.value = String(e);
  }
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
      },
    });
    saved.value = true;
    if (closeAfter) {
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
      <span class="lab">全局快捷键</span>
      <input v-model="hotkey" placeholder="Ctrl+Alt+T" spellcheck="false" />
      <span class="tip">
        格式如 <code>Ctrl+Alt+T</code>、<code>Alt+F2</code>。修饰键 Ctrl / Alt / Shift / Super，
        主键 a-z、0-9、F1-F12、Space。
      </span>
    </label>

    <details class="adv">
      <summary>高级</summary>
      <label class="field">
        <span class="lab">模型</span>
        <input v-model="model" placeholder="deepseek-chat" spellcheck="false" />
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

input {
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
input:focus {
  border-color: #6366f1;
}

.key-row {
  display: flex;
  gap: 8px;
}
.key-row input {
  flex: 1;
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
