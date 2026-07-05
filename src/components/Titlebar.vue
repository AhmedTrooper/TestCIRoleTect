<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Maximize2 } from "@lucide/vue";
import { ref, onMounted } from "vue";

const appWindow = getCurrentWindow();
const isMaximized = ref(false);

const minimize = () => appWindow.minimize();
const toggleMaximize = async () => {
  await appWindow.toggleMaximize();
  isMaximized.value = await appWindow.isMaximized();
};
const close = () => appWindow.close();

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized();
  // Listen for maximize events to sync state
  appWindow.onResized(async () => {
    isMaximized.value = await appWindow.isMaximized();
  });
});
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <div class="logo-dot" data-tauri-drag-region></div>
      <span class="app-title" data-tauri-drag-region>Roletect</span>
    </div>
    
    <div class="titlebar-center" data-tauri-drag-region>
      <!-- Optional: Add search or other centered elements here -->
    </div>

    <div class="titlebar-right">
      <button class="titlebar-button" @click="minimize" title="Minimize">
        <Minus :size="14" />
      </button>
      <button class="titlebar-button" @click="toggleMaximize" :title="isMaximized ? 'Restore' : 'Maximize'">
        <component :is="isMaximized ? Square : Maximize2" :size="isMaximized ? 10 : 12" />
      </button>
      <button class="titlebar-button close-button" @click="close" title="Close">
        <X :size="14" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 36px;
  background: var(--bg-accent);
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--line);
  user-select: none;
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 10000;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-left: 14px;
  height: 100%;
}

.logo-dot {
  width: 6px;
  height: 6px;
  background: var(--accent);
  border-radius: 50%;
  box-shadow: 0 0 8px var(--accent);
}

.app-title {
  font-size: 0.65rem;
  font-weight: 800;
  color: var(--muted);
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.titlebar-center {
  flex: 1;
  height: 100%;
}

.titlebar-right {
  display: flex;
  height: 100%;
}

.titlebar-button {
  width: 46px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--muted);
  cursor: default;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  border-radius: 0;
}

.titlebar-button:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.close-button:hover {
  background: #e81123 !important;
  color: white !important;
}

/* Add a subtle highlight to the top edge for a premium look */
.titlebar::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.05), transparent);
}


</style>
