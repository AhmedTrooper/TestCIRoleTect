<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useInboxStore, InboxJob } from '../store/inbox';
import { useSettingsStore } from '../store/settings';
import { useDialogStore } from '../store/dialog';
import { Motion, AnimatePresence } from 'motion-v';
import { invoke } from '@tauri-apps/api/core';
import { 
  Inbox, 
  Trash2, 
  CheckCircle, 
  ExternalLink, 
  Key, 
  Wifi, 
  Info,
  Clock,
  Settings2,
  X,
  Check,
  RefreshCw,
  Cpu,
  ClipboardList,
  Filter,
  ArrowUpDown
} from '@lucide/vue';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import CustomSelect from './CustomSelect.vue';

const inboxStore = useInboxStore();
const settingsStore = useSettingsStore();
const dialog = useDialogStore();
const router = useRouter();

const activeTooltip = ref<string | null>(null);
const isSelectionMode = ref(false);
const selectedJobs = ref<Set<string>>(new Set());
const isProcessing = ref(false);
const processingProgress = ref({ current: 0, total: 0 });

// Filtering & Sorting
const statusFilter = ref('All');
const sortBy = ref('date-desc');
const statuses = ['All', 'Pending', 'Processed'];

// Individual processing states
const processingJobs = ref<Set<string>>(new Set());

onMounted(async () => {
  await inboxStore.loadJobs();
  await inboxStore.loadExtensionConfig();
});

const filteredJobs = computed(() => {
  let result = [...inboxStore.jobs];

  // Status Filter
  if (statusFilter.value !== 'All') {
    result = result.filter(j => j.status === statusFilter.value);
  }

  // Sort
  result.sort((a, b) => {
    const dateA = new Date(a.created_at).getTime();
    const dateB = new Date(b.created_at).getTime();
    return sortBy.value === 'date-desc' ? dateB - dateA : dateA - dateB;
  });

  return result;
});

const handleCardClick = (job: InboxJob) => {
  if (isSelectionMode.value) {
    if (selectedJobs.value.has(job.id)) {
      selectedJobs.value.delete(job.id);
    } else {
      selectedJobs.value.add(job.id);
    }
  } else {
    router.push(`/inbox/${job.id}`);
  }
};

const deleteJob = async (id: string) => {
  const confirmed = await dialog.showConfirm('Delete this raw job data?', 'Confirm Deletion');
  if (confirmed) {
    await inboxStore.deleteJob(id);
  }
};

const deleteSelected = async () => {
  const confirmed = await dialog.showConfirm(`Delete ${selectedJobs.value.size} selected items?`, 'Batch Delete');
  if (confirmed) {
    for (const id of selectedJobs.value) {
      await invoke('delete_inbox_job', { id });
    }
    selectedJobs.value.clear();
    isSelectionMode.value = false;
    await inboxStore.loadJobs();
  }
};

const copyToClipboard = async (text: string, label: string) => {
  await writeText(text);
  await dialog.showAlert(`${label} copied to clipboard!`, 'Copied');
};

const handleResetSecret = async () => {
  const confirmed = await dialog.showConfirm(
    'Regenerating the secret key will invalidate your current connection. You will need to update the key in your browser extension. Continue?',
    'Reset Secret Key'
  );

  if (confirmed) {
    try {
      await inboxStore.resetSecret();
      await dialog.showAlert('Secret key has been regenerated successfully.', 'Key Reset');
    } catch (err: any) {
      await dialog.showAlert(`Failed to reset secret: ${err.toString()}`, 'Error');
    }
  }
};

