<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Motion, AnimatePresence } from 'motion-v';
import { Cpu, ShieldCheck, Database, Zap } from '@lucide/vue';

const messages = [
  { text: "Initializing Core Systems", icon: Cpu },
  { text: "Unlocking Secure Vault", icon: ShieldCheck },
  { text: "Synchronizing Database", icon: Database },
  { text: "Priming AI Engines", icon: Zap }
];

const currentStep = ref(0);

onMounted(() => {
  const interval = setInterval(() => {
    if (currentStep.value < messages.length - 1) {
      currentStep.value++;
    } else {
      clearInterval(interval);
    }
  }, 800);
});
</script>

<template>
  <div class="splash-screen">
    <div class="content">
      <!-- Animated Logo -->
      <div class="logo-container">
        <Motion
          :animate="{
            scale: [1, 1.1, 1],
            rotate: [0, 5, -5, 0],
          }"
          :transition="{
            duration: 3,
            repeat: Infinity,
            ease: 'easeInOut'
          }"
          class="logo-icon"
        >
          <div class="icon-pulse"></div>
          <svg viewBox="0 0 100 100" class="main-svg">
            <defs>
              <linearGradient id="logo-grad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" style="stop-color: var(--accent); stop-opacity: 1" />
                <stop offset="100%" style="stop-color: #4cc9f0; stop-opacity: 1" />
              </linearGradient>
            </defs>
            <path 
              d="M20,30 L80,30 L80,70 L20,70 Z" 
              fill="none" 
              stroke="url(#logo-grad)" 
              stroke-width="4" 
              stroke-linejoin="round"
            />
            <path 
              d="M30,45 L70,45 M30,55 L55,55" 
              stroke="var(--accent)" 
              stroke-width="4" 
              stroke-linecap="round"
            />
            <circle cx="75" cy="25" r="5" fill="var(--accent)" />
          </svg>
        </Motion>
        <h1 class="brand-name">Roletect</h1>
      </div>

      <!-- Loading Messages -->
      <div class="loading-footer">
        <AnimatePresence mode="wait">
          <Motion
            :key="currentStep"
            :initial="{ opacity: 0, y: 10 }"
            :animate="{ opacity: 1, y: 0 }"
            :exit="{ opacity: 0, y: -10 }"
            :transition="{ duration: 0.3 }"
            class="step-item"
          >
            <component :is="messages[currentStep].icon" :size="18" class="step-icon" />
            <span>{{ messages[currentStep].text }}</span>
          </Motion>
        </AnimatePresence>
        
        <div class="progress-container">
          <Motion
            class="progress-bar"
            :initial="{ width: '0%' }"
            :animate="{ width: `${((currentStep + 1) / messages.length) * 100}%` }"
            :transition="{ duration: 0.5 }"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.splash-screen {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: #0d1117; /* Darkest theme background */
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20000;
  color: white;
  overflow: hidden;
}

.content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 60px;
  width: 100%;
  max-width: 400px;
}

.logo-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
}

.logo-icon {
  position: relative;
  width: 120px;
  height: 120px;
}

.main-svg {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 0 15px rgba(58, 134, 255, 0.4));
}

.icon-pulse {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 80%;
  height: 80%;
  background: var(--accent);
  border-radius: 50%;
  filter: blur(40px);
  opacity: 0.15;
  animation: pulse-ring 2s infinite;
}

@keyframes pulse-ring {
  0% { transform: translate(-50%, -50%) scale(0.8); opacity: 0.1; }
  50% { transform: translate(-50%, -50%) scale(1.2); opacity: 0.2; }
  100% { transform: translate(-50%, -50%) scale(0.8); opacity: 0.1; }
}

.brand-name {
  font-size: 2.5rem;
  font-weight: 900;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  background: linear-gradient(135deg, #fff 0%, #8899a6 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin: 0;
}

.loading-footer {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 0 40px;
}

.step-item {
  display: flex;
  align-items: center;
  gap: 10px;
  color: #8899a6;
  font-size: 0.85rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.step-icon {
  color: var(--accent);
}

.progress-container {
  width: 100%;
  height: 4px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), #4cc9f0);
  box-shadow: 0 0 10px rgba(58, 134, 255, 0.5);
}
</style>
