<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getName, getVersion, getTauriVersion } from '@tauri-apps/api/app';
import { type } from '@tauri-apps/plugin-os';
import { Code, Video, Heart, Copy, Check, ShieldCheck } from '@lucide/vue';
import { openUrl } from '@tauri-apps/plugin-opener';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { Motion } from 'motion-v';

const appName = ref('Roletect');
const appVersion = ref('');
const tauriVersion = ref('');
const osType = ref('');
const identifier = 'com.ahmedtrooper.roletect';
const copied = ref(false);

onMounted(async () => {
  try {
    appName.value = await getName();
    appVersion.value = await getVersion();
    tauriVersion.value = await getTauriVersion();
    osType.value = await type();
  } catch (e) {
    console.error("Failed to load app info:", e);
  }
});

const openLink = (url: string) => {
  openUrl(url).catch((err: any) => console.error('Failed to open URL:', err));
};

const copyIdentifier = async () => {
  await writeText(identifier);
  copied.value = true;
  setTimeout(() => copied.value = false, 2000);
};
</script>

<template>
  <div class="about-container">
    <Motion 
      :initial="{ opacity: 0, scale: 0.95 }"
      :animate="{ opacity: 1, scale: 1 }"
      :transition="{ duration: 0.4, ease: 'easeOut' }"
      class="about-card"
    >
      <div class="card-glow"></div>
      
      <header class="about-header">
        <div class="app-icon-container">
          <div class="app-icon-ring"></div>
          <div class="app-icon-inner">
            <div class="app-icon-core"></div>
          </div>
        </div>
        
        <div class="hero-text">
          <h1 class="app-title">{{ appName }}</h1>
          <div class="badge">
            <ShieldCheck :size="10" />
            <span>Secure Enterprise Build</span>
          </div>
        </div>
      </header>

      <div class="tagline-section">
        <p class="tagline">The precision engine for high-performance LaTeX resume tailoring.</p>
      </div>

      <div class="specs-grid">
        <div class="spec-item" @click="copyIdentifier">
          <div class="spec-header">
            <span class="spec-label">IDENTIFIER</span>
            <component :is="copied ? Check : Copy" :size="10" :class="{ 'text-accent': copied }" />
          </div>
          <span class="spec-value mono">{{ identifier }}</span>
        </div>
        <div class="spec-item">
          <div class="spec-header">
            <span class="spec-label">VERSION</span>
          </div>
          <span class="spec-value mono">v{{ appVersion }}</span>
        </div>
        <div class="spec-item">
          <div class="spec-header">
            <span class="spec-label">ENGINE</span>
          </div>
          <span class="spec-value mono">Tauri {{ tauriVersion }}</span>
        </div>
        <div class="spec-item">
          <div class="spec-header">
            <span class="spec-label">PLATFORM</span>
          </div>
          <span class="spec-value mono">{{ osType }}</span>
        </div>
      </div>

      <div class="description-box">
        <p>
          Roletect
 integrates sovereign LLM orchestration with professional TeX typesetting. 
          Built for those who treat their professional narrative as a technical specification.
        </p>
      </div>

      <div class="action-row">
        <button class="btn-premium" @click="openLink('https://github.com/AhmedTrooper/Roletect')">
          <Code :size="14" />
          <span>Source Repository</span>
        </button>
        <button class="btn-premium" @click="openLink('https://www.youtube.com/@AhmedTrooper')">
          <Video :size="14" />
          <span>Technical Demo</span>
        </button>
      </div>

      <footer class="about-footer">
        <div class="footer-line"></div>
        <div class="footer-content">
          <p>ENGINEERED BY AHMEDTROOPER</p>
          <div class="heart-pulse">
            <Heart :size="12" fill="var(--warning)" stroke="none" />
          </div>
        </div>
      </footer>
    </Motion>
  </div>
</template>

<style scoped>
.about-container {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: radial-gradient(circle at 50% -20%, rgba(35, 134, 54, 0.05), transparent 70%);
  overflow-y: auto;
}

.about-card {
  width: 100%;
  max-width: 480px;
  background: rgba(22, 25, 35, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 32px;
  padding: 48px;
  position: relative;
  box-shadow: 
    0 24px 64px rgba(0, 0, 0, 0.4),
    inset 0 1px 1px rgba(255, 255, 255, 0.05);
  overflow: hidden;
}

.card-glow {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 100%;
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--accent), transparent);
  opacity: 0.5;
}

.about-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  margin-bottom: 32px;
}

.app-icon-container {
  position: relative;
  width: 80px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.app-icon-ring {
  position: absolute;
  inset: 0;
  border: 1px solid var(--line);
  border-radius: 24px;
  transform: rotate(45deg);
}

.app-icon-inner {
  width: 56px;
  height: 56px;
  background: var(--bg-accent);
  border: 1px solid var(--line);
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
}

.app-icon-core {
  width: 12px;
  height: 12px;
  background: var(--accent);
  border-radius: 50%;
  box-shadow: 0 0 20px var(--accent);
}

.hero-text {
  text-align: center;
}

.app-title {
  font-size: 2rem;
  font-weight: 900;
  color: var(--ink);
  margin: 0;
  letter-spacing: -0.04em;
  background: linear-gradient(to bottom, #fff, #888);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: rgba(35, 134, 54, 0.1);
  border: 1px solid rgba(35, 134, 54, 0.2);
  padding: 4px 10px;
  border-radius: 100px;
  margin-top: 8px;
}

.badge span {
  font-size: 0.6rem;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--accent);
}

.tagline-section {
  text-align: center;
  margin-bottom: 32px;
}

.tagline {
  font-size: 0.9rem;
  color: var(--muted);
  line-height: 1.5;
  margin: 0;
}

.specs-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 32px;
}

.spec-item {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 16px;
  transition: all 0.2s ease;
  cursor: default;
}

.spec-item:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: var(--muted);
}

.spec-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
  color: var(--muted);
}

.spec-label {
  font-size: 0.6rem;
  font-weight: 800;
  letter-spacing: 0.1em;
}

.spec-value {
  font-size: 0.8rem;
  color: var(--ink);
  display: block;
}

.mono {
  font-family: 'JetBrains Mono', monospace;
}

.description-box {
  background: var(--bg-accent);
  border-radius: 20px;
  padding: 20px;
  margin-bottom: 32px;
  border-left: 3px solid var(--accent);
}

.description-box p {
  font-size: 0.8rem;
  line-height: 1.6;
  color: var(--muted);
  margin: 0;
}

.action-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 40px;
}

.btn-premium {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 12px;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 12px;
  color: var(--ink);
  font-size: 0.75rem;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-premium:hover {
  border-color: var(--accent);
  background: var(--surface);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.about-footer {
  text-align: center;
}

.footer-line {
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--line), transparent);
  margin-bottom: 20px;
}

.footer-content {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.footer-content p {
  font-size: 0.6rem;
  font-weight: 800;
  color: var(--muted);
  letter-spacing: 0.2em;
  margin: 0;
}

.heart-pulse {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% { transform: scale(1); opacity: 0.8; }
  50% { transform: scale(1.2); opacity: 1; }
  100% { transform: scale(1); opacity: 0.8; }
}

.text-accent {
  color: var(--accent);
}

@media (max-width: 480px) {
  .specs-grid { grid-template-columns: 1fr; }
  .action-row { grid-template-columns: 1fr; }
  .about-card { padding: 32px; }
}
</style>
>