const processJob = async (inboxJob: InboxJob) => {
  processingJobs.value.add(inboxJob.id);
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) {
      await dialog.showAlert('Please set your AI API key in Settings first.', 'API Key Missing');
      return;
    }

    const result = await invoke<any>('parse_job', {
      provider: settingsStore.selectedAiProvider,
      model: settingsStore.selectedAiModel,
      apiKey,
      rawJd: inboxJob.raw_description,
      jobUrl: inboxJob.url
    });

    const jobPayload = {
      id: Math.random().toString(36).substring(2, 11),
      company_name: result.details.company_name,
      job_title: result.details.job_title,
      work_model: result.details.work_model,
      employment_type: result.details.employment_type,
      status: 'Drafting',
      raw_jd: inboxJob.raw_description,
      requirements: result.details.requirements.join('\n'),
      core_responsibilities: result.details.core_responsibilities.join('\n'),
      job_url: inboxJob.url,
      // other fields null
    };

    await invoke('save_job', { payload: jobPayload });
    await inboxStore.markProcessed(inboxJob.id);
    
    return true;
  } catch (error: any) {
    console.error('Processing error:', error);
    await dialog.showAlert(`Failed to process job: ${error.toString()}`, 'Error');
    return false;
  } finally {
    processingJobs.value.delete(inboxJob.id);
  }
};

const processSelected = async () => {
  if (selectedJobs.value.size === 0) return;
  
  const jobsToProcess = inboxStore.jobs.filter(j => selectedJobs.value.has(j.id));
  const alreadyProcessedCount = jobsToProcess.filter(j => j.status === 'Processed').length;

  if (alreadyProcessedCount > 0) {
    const confirmed = await dialog.showConfirm(
      `${alreadyProcessedCount} of the selected jobs have already been processed. Re-processing will create duplicate entries in your main Jobs vault. Do you want to continue?`,
      'Duplicate Warning'
    );
    if (!confirmed) return;
  }

  isProcessing.value = true;
  processingProgress.value = { current: 0, total: jobsToProcess.length };
  
  for (const job of jobsToProcess) {
    processingProgress.value.current++;
    await processJob(job);
  }

  isProcessing.value = false;
  selectedJobs.value.clear();
  isSelectionMode.value = false;
  await inboxStore.loadJobs();
  await dialog.showAlert('Batch processing complete. Processed jobs are now in your main Vault.', 'Success');
};

const getStatusClass = (status: string) => {
  return `status-badge ${status.toLowerCase()}`;
};
</script>

