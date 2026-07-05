<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { InboxJob } from '../store/inbox';
import { useSettingsStore } from '../store/settings';
import { useDialogStore } from '../store/dialog';
import { 
  ArrowLeft, 
  ExternalLink, 
  Cpu, 
  Trash2, 
  Clock, 
  RefreshCw,
  Globe,
  FileText,
  Copy,
  Check,
  Hash,
  Type
} from '@lucide/vue';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

const route = useRoute();
const router = useRouter();
const settingsStore = useSettingsStore();
const dialog = useDialogStore();

const job = ref<InboxJob | null>(null);
const isLoading = ref(true);
const isProcessing = ref(false);
const isCopied = ref(false);

const stats = computed(() => {
  if (!job.value) return { chars: 0, words: 0 };
  const text = job.value.raw_description || '';
  return {
    chars: text.length,
    words: text.split(/\s+/).filter(w => w.length > 0).length
  };
});

const loadJob = async () => {
  isLoading.value = true;
  try {
    const id = route.params.id as string;
    job.value = await invoke<InboxJob>('get_inbox_job_by_id', { id });
  } catch (err: any) {
    console.error('Failed to load inbox job:', err);
    await dialog.showAlert('Failed to load job details.', 'Error');
    router.push('/inbox');
  } finally {
    isLoading.value = false;
  }
};

onMounted(loadJob);

const goBack = () => router.push('/inbox');

const copyToClipboard = async () => {
  if (!job.value) return;
  await writeText(job.value.raw_description);
  isCopied.value = true;
  setTimeout(() => isCopied.value = false, 2000);
};

const processJob = async () => {
  if (!job.value || isProcessing.value) return;
  
  isProcessing.value = true;
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
      rawJd: job.value.raw_description,
      jobUrl: job.value.url
    });

    const jobPayload = {
      id: Math.random().toString(36).substring(2, 11),
      company_name: result.details.company_name,
      job_title: result.details.job_title,
      work_model: result.details.work_model,
      employment_type: result.details.employment_type,
      status: 'Drafting',
      raw_jd: job.value.raw_description,
      requirements: JSON.stringify(result.details.requirements || []),
      core_responsibilities: JSON.stringify(result.details.core_responsibilities || []),
      job_url: job.value.url,
    };

    await invoke('save_job', { payload: jobPayload });
    await invoke('mark_inbox_job_processed', { id: job.value.id });
    
    await dialog.showAlert('Job processed and moved to vault successfully!', 'Success');
    router.push('/inbox');
  } catch (error: any) {
    console.error('Processing error:', error);
    await dialog.showAlert(`Failed to process job: ${error.toString()}`, 'Error');
  } finally {
    isProcessing.value = false;
  }
};

const deleteJob = async () => {
  if (!job.value) return;
  const confirmed = await dialog.showConfirm('Are you sure you want to delete this captured job data?', 'Delete Capture');
  if (confirmed) {
    try {
      await invoke('delete_inbox_job', { id: job.value.id });
      router.push('/inbox');
    } catch (err: any) {
      await dialog.showAlert(err.toString(), 'Delete Failed');
    }
  }
};
</script>

<template>
  <div class="detail-container">
    <header class="detail-header">
      <button class="back-btn" @click="goBack">
        <ArrowLeft :size="20" />
      </button>
      <div class="header-main">
        <h1>Capture Details</h1>
        <div class="badge-row" v-if="job">
          <span :class="['status-badge', job.status.toLowerCase()]">{{ job.status }}</span>
          <span class="timestamp"><Clock :size="12" /> {{ job.created_at }}</span>
        </div>
      </div>
      <div class="header-actions" v-if="job">
        <button class="action-btn danger" @click="deleteJob" title="Delete Capture">
          <Trash2 :size="18" />
        </button>
        <button 
          v-if="job.status === 'Pending'"
          class="action-btn primary" 
          @click="processJob"
          :disabled="isProcessing"
        >
          <RefreshCw v-if="isProcessing" :size="18" class="spinner" />
          <Cpu v-else :size="18" />
          <span>{{ isProcessing ? 'Processing...' : 'Process with AI' }}</span>
        </button>
      </div>
    </header>

    <main class="detail-content">
      <div v-if="isLoading" class="loading-state">
        <RefreshCw :size="48" class="spinner" />
        <p>Loading capture data...</p>
      </div>

      <template v-else-if="job">
        <section class="info-section">
          <div class="section-card">
            <div class="card-header">
              <Globe :size="18" />
              <h3>Source URL</h3>
            </div>
            <div class="url-box">
              <a v-if="job.url" :href="job.url" target="_blank" class="job-link">
                {{ job.url }}
                <ExternalLink :size="14" />
              </a>
              <span v-else class="no-url">No URL captured</span>
            </div>
          </div>

          <div class="section-card description-card">
            <div class="card-header">
              <div class="header-left">
                <FileText :size="18" />
                <h3>Raw Content</h3>
              </div>
              <div class="header-right">
                <div class="stat-item">
                  <Hash :size="12" />
                  <span>{{ stats.chars.toLocaleString() }} chars</span>
                </div>
                <div class="stat-divider"></div>
                <div class="stat-item">
                  <Type :size="12" />
                  <span>{{ stats.words.toLocaleString() }} words</span>
                </div>
                <button class="copy-small-btn" @click="copyToClipboard" :title="isCopied ? 'Copied!' : 'Copy to clipboard'">
                  <component :is="isCopied ? Check : Copy" :size="14" />
                </button>
              </div>
            </div>
            <div class="content-box">
              <pre class="raw-text">{{ job.raw_description }}</pre>
            </div>
          </div>
        </section>
      </template>
    </main>
  </div>
