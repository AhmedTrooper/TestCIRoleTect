<script setup lang="ts">
import { Motion, AnimatePresence } from 'motion-v';
import { CloudUpload } from '@lucide/vue';

defineProps<{
  isVisible: boolean;
}>();
</script>

<template>
  <AnimatePresence>
    <Motion
      v-if="isVisible"
      :initial="{ opacity: 0 }"
      :animate="{ opacity: 1 }"
      :exit="{ opacity: 0 }"
      :transition="{ duration: 0.3 }"
      class="upload-overlay"
    >
      <Motion
        :initial="{ scale: 0.9, opacity: 0 }"
        :animate="{ scale: 1, opacity: 1 }"
        :transition="{ delay: 0.1, duration: 0.3, ease: 'easeOut' }"
        class="upload-content"
      >
        <div class="icon-container">
          <CloudUpload :size="48" class="pulse-icon" />
        </div>
        <h2>Syncing to Cloud Vault...</h2>
        <p>Safely encrypting and backing up your local data before shutdown.</p>
        <div class="loading-bar">
          <div class="loading-progress"></div>
        </div>
      </Motion>
    </Motion>
  </AnimatePresence>
</template>

<style scoped>
.upload-overlay {
  position: fixed;
  inset: 0;
  z-index: 99999;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.upload-content {
  background: var(--surface);
  border: 1px solid var(--line);
  padding: 48px;
  border-radius: 24px;
  box-shadow: 0 24px 64px rgba(0,0,0,0.4);
  text-align: center;
  max-width: 400px;
  width: 90%;
}

.icon-container {
  display: flex;
  justify-content: center;
  margin-bottom: 24px;
  color: var(--accent);
}

.pulse-icon {
  animation: float-pulse 2s ease-in-out infinite;
}

@keyframes float-pulse {
  0% { transform: translateY(0) scale(1); opacity: 0.8; }
  50% { transform: translateY(-8px) scale(1.05); opacity: 1; }
  100% { transform: translateY(0) scale(1); opacity: 0.8; }
}

h2 {
  color: var(--ink);
  font-size: 1.5rem;
  margin: 0 0 12px 0;
  font-weight: 700;
  letter-spacing: -0.02em;
}

p {
  color: var(--muted);
  font-size: 0.9rem;
  margin: 0 0 32px 0;
  line-height: 1.5;
}

.loading-bar {
  width: 100%;
  height: 6px;
  background: var(--surface-soft);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}

.loading-progress {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  width: 50%;
  background: var(--accent);
  border-radius: 4px;
  animation: indeterminate 1.5s cubic-bezier(0.65, 0.815, 0.735, 0.395) infinite;
}

@keyframes indeterminate {
  0% { left: -35%; right: 100%; }
  60% { left: 100%; right: -90%; }
  100% { left: 100%; right: -90%; }
}
</style>