<template>
  <div class="inbox-container">
    <header class="page-header">
      <div class="title-group">
        <h1>Extension Inbox</h1>
        <p class="subtitle">Raw jobs captured via the browser extension.</p>
      </div>
      
      <div class="header-actions">
        <template v-if="!isSelectionMode">
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'refresh'" @mouseleave="activeTooltip = null">
            <button class="btn-secondary" @click="inboxStore.loadJobs()">
              <RefreshCw :size="18" :class="{ 'spinner': inboxStore.isLoading }" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'refresh'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Refresh
              </Motion>
            </AnimatePresence>
          </div>
          
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'selection'" @mouseleave="activeTooltip = null">
            <button class="btn-secondary" @click="isSelectionMode = true" :disabled="inboxStore.jobs.length === 0">
              <Settings2 :size="18" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'selection'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Selection Mode
              </Motion>
            </AnimatePresence>
          </div>
        </template>

        <template v-else>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'process-batch'" @mouseleave="activeTooltip = null">
            <button class="btn-primary" @click="processSelected" :disabled="selectedJobs.size === 0 || isProcessing">
              <Cpu :size="18" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'process-batch'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Process Selected ({{ selectedJobs.size }})
              </Motion>
            </AnimatePresence>
          </div>

          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'delete-batch'" @mouseleave="activeTooltip = null">
            <button class="btn-danger-outline" @click="deleteSelected" :disabled="selectedJobs.size === 0">
              <Trash2 :size="18" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'delete-batch'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Delete Selected
              </Motion>
            </AnimatePresence>
          </div>

          <button class="btn-primary" @click="isSelectionMode = false">
            <X :size="18" />
          </button>
        </template>
      </div>
    </header>

    <div class="filters-bar" v-if="!isSelectionMode">
      <div class="controls">
        <div 
          class="filter-group"
          @mouseenter="activeTooltip = 'status-filter'"
          @mouseleave="activeTooltip = null"
          style="position: relative;"
        >
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'status-filter'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              class="floating-message tooltip-top"
            >
              Filter Status
            </Motion>
          </AnimatePresence>
          <CustomSelect
            v-model="statusFilter"
            :options="statuses.map(s => ({ value: s, label: s }))"
            style="min-width: 140px;"
          >
            <template #icon>
              <Filter :size="14" style="color: var(--muted);" />
            </template>
          </CustomSelect>
        </div>

        <div 
          class="filter-group"
          @mouseenter="activeTooltip = 'sort-filter'"
          @mouseleave="activeTooltip = null"
          style="position: relative;"
        >
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'sort-filter'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              class="floating-message tooltip-top"
            >
              Sort Order
            </Motion>
          </AnimatePresence>
          <CustomSelect
            v-model="sortBy"
            :options="[
              { value: 'date-desc', label: 'Newest First' },
              { value: 'date-asc', label: 'Oldest First' }
            ]"
            style="min-width: 150px;"
          >
            <template #icon>
              <ArrowUpDown :size="14" style="color: var(--muted);" />
            </template>
          </CustomSelect>
        </div>
      </div>

      <div class="inbox-stats">
        <span>Showing {{ filteredJobs.length }} captures</span>
      </div>
    </div>

    <div class="inbox-layout">
      <div class="main-content">
        <div v-if="isProcessing" class="processing-banner">
          <div class="progress-info">
            <Cpu :size="20" class="spinner" />
            <span>AI Processing: {{ processingProgress.current }} / {{ processingProgress.total }}</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: (processingProgress.current / processingProgress.total * 100) + '%' }"></div>
          </div>
        </div>

        <div v-if="inboxStore.isLoading && inboxStore.jobs.length === 0" class="empty-state">
          <RefreshCw :size="48" class="spinner" />
          <p>Syncing inbox...</p>
        </div>

        <div v-else-if="inboxStore.jobs.length === 0" class="empty-state">
          <Inbox :size="48" class="empty-icon" />
          <h3>Inbox is empty</h3>
          <p>Use the browser extension to capture job listings.</p>
        </div>

        <div v-else class="inbox-list">
          <div 
            v-for="job in filteredJobs" 
            :key="job.id" 
            class="inbox-card"
            :class="{ 
              'selected': selectedJobs.has(job.id), 
              'selection-mode': isSelectionMode,
              'processed': job.status === 'Processed'
            }"
            @click="handleCardClick(job)"
          >
            <div class="card-header">
              <div class="left">
                <div v-if="isSelectionMode" class="checkbox" :class="{ 'checked': selectedJobs.has(job.id) }">
                  <Check v-if="selectedJobs.has(job.id)" :size="12" />
                </div>
                <span :class="getStatusClass(job.status)">{{ job.status }}</span>
              </div>
              <div class="right">
                <Clock :size="12" />
                <span>{{ job.created_at.split(' ')[0] }}</span>
              </div>
            </div>

            <div class="card-body">
              <div class="url-line" v-if="job.url">
                <ExternalLink :size="14" />
                <span class="url-text">{{ job.url }}</span>
              </div>
              <div class="description-preview">
                {{ job.raw_description.substring(0, 80) }}...
              </div>
            </div>

            <div class="card-footer" v-if="!isSelectionMode">
              <div class="actions">
                <button class="icon-btn danger" @click.stop="deleteJob(job.id)">
                  <Trash2 :size="16" />
                </button>
                <div style="display: flex; gap: 12px; align-items: center;">
                  <div v-if="job.status === 'Processed'" class="done-indicator">
                    <CheckCircle :size="14" /> Processed
                  </div>
                  <button 
                    class="process-btn" 
                    @click.stop="processJob(job)"
                    :disabled="isProcessing || processingJobs.has(job.id)"
                  >
                    <RefreshCw v-if="processingJobs.has(job.id)" :size="14" class="spinner" />
                    <Cpu v-else :size="14" /> 
                    {{ processingJobs.has(job.id) ? 'Analyzing...' : (job.status === 'Processed' ? 'Re-Process' : 'Process with AI') }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <aside class="config-sidebar">
        <div class="config-card">
          <div class="card-header">
            <Wifi :size="18" />
            <h3>Extension Link</h3>
          </div>
          <p class="config-desc">Configure your browser extension with these credentials to enable instant job capture.</p>
          
          <div class="config-item">
            <label>Server Port</label>
            <div class="value-row">
              <code>{{ inboxStore.extensionConfig?.port || '...' }}</code>
              <button class="copy-btn" @click="copyToClipboard(inboxStore.extensionConfig?.port || '', 'Port')">
                <ClipboardList :size="14" />
              </button>
            </div>
          </div>

          <div class="config-item">
            <label>Secret Key</label>
            <div class="value-row">
              <code class="secret">{{ inboxStore.extensionConfig?.secret ? '••••••••' + inboxStore.extensionConfig.secret.slice(-4) : '...' }}</code>
              <button class="copy-btn" @click="handleResetSecret" title="Regenerate Key">
                <RefreshCw :size="14" />
              </button>
              <button class="copy-btn" @click="copyToClipboard(inboxStore.extensionConfig?.secret || '', 'Secret Key')" title="Copy Key">
                <Key :size="14" />
              </button>
            </div>
          </div>

          <div class="setup-tip">
            <Info :size="14" />
            <p>Paste these into the extension settings to securely link your browser.</p>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.inbox-container {
  padding: 40px;
  max-width: 1200px;
  margin: 0 auto;
  min-height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 32px;
}

.page-header h1 { font-size: 2.2rem; margin: 0; color: var(--ink); }
.subtitle { color: var(--muted); margin: 8px 0 0; }

.filters-bar {
  background: var(--surface);
  border: 1px solid var(--line);
  padding: 12px 20px;
  border-radius: 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  box-shadow: var(--shadow);
}

.controls { display: flex; gap: 12px; }

.filter-group { display: flex; align-items: center; gap: 6px; position: relative; }

.icon-select {
  flex-direction: row;
  align-items: center;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 0 8px 0 12px;
  transition: border-color 0.2s;
}

.icon-select:focus-within {
  border-color: var(--accent);
}

.icon-indicator {
  color: var(--muted);
  display: flex;
  align-items: center;
  position: relative;
}

.filter-group select {
  padding: 8px 24px 8px 4px;
  font-weight: 700;
  cursor: pointer;
  background: transparent;
  border: none;
  font-size: 0.75rem;
  color: var(--ink);
  outline: none;
}

.inbox-stats {
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.header-actions { display: flex; gap: 12px; }

.inbox-layout {
  display: flex;
  gap: 32px;
  flex: 1;
  min-height: 400px;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-height: 0;
  min-width: 0;
}

.processing-banner {
  background: var(--accent-soft);
  border: 1px solid var(--accent);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.progress-info {
  display: flex;
  align-items: center;
  gap: 12px;
  font-weight: 700;
  color: var(--accent);
  font-size: 0.9rem;
}

.progress-bar {
  height: 6px;
  background: var(--surface-soft);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.3s ease;
}

.inbox-list {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
  overflow-y: auto;
  padding-right: 8px;
}

.inbox-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 20px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  min-width: 0;
  overflow: hidden;
}

.inbox-card:hover {
  border-color: var(--accent);
  background: var(--bg-accent);
}

.inbox-card.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.inbox-card.processed {
  opacity: 0.7;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.card-header .left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.card-header .right {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--muted);
  font-family: monospace;
}

.checkbox {
  width: 18px;
  height: 18px;
  border: 2px solid var(--line);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg);
}

.checkbox.checked {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.status-badge {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.65rem;
  font-weight: 800;
  text-transform: uppercase;
}

.status-badge.pending { background: var(--accent-soft); color: var(--accent); }
.status-badge.processed { background: var(--surface-soft); color: var(--muted); }

.card-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.url-line {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--accent);
  font-size: 0.8rem;
  font-weight: 600;
  min-width: 0;
}

.url-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1;
  display: block;
}

