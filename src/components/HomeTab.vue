<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useSettingsStore } from '../store/settings';
import { useJobsStore, Job } from '../store/jobs';
import { Motion, AnimatePresence } from 'motion-v';

import { Activity, Plus, FileText, LayoutGrid, Mail } from '@lucide/vue';

const router = useRouter();
const route = useRoute();
const settingsStore = useSettingsStore();
const jobsStore = useJobsStore();
const settingsError = ref('');
const isLoadingSettings = ref(false);

// Tooltip State
const activeTooltip = ref<string | null>(null);

const savedJobs = ref<Job[]>([]);

const navigateToJob = (id: string) => {
  router.push(`/job/${id}`);
};

const refreshData = async () => {
  isLoadingSettings.value = true;
  settingsError.value = '';
  try {
    await settingsStore.loadSettings();
    await settingsStore.loadProviderKeyStatus(settingsStore.selectedAiProvider);
    savedJobs.value = await jobsStore.loadAllJobs();
  } catch (err: any) {
    settingsError.value = err?.message || 'Failed to load data.';
  } finally {
    isLoadingSettings.value = false;
  }
};

onMounted(refreshData);

watch(() => route.fullPath, async () => {
  if (route.name === 'Home') await refreshData();
});
</script>

<template>
  <div class="home-container">
    <div class="hero">
      <div class="status-indicator">
        <Activity :size="12" class="status-icon" />
        <span class="text">Engine Ready</span>
      </div>
      
      <h1 class="main-title">Craft your professional narrative.</h1>
      
      <p class="subtitle">
        Surgical AI tailoring for high-performance LaTeX resumes.
      </p>

      <div class="actions">
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'new-app'" @mouseleave="activeTooltip = null">
          <button class="btn-primary" @click="$router.push('/parse')"><Plus :size="18" /></button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'new-app'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-top"
            >
              New Application
            </Motion>
          </AnimatePresence>
        </div>
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'resumes'" @mouseleave="activeTooltip = null">
          <button class="btn-secondary" @click="$router.push('/resumes')"><FileText :size="18" /></button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'resumes'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-top"
            >
              Resume Templates
            </Motion>
          </AnimatePresence>
        </div>
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'cls'" @mouseleave="activeTooltip = null">
          <button class="btn-secondary" @click="$router.push('/cover-letters')"><Mail :size="18" /></button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'cls'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-top"
            >
              CL Templates
            </Motion>
          </AnimatePresence>
        </div>
      </div>
    </div>

    <div class="recent-section">
      <div class="section-header">
        <h3>RECENT APPLICATIONS</h3>
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'all-jobs'" @mouseleave="activeTooltip = null">
          <button class="link-btn" @click="$router.push('/jobs')"><LayoutGrid :size="18" /></button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'all-jobs'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              All Applications
            </Motion>
          </AnimatePresence>
        </div>
      </div>

      <div v-if="savedJobs.length === 0" class="empty-state">
        No active applications. Start by parsing a job description.
      </div>
      
      <div v-else class="list">
        <button
          v-for="job in savedJobs.slice(0, 8)"
          :key="job.id"
          class="item"
          @click="navigateToJob(job.id)"
        >
          <div class="item-main">
            <span class="item-title">{{ job.job_title }}</span>
            <span class="item-meta">{{ job.company_name }}</span>
          </div>
          <span class="item-date">{{ job.created_at?.split(' ')[0] }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-container {
  padding: 40px;
  max-width: 800px;
  margin: 0 auto;
}

.hero {
  margin-bottom: 40px;
}

.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--muted);
}
.status-icon { color: var(--accent); }

.main-title {
  font-size: 2rem;
  font-weight: 700;
  color: var(--ink);
  margin: 0 0 8px 0;
  letter-spacing: -0.01em;
}

.subtitle {
  font-size: 0.9rem;
  color: var(--muted);
  margin-bottom: 24px;
}

.actions { display: flex; gap: 12px; }

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
}

.btn-primary, .btn-secondary {
  padding: 10px;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-lg);
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: 0.15s;
}

.btn-primary { background: var(--accent); color: #fff; border: none; }
.btn-primary:hover { opacity: 0.9; }

.btn-secondary { background: var(--surface-soft); color: var(--ink); border: 1px solid var(--line); }
.btn-secondary:hover { border-color: var(--muted); }

.recent-section {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.section-header {
  padding: 12px 16px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.section-header h3 {
  font-size: 0.65rem;
  color: var(--muted);
  margin: 0;
  letter-spacing: 0.05em;
}

.link-btn {
  background: none;
  border: none;
  color: var(--accent);
  font-weight: 700;
  font-size: 0.65rem;
  cursor: pointer;
}

.list { display: flex; flex-direction: column; }

.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  background: none;
  border: none;
  border-bottom: 1px solid var(--line);
  width: 100%;
  text-align: left;
  cursor: pointer;
  transition: 0.15s;
}
.item:last-child { border-bottom: none; }
.item:hover { background: var(--surface-soft); }

.item-main {
  min-width: 0;
  flex: 1;
  padding-right: 12px;
}
.item-title { display: block; font-size: 0.85rem; font-weight: 600; color: var(--ink); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.item-meta { font-size: 0.75rem; color: var(--muted); display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.item-date { font-size: 0.7rem; color: var(--muted); font-family: monospace; white-space: nowrap; }

.empty-state {
  padding: 32px;
  text-align: center;
  color: var(--muted);
  font-size: 0.8rem;
}

@media (max-width: 600px) {
  .home-container { padding: 20px; }
  .main-title { font-size: 1.6rem; }
}
</style>