</template>

<style scoped>
.detail-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}

.detail-header {
  height: 80px;
  padding: 0 40px;
  display: flex;
  align-items: center;
  gap: 24px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.back-btn {
  background: var(--surface-soft);
  border: 1px solid var(--line);
  color: var(--ink);
  width: 44px;
  height: 44px;
  border-radius: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: 0.2s;
}

.back-btn:hover { border-color: var(--accent); color: var(--accent); background: var(--bg); }

.header-main { flex: 1; }
.header-main h1 { margin: 0; font-size: 1.5rem; color: var(--ink); font-weight: 800; }

.badge-row { display: flex; align-items: center; gap: 12px; margin-top: 4px; }

.status-badge {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.65rem;
  font-weight: 800;
  text-transform: uppercase;
}
.status-badge.pending { background: var(--accent-soft); color: var(--accent); }
.status-badge.processed { background: var(--surface-soft); color: var(--muted); }

.timestamp { font-size: 0.75rem; color: var(--muted); font-family: monospace; display: flex; align-items: center; gap: 4px; }

.header-actions { display: flex; gap: 12px; }

.action-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 20px;
  height: 44px;
  border-radius: 12px;
  font-weight: 700;
  font-size: 0.9rem;
  cursor: pointer;
  transition: 0.2s;
  border: 1px solid var(--line);
}

.action-btn.primary { background: var(--accent); color: white; border: none; }
.action-btn.danger { background: transparent; color: var(--warning); border: 1px solid var(--warning); padding: 0 14px; }
.action-btn.danger:hover { background: var(--warning); color: white; }
.action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 40px;
}

.info-section {
  max-width: 1000px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.section-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 20px;
  padding: 24px;
  box-shadow: var(--shadow);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  color: var(--accent);
  margin-bottom: 20px;
}

.header-left { display: flex; align-items: center; gap: 12px; }
.header-right { display: flex; align-items: center; gap: 12px; }

.stat-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--muted);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.stat-divider { width: 1px; height: 12px; background: var(--line); }

.copy-small-btn {
  background: var(--surface-soft);
  border: 1px solid var(--line);
  color: var(--muted);
  width: 32px;
  height: 32px;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: 0.2s;
  margin-left: 8px;
}

.copy-small-btn:hover { border-color: var(--accent); color: var(--accent); }

.card-header h3 { margin: 0; font-size: 1.1rem; color: var(--ink); }

.url-box {
  background: var(--bg);
  padding: 16px;
  border-radius: 12px;
  border: 1px solid var(--line);
}

.job-link {
  color: var(--accent);
  text-decoration: none;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
  word-break: break-all;
}

.no-url { color: var(--muted); font-style: italic; }

.description-card { flex: 1; }

.content-box {
  background: var(--bg);
  border-radius: 12px;
  border: 1px solid var(--line);
  overflow: hidden;
}

.raw-text {
  margin: 0;
  padding: 24px;
  font-family: inherit;
  font-size: 0.95rem;
  line-height: 1.6;
  color: var(--ink);
  white-space: pre-wrap;
  word-break: break-word;
}

.loading-state {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  gap: 16px;
}

.spinner { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

@media (max-width: 768px) {
  .detail-header { padding: 0 20px; }
  .detail-content { padding: 20px; }
  .action-btn span { display: none; }
  .action-btn { padding: 0 14px; }
}
</style>