.description-preview {
  font-size: 0.85rem;
  color: var(--muted);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-word;
}

.card-footer {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

.actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.process-btn {
  background: var(--accent);
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 700;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
}

.done-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--muted);
}

.config-sidebar {
  width: 300px;
  flex-shrink: 0;
}

.config-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 24px;
  position: sticky;
  top: 0;
}

.config-card .card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
  color: var(--accent);
}

.config-card h3 { margin: 0; font-size: 1.1rem; color: var(--ink); }

.config-desc {
  font-size: 0.8rem;
  color: var(--muted);
  line-height: 1.5;
  margin-bottom: 24px;
}

.config-item {
  margin-bottom: 16px;
}

.config-item label {
  display: block;
  font-size: 0.65rem;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--accent);
  margin-bottom: 8px;
  letter-spacing: 0.05em;
}

.value-row {
  display: flex;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 8px;
  overflow: hidden;
}

.value-row code {
  flex: 1;
  padding: 8px 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.8rem;
  color: var(--ink);
}

.copy-btn {
  background: var(--surface-soft);
  border: none;
  border-left: 1px solid var(--line);
  padding: 0 12px;
  color: var(--muted);
  cursor: pointer;
  transition: 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0; /* Let parent overflow: hidden handle corners */
}

.copy-btn:hover {
  background: var(--accent);
  color: white;
}

