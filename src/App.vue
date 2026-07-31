<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const source = ref("");
const translation = ref("");
const target = ref("中文");
const model = ref("deepseek-chat");
const loading = ref(false);
const errorMsg = ref("");
const copied = ref(false);
const sourceOpen = ref(false);
const bodyEl = ref<HTMLElement | null>(null);

const unlisteners: UnlistenFn[] = [];

async function hide() {
  await invoke("hide_window");
}

async function copy() {
  if (!translation.value) return;
  await invoke("copy_text", { text: translation.value });
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") hide();
}

/**
 * 拦掉双击标题栏的「缩放窗口」。
 *
 * Tauri 的 drag region 脚本在 macOS 上是在 document 的 mouseup 里触发 zoom 的
 * （Windows/Linux 走 mousedown），双击把这个小浮窗放大成一整块很怪。
 * 在 header 这一层截断冒泡即可，单击拖动走的是 mousedown，不受影响。
 */
function swallowTitleBarZoom(e: MouseEvent) {
  if (e.detail >= 2) e.stopPropagation();
}

async function scrollToBottom() {
  await nextTick();
  if (bodyEl.value) bodyEl.value.scrollTop = bodyEl.value.scrollHeight;
}

onMounted(async () => {
  window.addEventListener("keydown", onKey);

  unlisteners.push(
    await listen<{ source: string; target: string; model: string }>(
      "tr://start",
      (e) => {
        source.value = e.payload.source;
        target.value = e.payload.target;
        model.value = e.payload.model;
        translation.value = "";
        errorMsg.value = "";
        sourceOpen.value = false;
        loading.value = true;
      }
    )
  );

  unlisteners.push(
    await listen<string>("tr://chunk", async (e) => {
      translation.value += e.payload;
      await scrollToBottom();
    })
  );

  unlisteners.push(
    await listen("tr://done", () => {
      loading.value = false;
    })
  );

  unlisteners.push(
    await listen<string>("tr://error", (e) => {
      loading.value = false;
      errorMsg.value = e.payload;
    })
  );
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKey);
  unlisteners.forEach((u) => u());
});
</script>

<template>
  <div class="card">
    <header class="bar" data-tauri-drag-region="deep" @mouseup="swallowTitleBarZoom">
      <span class="title">
        <span class="dot" :class="{ spin: loading }"></span>
        划词翻译
        <span class="arrow">→ {{ target }}</span>
      </span>
      <button class="icon-btn" title="关闭 (Esc)" @click="hide">✕</button>
    </header>

    <div v-if="source" class="src" :class="{ open: sourceOpen }" @click="sourceOpen = !sourceOpen">
      <span class="src-label">原文</span>
      <span class="src-text">{{ source }}</span>
    </div>

    <main ref="bodyEl" class="body">
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
      <p v-else class="translation">
        {{ translation }}<span v-if="loading" class="caret"></span>
      </p>
      <p v-if="!errorMsg && !translation && !loading" class="hint">
        选中文本后按快捷键即可翻译
      </p>
    </main>

    <footer class="foot">
      <span class="model">{{ model }}</span>
      <button class="copy-btn" :disabled="!translation" @click="copy">
        {{ copied ? "已复制" : "复制译文" }}
      </button>
    </footer>
  </div>
</template>

<style scoped>
.card {
  height: 100%;
  margin: 10px;
  display: flex;
  flex-direction: column;
  border-radius: 16px;
  background: rgba(24, 24, 28, 0.92);
  backdrop-filter: blur(18px) saturate(160%);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
  overflow: hidden;
  color: #eceef2;
}

.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: default;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), transparent);
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.arrow {
  color: #8b8f99;
  font-weight: 500;
  font-size: 12px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6b7280;
}
.dot.spin {
  background: conic-gradient(#6366f1, #38bdf8, #6366f1);
  animation: rot 0.9s linear infinite;
}
@keyframes rot {
  to {
    transform: rotate(360deg);
  }
}

.icon-btn {
  border: none;
  background: transparent;
  color: #9aa0aa;
  font-size: 14px;
  cursor: pointer;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}
.icon-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
}

.src {
  margin: 0 12px;
  padding: 7px 10px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  font-size: 12px;
  color: #a8adb8;
  cursor: pointer;
  display: flex;
  gap: 8px;
  align-items: baseline;
}
.src-label {
  flex: none;
  color: #6b7280;
  font-size: 11px;
}
.src-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.src.open .src-text {
  white-space: pre-wrap;
  overflow: visible;
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  min-height: 60px;
}

.translation {
  font-size: 15px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text;
}

.caret {
  display: inline-block;
  width: 7px;
  height: 16px;
  margin-left: 2px;
  vertical-align: text-bottom;
  background: #6366f1;
  border-radius: 1px;
  animation: blink 1s steps(2) infinite;
}
@keyframes blink {
  50% {
    opacity: 0;
  }
}

.error {
  color: #f87171;
  font-size: 13px;
  line-height: 1.6;
}
.hint {
  color: #6b7280;
  font-size: 13px;
}

.foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}
.model {
  font-size: 11px;
  color: #6b7280;
  font-family: ui-monospace, "Cascadia Code", monospace;
}
.copy-btn {
  border: none;
  border-radius: 8px;
  padding: 5px 14px;
  font-size: 12px;
  font-weight: 600;
  color: #fff;
  background: linear-gradient(135deg, #6366f1, #38bdf8);
  cursor: pointer;
  transition: opacity 0.15s, transform 0.1s;
}
.copy-btn:hover:not(:disabled) {
  transform: translateY(-1px);
}
.copy-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.body::-webkit-scrollbar {
  width: 6px;
}
.body::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.14);
  border-radius: 3px;
}
</style>