.setup-tip {
  margin-top: 24px;
  display: flex;
  gap: 10px;
  background: var(--bg-accent);
  padding: 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
}

.setup-tip p {
  font-size: 0.7rem;
  margin: 0;
  color: var(--muted);
  line-height: 1.4;
}

.setup-tip svg {
  color: var(--accent);
  flex-shrink: 0;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  text-align: center;
  gap: 16px;
}

.empty-icon { opacity: 0.2; }

.btn-primary, .btn-secondary, .btn-danger-outline {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  cursor: pointer;
}

.btn-primary { background: var(--accent); color: white; border: none; }
.btn-secondary { background: var(--surface-soft); border: 1px solid var(--line); color: var(--ink); }
.btn-danger-outline { background: transparent; border: 1px solid var(--warning); color: var(--warning); }

.icon-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  transition: 0.2s;
}

.icon-btn:hover { background: var(--surface-soft); color: var(--ink); }
.icon-btn.danger:hover { color: var(--warning); background: rgba(248, 51, 73, 0.1); }

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 900px) {
  .inbox-layout { flex-direction: column; }
  .config-sidebar { width: 100%; }
}

@media (max-width: 768px) {
  .inbox-container { padding: 16px; }
  .page-header { flex-direction: column; gap: 16px; align-items: stretch; margin-bottom: 20px; }
  .filters-bar { flex-direction: column; gap: 16px; align-items: stretch; padding: 16px; margin-bottom: 16px; }
  .controls { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .inbox-stats { text-align: center; margin-top: 8px; }
  .btn-primary, .btn-secondary, .btn-danger-outline { width: 36px; height: 36px; }
  .header-actions { justify-content: flex-end; }
}

@media (max-width: 500px) {
  .inbox-card {
    padding: 16px;
  }
  .card-footer {
    margin-top: 12px;
    padding-top: 12px;
  }
  .actions {
    gap: 12px;
  }
  .process-btn {
    flex: 1;
    justify-content: center;
    padding: 10px 12px;
    font-size: 0.8rem;
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
}
</style>
