<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { useSettingsStore } from '../store/settings';
import { 
  CheckCircle, 
  Info, 
  Save, 
  RotateCcw, 
  Download, 
  Database, 
  Upload, 
  RefreshCw,
  Palette,
  Plus,
  Trash2,
  Type,
  Italic,
  Play,
  DownloadCloud
} from '@lucide/vue';
import { Motion, AnimatePresence } from 'motion-v';
import { invoke } from '@tauri-apps/api/core';
import { save as saveDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs';

import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useDialogStore } from '../store/dialog';
import { ask } from '@tauri-apps/plugin-dialog';
import CustomSelect from './CustomSelect.vue';

const store = useSettingsStore();
const dialog = useDialogStore();

// Tooltip State
const activeTooltip = ref<string | null>(null);

// Theme State
const isImportingTheme = ref(false);
const customThemeJson = ref('');
const themeError = ref('');

// Typography Data
const fontFamilies = [
  { id: 'Inter', name: 'Inter (Sans)' },
  { id: 'Geist Sans', name: 'Geist (Modern)' },
  { id: 'Merriweather', name: 'Merriweather (Serif)' },
  { id: 'JetBrains Mono', name: 'JetBrains Mono (Code)' }
];

const fontWeights = [
  { id: '300', name: 'Light' },
  { id: '400', name: 'Regular' },
  { id: '500', name: 'Medium' },
  { id: '600', name: 'Semi-Bold' },
  { id: '700', name: 'Bold' }
];

const fontStyles = [
  { id: 'normal', name: 'Normal' },
  { id: 'italic', name: 'Italic' }
];

const copyDemoTheme = async () => {
  const demoTheme = {
    name: "Surgical Neon",
    colors: {
      "--bg": "#0a0a0a",
      "--bg-accent": "#121212",
      "--surface": "#1a1a1a",
      "--surface-soft": "#242424",
      "--ink": "#ffffff",
      "--muted": "#666666",
      "--line": "#333333",
      "--accent": "#00ff9d",
      "--accent-soft": "rgba(0, 255, 157, 0.1)",
      "--warning": "#ff3e3e"
    }
  };
  
  try {
    await writeText(JSON.stringify(demoTheme, null, 2));
    await dialog.showAlert('Demo theme JSON copied to clipboard!', 'Success');
  } catch (err) {
    console.error('Failed to copy to clipboard:', err);
  }
};

// --- 1. Draft State (Local only) ---
const providerInput = ref('');
const modelInput = ref('');
const apiKeyInput = ref('');
const customBaseUrlInput = ref('');
const customModelInput = ref('');
const savedCustomBaseUrl = ref('');
const savedCustomModel = ref('');

// UI feedback states
const isSaving = ref(false);
const isExporting = ref(false);
const isImporting = ref(false);
const isClearingCache = ref(false);
const showSuccess = ref(false);
const saveError = ref('');

const handleClearCache = async () => {
  const confirmed = await dialog.showConfirm(
    'This will delete the entire Tectonic cache. It will be rebuilt automatically during the next compilation, which may take some time. Proceed?',
    'Purge LaTeX Cache'
  );
  if (!confirmed) return;

  isClearingCache.value = true;
  try {
    await invoke('clear_tectonic_cache');
    await dialog.showAlert('Tectonic cache has been successfully purged.', 'Cache Cleared');
  } catch (err: any) {
    await dialog.showAlert(`Failed to clear cache: ${err.toString()}`, 'Error');
  } finally {
    isClearingCache.value = false;
  }
};

const exportData = async () => {
  isExporting.value = true;
  try {
    const data = await invoke('export_all_data');
    const now = new Date();
    const timestamp = now.toISOString().replace(/[:.]/g, '-').split('T');
    const dateStr = timestamp[0];
    const timeStr = timestamp[1].split('Z')[0];
    
    const path = await saveDialog({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: `roletect_backup_${dateStr}_${timeStr}.json`
    });
    
    if (path) {
      await writeTextFile(path, JSON.stringify(data, null, 2));
    }
  } catch (error: any) {
    saveError.value = error.toString();
  } finally {
    isExporting.value = false;
  }
};

const handleImport = async (mode: 'merge' | 'overwrite') => {
  if (mode === 'overwrite') {
    const confirmed = await ask(
      'Are you absolutely sure you want to overwrite your entire local vault? This action cannot be undone and will permanently delete any unsaved local changes.',
      { title: 'Overwrite Local Vault?', kind: 'warning' }
    );
    if (!confirmed) return;
  }

  const path = await openDialog({
    filters: [{ name: 'JSON', extensions: ['json'] }],
    multiple: false
  });

  if (!path) return;

  isImporting.value = true;
  try {
    const content = await readTextFile(path as string);
    const data = JSON.parse(content);
    await invoke('import_data', { data, mode });
    await dialog.showAlert(`Successfully ${mode === 'merge' ? 'synchronized' : 'restored'} your vault. The application will now reload to apply changes.`, 'Import Successful');
    window.location.reload();
  } catch (error: any) {
    saveError.value = `Import Error: ${error.toString()}`;
  } finally {
    isImporting.value = false;
  }
};

// --- S3 Backup State ---
const s3Endpoint = ref('');
const s3Bucket = ref('');
const s3Region = ref('us-east-1');
const s3AccessKey = ref('');
const hasS3AccessKey = ref(false);
const hasS3SecretKey = ref(false);
const s3SecretKey = ref('');
const s3ForcePathStyle = ref(true);
const isTestingS3 = ref(false);
const s3TestError = ref('');
const s3TestSuccess = ref(false);
const isSavingS3 = ref(false);
const s3SetupOk = ref(false);
const s3LastUpload = ref('Never');

const autoCloudBackup = ref(true);
const autoLocalBackup = ref(true);

const handleTestS3 = async (silent = false) => {
  if (!silent) {
    isTestingS3.value = true;
    s3TestError.value = '';
    s3TestSuccess.value = false;
  }
  
  try {
    let ak = s3AccessKey.value.trim();
    if (!ak) ak = await store.getSecret('s3_access_key') || '';
    
    let sk = s3SecretKey.value.trim();
    if (!sk) sk = await store.getSecret('s3_secret_key') || '';
    
    const res = await invoke<string>('test_s3_connection', {
      endpointUrl: s3Endpoint.value.trim(),
      bucketName: s3Bucket.value.trim(),
      region: s3Region.value.trim(),
      accessKeyId: ak,
      secretAccessKey: sk,
      forcePathStyle: s3ForcePathStyle.value
    });
    
    if (!silent) {
      s3TestSuccess.value = true;
      await dialog.showAlert(res, 'S3 Connection Succeeded');
    }
    return true;
  } catch (err: any) {
    if (!silent) s3TestError.value = err.toString();
    return false;
  } finally {
    if (!silent) isTestingS3.value = false;
  }
};

const handleSaveS3 = async () => {
  isSavingS3.value = true;
  s3TestError.value = '';
  s3TestSuccess.value = false;
  
  try {
    // Implicit test on save
    const testPassed = await handleTestS3(true);
    if (!testPassed) {
      s3SetupOk.value = false;
      await invoke('save_setting', { key: 's3_setup_ok', value: 'false' });
      throw new Error('Connection test failed. Please verify credentials/endpoint.');
    }
    
    await invoke('save_setting', { key: 's3_endpoint_url', value: s3Endpoint.value.trim() });
    await invoke('save_setting', { key: 's3_bucket_name', value: s3Bucket.value.trim() });
    await invoke('save_setting', { key: 's3_region', value: s3Region.value.trim() });
    await invoke('save_setting', { key: 's3_force_path_style', value: s3ForcePathStyle.value ? 'true' : 'false' });
    await invoke('save_setting', { key: 's3_setup_ok', value: 'true' });
    s3SetupOk.value = true;
    
    if (s3AccessKey.value.trim()) {
      await store.saveSecret('s3_access_key', s3AccessKey.value.trim());
    }
    if (s3SecretKey.value.trim()) {
      await store.saveSecret('s3_secret_key', s3SecretKey.value.trim());
    }
    
    s3TestSuccess.value = true;
    setTimeout(() => { s3TestSuccess.value = false; }, 3000);
  } catch (err: any) {
    s3TestError.value = err.toString();
  } finally {
    isSavingS3.value = false;
  }
};

interface BackupEntry {
  key: string;
  size: number;
  last_modified: string;
}

const isFetchingBackups = ref(false);
const fetchBackupsError = ref('');
const availableBackups = ref<BackupEntry[]>([]);
const selectedBackup = ref('');
const restoreMode = ref('merge');
const isRestoringBackup = ref(false);

const availableBackupsOptions = computed(() => {
  return availableBackups.value.map(b => ({
    value: b.key,
    label: `${b.key} (${Math.round(b.size / 1024)} KB)`
  }));
});

const restoreModeOptions = [
  { value: 'merge', label: 'Merge (Keep Existing Local Data)' },
  { value: 'overwrite', label: 'Overwrite All Local Data' }
];

const handleFetchBackups = async () => {
  isFetchingBackups.value = true;
  fetchBackupsError.value = '';
  availableBackups.value = [];
  selectedBackup.value = '';
  
  try {
    let ak = await store.getSecret('s3_access_key') || '';
    let sk = await store.getSecret('s3_secret_key') || '';
    
    const backups = await invoke<BackupEntry[]>('list_s3_backups', {
      accessKeyId: ak,
      secretAccessKey: sk
    });
    
    // Sort descending by last_modified
    availableBackups.value = backups.sort((a, b) => b.last_modified.localeCompare(a.last_modified));
    if (availableBackups.value.length > 0) {
      selectedBackup.value = availableBackups.value[0].key;
    } else {
      fetchBackupsError.value = 'No backups found in the bucket.';
    }
  } catch (err: any) {
    fetchBackupsError.value = err.toString();
  } finally {
    isFetchingBackups.value = false;
  }
};

const handleRestoreBackup = async () => {
  if (!selectedBackup.value) return;
  
  const modeText = restoreMode.value === 'overwrite' ? 'OVERWRITE ALL LOCAL DATA' : 'merge the cloud backup into your local data';
  
  const confirmed = await ask(
    `Are you sure you want to restore "${selectedBackup.value}"?\n\nWARNING: This will ${modeText}. The application will reload upon success.`,
    { title: 'Restore from Cloud Backup?', kind: 'warning' }
  );
  
  if (!confirmed) return;
  
  isRestoringBackup.value = true;
  try {
    let ak = await store.getSecret('s3_access_key') || '';
    let sk = await store.getSecret('s3_secret_key') || '';
    
    await invoke('restore_from_s3', {
      accessKeyId: ak,
      secretAccessKey: sk,
      key: selectedBackup.value,
      mode: restoreMode.value
    });
    
    await dialog.showAlert('Restore successful! The application will now reload to apply the restored data.', 'Restore Complete');
    window.location.reload();
  } catch (err: any) {
    await dialog.showAlert(err.toString(), 'Restore Failed');
  } finally {
    isRestoringBackup.value = false;
  }
};

// removed duplicate clearDataDialog

// --- 2. Configuration Data ---
const providers = [
  { id: 'openai', name: 'OpenAI' },
  { id: 'gemini', name: 'Google Gemini' },
  { id: 'anthropic', name: 'Anthropic Claude' },
  { id: 'groq', name: 'Groq (Ultra-Fast)' },
  { id: 'bedrock', name: 'AWS Bedrock' },
  { id: 'ollama', name: 'Ollama (Local LLM)' },
  { id: 'openrouter', name: 'OpenRouter' },
  { id: 'deepseek', name: 'DeepSeek' }
];

const modelsByProvider: Record<string, {id: string, name: string}[]> = {
  openrouter: [
    // --- Gemini 2.x and 2.5 ---
    { id: 'google/gemini-2.5-pro', name: 'Gemini 2.5 Pro' },
    { id: 'google/gemini-2.5-flash', name: 'Gemini 2.5 Flash' },
    { id: 'google/gemini-2.0-flash-exp', name: 'Gemini 2.0 Flash Exp' },
    
    // --- Claude Models ---
    { id: 'anthropic/claude-3.7-sonnet', name: 'Claude 3.7 Sonnet' },
    { id: 'anthropic/claude-3.7-sonnet:thinking', name: 'Claude 3.7 Sonnet (Thinking)' },
    { id: 'anthropic/claude-3.5-sonnet', name: 'Claude 3.5 Sonnet' },
    { id: 'anthropic/claude-3.5-sonnet:beta', name: 'Claude 3.5 Sonnet (Beta)' },
    { id: 'anthropic/claude-3-opus', name: 'Claude 3 Opus' },
    { id: 'anthropic/claude-3-haiku', name: 'Claude 3 Haiku' },
    { id: 'anthropic/claude-3.5-haiku', name: 'Claude 3.5 Haiku' },

    // --- DeepSeek Models ---
    { id: 'deepseek/deepseek-chat', name: 'DeepSeek V3' },
    { id: 'deepseek/deepseek-r1', name: 'DeepSeek R1' },
    { id: 'deepseek/deepseek-r1:free', name: 'DeepSeek R1 (Free)' },
    { id: 'deepseek/deepseek-chat:free', name: 'DeepSeek V3 (Free)' },

    // --- Llama Models ---
    { id: 'meta-llama/llama-3.3-70b-instruct', name: 'Llama 3.3 70B Instruct' },
    { id: 'meta-llama/llama-3.1-405b-instruct', name: 'Llama 3.1 405B Instruct' },
    { id: 'meta-llama/llama-3.1-70b-instruct', name: 'Llama 3.1 70B Instruct' },
    { id: 'meta-llama/llama-3.1-8b-instruct', name: 'Llama 3.1 8B Instruct' },
    { id: 'meta-llama/llama-3.2-3b-instruct', name: 'Llama 3.2 3B Instruct' },
    { id: 'meta-llama/llama-3.2-11b-vision-instruct', name: 'Llama 3.2 11B Vision' },
    { id: 'meta-llama/llama-3.2-90b-vision-instruct', name: 'Llama 3.2 90B Vision' },

    // --- Qwen & Mistral Models ---
    { id: 'qwen/qwen-2.5-72b-instruct', name: 'Qwen 2.5 72B Instruct' },
    { id: 'qwen/qwen-2.5-coder-32b-instruct', name: 'Qwen 2.5 Coder 32B' },
    { id: 'qwen/qwen-2.5-7b-instruct', name: 'Qwen 2.5 7B' },
    { id: 'mistralai/mistral-large', name: 'Mistral Large 2411' },
    { id: 'mistralai/pixtral-large-2411', name: 'Pixtral Large' },
    { id: 'mistralai/mistral-7b-instruct', name: 'Mistral 7B Instruct' },

    // --- OpenAI Models ---
    { id: 'openai/gpt-4o', name: 'GPT-4o' },
    { id: 'openai/gpt-4o-mini', name: 'GPT-4o Mini' },
    { id: 'openai/o1', name: 'o1' },
    { id: 'openai/o1-mini', name: 'o1 Mini' },
    { id: 'openai/o1-preview', name: 'o1 Preview' },
    { id: 'openai/o3-mini', name: 'o3 Mini' },
    { id: 'openai/o3-mini:high', name: 'o3 Mini (High Reasoning)' },
    { id: 'openai/gpt-4-turbo', name: 'GPT-4 Turbo' }
  ],
  openai: [
    // --- Legacy / Very Old ---
    { id: 'gpt-1', name: 'GPT-1 (Historical)' },
    { id: 'gpt-2', name: 'GPT-2 (Historical)' },
    { id: 'text-ada-001', name: 'GPT-3 Ada (Legacy)' },
    { id: 'text-babbage-001', name: 'GPT-3 Babbage (Legacy)' },
    { id: 'text-curie-001', name: 'GPT-3 Curie (Legacy)' },
    { id: 'text-davinci-003', name: 'GPT-3 Davinci (Legacy)' },
    { id: 'gpt-3.5-turbo-0301', name: 'GPT-3.5 Turbo (Initial)' },
    { id: 'gpt-3.5-turbo-16k', name: 'GPT-3.5 Turbo 16k (Legacy)' },
    { id: 'gpt-3.5-turbo', name: 'GPT-3.5 Turbo (Final)' },
    
    // --- GPT-4 Era ---
    { id: 'gpt-4-0314', name: 'GPT-4 (Initial)' },
    { id: 'gpt-4-32k', name: 'GPT-4 32k' },
    { id: 'gpt-4-1106-preview', name: 'GPT-4 Turbo Preview' },
    { id: 'gpt-4-turbo', name: 'GPT-4 Turbo' },
    { id: 'gpt-4o', name: 'GPT-4o (Omni)' },
    { id: 'gpt-4o-mini', name: 'GPT-4o Mini' },
    { id: 'gpt-4.5', name: 'GPT-4.5 (Legacy)' },
    
    // --- Early Reasoning Era ---
    { id: 'o1-preview', name: 'o1 Preview' },
    { id: 'o1-mini', name: 'o1 Mini' },
    { id: 'o1', name: 'o1' },
    { id: 'o3', name: 'o3 (Reasoning)' },
    { id: 'o4-mini', name: 'o4 Mini' },

    // --- GPT-5 Era (Current) ---
    { id: 'gpt-5-nano', name: 'GPT-5 Nano' },
    { id: 'gpt-5-mini', name: 'GPT-5 Mini' },
    { id: 'gpt-5-main', name: 'GPT-5 Foundation' },
    { id: 'gpt-5.1', name: 'GPT-5.1' },
    { id: 'gpt-5.2', name: 'GPT-5.2' },
    { id: 'gpt-5.3-instant', name: 'GPT-5.3 Instant' },
    { id: 'gpt-5.3-codex-spark', name: 'GPT-5.3 Codex Spark (Real-time Coding)' },
    { id: 'gpt-5.4-nano', name: 'GPT-5.4 Nano' },
    { id: 'gpt-5.4-mini', name: 'GPT-5.4 Mini' },
    { id: 'gpt-5.4', name: 'GPT-5.4 Standard' },
    { id: 'gpt-5.5-instant', name: 'GPT-5.5 Instant' },
    { id: 'gpt-5.5', name: 'GPT-5.5' },
    { id: 'gpt-5.5-thinking', name: 'GPT-5.5 Thinking' },
    { id: 'gpt-5.5-pro', name: 'GPT-5.5 Pro (Thinking)' },
    
    // --- Specialty / Open ---
    { id: 'gpt-rosalind', name: 'GPT-Rosalind (Life Sciences)' },
    { id: 'chatgpt-images-2', name: 'ChatGPT Images 2.0' }
  ],
  
  gemini: [
    // --- Legacy / Very Old ---
    { id: 'text-bison-001', name: 'PaLM (Text Bison)' },
    { id: 'chat-bison-001', name: 'PaLM (Chat Bison)' },
    { id: 'gemini-1.0-nano', name: 'Gemini 1.0 Nano (Legacy)' },
    { id: 'gemini-1.0-pro', name: 'Gemini 1.0 Pro (Legacy)' },
    { id: 'gemini-1.0-ultra', name: 'Gemini 1.0 Ultra (Legacy)' },
    { id: 'gemini-1.5-flash', name: 'Gemini 1.5 Flash (Legacy)' },
    { id: 'gemini-1.5-pro', name: 'Gemini 1.5 Pro (Legacy)' },

    // --- Gemini 2.x Era ---
    { id: 'gemini-2.0-flash-lite', name: 'Gemini 2.0 Flash-Lite (Legacy)' },
    { id: 'gemini-2.0-flash', name: 'Gemini 2.0 Flash (Legacy)' },
    { id: 'gemini-2.0-pro', name: 'Gemini 2.0 Pro (Legacy)' },
    { id: 'gemini-2.5-flash-lite', name: 'Gemini 2.5 Flash-Lite' },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash (Stable)' },
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro (Stable)' },

    // --- Gemini 3.x Era (Current) ---
    { id: 'gemini-3-flash-preview', name: 'Gemini 3.0 Flash (Preview)' },
    { id: 'gemini-3-deep-think', name: 'Gemini 3.0 Deep Think' },
    { id: 'gemini-3-pro-preview', name: 'Gemini 3.0 Pro (Preview)' },
    { id: 'gemini-3.1-flash-lite-preview', name: 'Gemini 3.1 Flash-Lite (Preview)' },
    { id: 'gemini-3.1-flash-lite', name: 'Gemini 3.1 Flash-Lite (Stable/GA)' },
    { id: 'gemini-3.1-flash-preview', name: 'Gemini 3.1 Flash (Preview)' },
    { id: 'gemini-3.1-pro-preview', name: 'Gemini 3.1 Pro (Preview)' },
    
    // --- Specialty / Agents / Vision ---
    { id: 'gemini-3.1-flash-image', name: 'Gemini 3.1 Flash Image (Nano Banana 2)' },
    { id: 'nano-banana-pro', name: 'Nano Banana Pro (Studio Quality)' },
    { id: 'deep-research-preview-04-2026', name: 'Deep Research Agent (Preview)' },
    { id: 'deep-research-max-preview-04-2026', name: 'Deep Research Max (Preview)' },
    { id: 'veo-3.1-lite-generate-preview', name: 'Veo 3.1 Lite Video (Preview)' },
    { id: 'gemini-robotics-er-1.6-preview', name: 'Gemini Robotics-ER 1.6' },
    { id: 'gemini-embedding-2', name: 'Gemini Embedding 2 (GA)' },
    { id: 'gemma-4-26b-a4b-it', name: 'Gemma 4 26B IT' },
    { id: 'gemma-4-31b-it', name: 'Gemma 4 31B IT' }
  ],
  
  anthropic: [
    // --- Legacy / Very Old ---
    { id: 'claude-1', name: 'Claude 1.0 (Legacy)' },
    { id: 'claude-instant-1.2', name: 'Claude Instant 1.2 (Legacy)' },
    { id: 'claude-2.0', name: 'Claude 2.0 (Legacy)' },
    { id: 'claude-2.1', name: 'Claude 2.1 (Legacy)' },
    
    // --- Claude 3 Era ---
    { id: 'claude-3-haiku-20240307', name: 'Claude 3 Haiku' },
    { id: 'claude-3-sonnet-20240229', name: 'Claude 3 Sonnet' },
    { id: 'claude-3-opus-20240229', name: 'Claude 3 Opus' },
    { id: 'claude-3-5-haiku-latest', name: 'Claude 3.5 Haiku' },
    { id: 'claude-3-5-sonnet-latest', name: 'Claude 3.5 Sonnet (Legacy)' },
    { id: 'claude-3-7-sonnet-latest', name: 'Claude 3.7 Sonnet' },
    
    // --- Claude 4 Era (Current) ---
    { id: 'claude-4-sonnet', name: 'Claude 4 Sonnet' },
    { id: 'claude-4-opus', name: 'Claude 4 Opus' },
    { id: 'claude-4-1-opus', name: 'Claude 4.1 Opus' },
    { id: 'claude-haiku-4-5', name: 'Claude Haiku 4.5 (Speed)' },
    { id: 'claude-sonnet-4-5', name: 'Claude Sonnet 4.5' },
    { id: 'claude-opus-4-5', name: 'Claude Opus 4.5' },
    { id: 'claude-sonnet-4-6', name: 'Claude Sonnet 4.6 (Balanced)' },
    { id: 'claude-opus-4-6', name: 'Claude Opus 4.6' },
    { id: 'claude-opus-4-7', name: 'Claude Opus 4.7 (Power)' },
    
    // --- Specialty ---
    { id: 'claude-mythos-preview', name: 'Claude Mythos Preview (Cybersecurity)' },
    { id: 'claude-cowork', name: 'Claude Cowork (Multi-step Agent)' }
  ],
  
  groq: [
    // --- Legacy / Very Old ---
    { id: 'llama2-70b-4096', name: 'Llama 2 70B (Legacy)' },
    { id: 'mixtral-8x7b-32768', name: 'Mixtral 8x7B (Legacy)' },
    { id: 'gemma-7b-it', name: 'Gemma 7B IT (Legacy)' },
    { id: 'llama3-8b-8192', name: 'Llama 3 8B (Legacy)' },
    { id: 'llama3-70b-8192', name: 'Llama 3 70B (Legacy)' },
    
    // --- Llama 3.x Era ---
    { id: 'llama-3.1-8b-instant', name: 'Llama 3.1 8B Instant' },
    { id: 'llama-3.1-70b-versatile', name: 'Llama 3.1 70B Versatile' },
    { id: 'llama-3.2-11b-vision-preview', name: 'Llama 3.2 11B Vision' },
    { id: 'llama-3.2-90b-vision-preview', name: 'Llama 3.2 90B Vision' },
    { id: 'llama-3.3-70b-versatile', name: 'Llama 3.3 70B' },
    { id: 'llama-3.3-70b-specdec', name: 'Llama 3.3 SpecDec' },

    // --- Llama 4 Era (Current) ---
    { id: 'meta-llama/llama-4-scout-17b-16e-instruct', name: 'Llama 4 Scout (17B x 16E)' },
    { id: 'meta-llama/llama-4-maverick-17b-128e-instruct', name: 'Llama 4 Maverick (17B x 128E)' },
    { id: 'meta-llama/llama-4-maverick-400b-instruct', name: 'Llama 4 Maverick (400B)' },

    // --- OpenAI Open Weights on Groq ---
    { id: 'openai/gpt-oss-20b', name: 'GPT-OSS 20B' },
    { id: 'openai/gpt-oss-safeguard-20b', name: 'GPT-OSS Safeguard 20B' },
    { id: 'openai/gpt-oss-120b', name: 'GPT-OSS 120B (Groq Flagship)' },

    // --- Mistral ---
    { id: 'mistral-medium-3.5', name: 'Mistral Medium 3.5' },
    { id: 'mistral-small-4', name: 'Mistral Small 4' },

    // --- DeepSeek, Qwen & Others ---
    { id: 'deepseek-v4-flash', name: 'DeepSeek-V4 Flash' },
    { id: 'deepseek-v4-pro', name: 'DeepSeek-V4 Pro' },
    { id: 'qwen-3-32b', name: 'Qwen3 32B' },
    { id: 'qwen/qwen3-vl-32b-instruct', name: 'Qwen 3 VL 32B Instruct' },
    { id: 'glm-5.1', name: 'GLM-5.1' },
    { id: 'moonshotai/kimi-k2-instruct-0905', name: 'Kimi K2 Instruct (Reasoning)' },
    { id: 'minimaxai/minimax-m2.5', name: 'Minimax M2.5 (Enterprise)' }
  ],
  
  bedrock: [
    // --- Deep Reasoning & Thinking (Heavy Logic, Code, Math) ---
    { id: 'deepseek.r1-v1:0', name: 'DeepSeek R1' },
    { id: 'anthropic.claude-opus-4-8', name: 'Claude Opus 4.8' },
    { id: 'anthropic.claude-opus-4-7', name: 'Claude Opus 4.7' },
    { id: 'anthropic.claude-opus-4-6', name: 'Claude Opus 4.6' },
    { id: 'openai.gpt-5-5-v1:0', name: 'GPT-5.5' },
    { id: 'moonshot.kimi-k2-thinking-v1:0', name: 'Kimi K2 Thinking' },
    { id: 'mistral.mistral-large-2407-v1:0', name: 'Mistral Large 3' },
    { id: 'mistral.devstral-2-123b-v1:0', name: 'Devstral 2 123B' },
    { id: 'qwen.qwen3-coder-next-v1:0', name: 'Qwen3 Coder Next' },
    { id: 'ai21.jamba-1-5-large-v1:0', name: 'Jamba 1.5 Large' },

    // --- General Purpose & Balanced (RAG, Standard Chat) ---
    { id: 'anthropic.claude-sonnet-4-6', name: 'Claude Sonnet 4.6' },
    { id: 'anthropic.claude-sonnet-4-5', name: 'Claude Sonnet 4.5' },
    { id: 'anthropic.claude-3-5-sonnet-20241022-v2:0', name: 'Claude 3.5 Sonnet v2' },
    { id: 'openai.gpt-5-4-v1:0', name: 'GPT-5.4' },
    { id: 'amazon.nova-pro-v1:0', name: 'Amazon Nova Pro' },
    { id: 'meta.llama4-maverick-17b-instruct-v1:0', name: 'Llama 4 Maverick 17B Instruct' },
    { id: 'meta.llama4-scout-17b-instruct-v1:0', name: 'Llama 4 Scout 17B Instruct' },
    { id: 'meta.llama3-3-70b-instruct-v1:0', name: 'Llama 3.3 70B Instruct' },
    { id: 'google.gemma-3-27b-pt-v1:0', name: 'Gemma 3 27B PT' },
    { id: 'z-ai.glm-4-7-v1:0', name: 'GLM 4.7' },

    // --- Light & Fast (High Throughput, Simple Classification) ---
    { id: 'anthropic.claude-haiku-4-5', name: 'Claude Haiku 4.5' },
    { id: 'anthropic.claude-3-5-haiku-20241022-v1:0', name: 'Claude 3.5 Haiku' },
    { id: 'amazon.nova-2-lite-v1:0', name: 'Amazon Nova 2 Lite' },
    { id: 'amazon.nova-sonic-v1:0', name: 'Amazon Nova Sonic' },
    { id: 'amazon.nova-micro-v1:0', name: 'Amazon Nova Micro' },
    { id: 'deepseek.v3-2-v1:0', name: 'DeepSeek V3.2' },
    { id: 'deepseek.v3-1-v1:0', name: 'DeepSeek V3.1' },
    { id: 'mistral.ministral-3-8b-v1:0', name: 'Ministral 3 8B' },
    { id: 'z-ai.glm-4-7-flash-v1:0', name: 'GLM 4.7 Flash' },
    { id: 'ai21.jamba-1-5-mini-v1:0', name: 'Jamba 1.5 Mini' },

    // --- Vision, Multimodal & Specialized ---
    { id: 'meta.llama3-2-90b-instruct-v1:0', name: 'Llama 3.2 90B Vision' },
    { id: 'meta.llama3-2-11b-instruct-v1:0', name: 'Llama 3.2 11B Vision' },
    { id: 'mistral.pixtral-large-2502-v1:0', name: 'Pixtral Large' },
    { id: 'qwen.qwen3-vl-235b-v1:0', name: 'Qwen3 VL 235B' },
    { id: 'google.gemma-3-12b-it-v1:0', name: 'Gemma 3 12B IT' },
    { id: 'moonshot.kimi-k2-5-v1:0', name: 'Kimi K2.5' },
    { id: 'minimax.m2-5-v1:0', name: 'MiniMax M2.5' },
    { id: 'writer.palmyra-vision-7b-v1:0', name: 'Palmyra Vision 7B' },
    { id: 'writer.palmyra-x5-v1:0', name: 'Palmyra X5' },
    { id: 'nvidia.nemotron-3-super-120b-v1:0', name: 'NVIDIA Nemotron 3 Super 120B' }
  ],
  ollama: [
    { id: 'llama3', name: 'Llama 3 (8B)' },
    { id: 'llama3.2', name: 'Llama 3.2 (3B)' },
    { id: 'llama3.2:1b', name: 'Llama 3.2 (1B)' },
    { id: 'llama3.3', name: 'Llama 3.3 (70B)' },
    { id: 'phi3', name: 'Phi 3 (3.8B)' },
    { id: 'gemma2', name: 'Gemma 2 (9B)' },
    { id: 'gemma2:2b', name: 'Gemma 2 (2B)' },
    { id: 'mistral', name: 'Mistral (7B)' },
    { id: 'codegemma', name: 'CodeGemma (7B)' },
    { id: 'codellama', name: 'Code Llama (7B)' },
    { id: 'qwen2.5-coder', name: 'Qwen 2.5 Coder (7B)' },
    { id: 'qwen2.5-coder:1.5b', name: 'Qwen 2.5 Coder (1.5B)' },
    { id: 'deepseek-r1:7b', name: 'DeepSeek R1 (7B)' },
    { id: 'deepseek-r1:8b', name: 'DeepSeek R1 (8B)' },
    { id: 'deepseek-r1:1.5b', name: 'DeepSeek R1 (1.5B)' }
  ],
  deepseek: [
    { id: 'deepseek-v4-pro', name: 'DeepSeek V4 Pro' },
    { id: 'deepseek-v4-flash', name: 'DeepSeek V4 Flash' },
    { id: 'deepseek-chat', name: 'DeepSeek V3 (Chat)' },
    { id: 'deepseek-reasoner', name: 'DeepSeek R1' }
  ]
};

// --- 3. Logic & Helpers ---

// Check if the current draft differs from the saved store
const hasChanges = computed(() => {
  return (
    providerInput.value !== store.selectedAiProvider ||
    modelInput.value !== store.selectedAiModel ||
    apiKeyInput.value.length > 0 ||
    customBaseUrlInput.value !== savedCustomBaseUrl.value ||
    customModelInput.value !== savedCustomModel.value
  );
});

const providerName = computed(() => 
  providers.find(p => p.id === providerInput.value)?.name || 'AI'
);

const currentModels = computed(() => 
  modelsByProvider[providerInput.value] || []
);

// Resets local UI state to match the DB
const syncFromStore = async () => {
  await store.loadSettings();
  providerInput.value = store.selectedAiProvider;
  modelInput.value = store.selectedAiModel;
  apiKeyInput.value = ''; // Reset the input buffer

  const url = await invoke('get_setting', { key: `${providerInput.value}_custom_base_url`, default_value: '' }) as string;
  const customModel = await invoke('get_setting', { key: `${providerInput.value}_custom_model`, default_value: '' }) as string;
  
  customBaseUrlInput.value = url;
  customModelInput.value = customModel;
  savedCustomBaseUrl.value = url;
  savedCustomModel.value = customModel;

  await store.loadProviderKeyStatus(providerInput.value);

  // Load S3 settings
  s3Endpoint.value = await invoke('get_setting', { key: 's3_endpoint_url', default_value: '' }) as string;
  s3Bucket.value = await invoke('get_setting', { key: 's3_bucket_name', default_value: '' }) as string;
  s3Region.value = await invoke('get_setting', { key: 's3_region', default_value: 'us-east-1' }) as string;
  s3ForcePathStyle.value = await invoke('get_setting', { key: 's3_force_path_style', default_value: 'true' }) === 'true';
  s3SetupOk.value = await invoke('get_setting', { key: 's3_setup_ok', default_value: 'false' }) === 'true';
  s3LastUpload.value = await invoke('get_setting', { key: 's3_last_upload', default_value: 'Never' }) as string;
  s3AccessKey.value = '';
  s3SecretKey.value = '';
  
  const s3Ak = await store.getSecret('s3_access_key');
  const s3Sk = await store.getSecret('s3_secret_key');
  hasS3AccessKey.value = !!s3Ak;
  hasS3SecretKey.value = !!s3Sk;
  
  autoCloudBackup.value = (await invoke<string>('get_setting', { key: 'auto_cloud_backup', default_value: 'true' })) === 'true';
  autoLocalBackup.value = (await invoke<string>('get_setting', { key: 'auto_local_backup', default_value: 'true' })) === 'true';
};

onMounted(syncFromStore);

watch(autoCloudBackup, (val) => {
  invoke('save_setting', { key: 'auto_cloud_backup', value: val ? 'true' : 'false' });
});

watch(autoLocalBackup, (val) => {
  invoke('save_setting', { key: 'auto_local_backup', value: val ? 'true' : 'false' });
});

// When provider changes, adjust the model but DON'T wipe the Store state yet
watch(providerInput, async (newProvider) => {
  if (!newProvider) return;
  const availableModels = modelsByProvider[newProvider];

  const url = await invoke('get_setting', { key: `${newProvider}_custom_base_url`, default_value: '' }) as string;
  const customModel = await invoke('get_setting', { key: `${newProvider}_custom_model`, default_value: '' }) as string;
  
  customBaseUrlInput.value = url;
  customModelInput.value = customModel;
  savedCustomBaseUrl.value = url;
  savedCustomModel.value = customModel;

  if (customModel) {
    modelInput.value = customModel;
  } else if (!availableModels.find(m => m.id === modelInput.value)) {
    modelInput.value = availableModels[0].id;
  }
  
  // Check if THIS specific provider has a key saved in Stronghold
  await store.loadProviderKeyStatus(newProvider);
});

const sortedThemes = computed(() => {
  return [...store.availableThemes].sort((a, b) => a.name.localeCompare(b.name));
});

const handleThemeChange = async (val: string | Event) => {
  const actualVal = typeof val === 'string' ? val : (val.target as HTMLSelectElement).value;
  await store.setTheme(actualVal);
};

const handleImportTheme = async () => {
  try {
    themeError.value = '';
    await store.importCustomTheme(customThemeJson.value);
    customThemeJson.value = '';
    isImportingTheme.value = false;
    await dialog.showAlert('Custom theme imported successfully.', 'Theme Imported');
  } catch (e: any) {
    themeError.value = e.message;
  }
};

const handleDeleteTheme = async (id: string) => {
  const confirmed = await dialog.showConfirm('Are you sure you want to delete this custom theme?', 'Delete Theme');
  if (confirmed) {
    try {
      await store.deleteCustomTheme(id);
      await dialog.showAlert('Theme deleted successfully.', 'Theme Deleted');
    } catch (e: any) {
      saveError.value = e.toString();
    }
  }
};

const showThemeSchema = () => {
  const schema = `Theme JSON should follow this format:
{
  "name": "My Theme",
  "colors": {
    "--bg": "#...",
    "--bg-accent": "#...",
    ...
  }
}`;
  dialog.showAlert(schema, 'Theme Schema');
};

const handleFontFamilyChange = async (val: string | Event) => {
  const actualVal = typeof val === 'string' ? val : (val.target as HTMLSelectElement).value;
  await store.setFontFamily(actualVal);
};

const handleFontSizeChange = async (event: Event) => {
  const target = event.target as HTMLInputElement;
  await store.setFontSize(parseInt(target.value));
};

const handleFontWeightChange = async (val: string | Event) => {
  const actualVal = typeof val === 'string' ? val : (val.target as HTMLSelectElement).value;
  await store.setFontWeight(actualVal);
};

const handleFontStyleChange = async (val: string | Event) => {
  const actualVal = typeof val === 'string' ? val : (val.target as HTMLSelectElement).value;
  await store.setFontStyle(actualVal);
};

const handleResetTypography = async () => {
  const confirmed = await dialog.showConfirm('Reset all typography settings to default?', 'Reset Typography');
  if (confirmed) {
    await store.resetTypography();
  }
};

const isTestingConnection = ref(false);

const handleTestConnection = async () => {
  isTestingConnection.value = true;
  saveError.value = '';
  
  try {
    let testKey = apiKeyInput.value.trim();
    if (testKey === '') {
      const decrypted = await store.getDecryptedKey(providerInput.value);
      testKey = decrypted || '';
    }
    
    const response = await invoke<string>('test_ai_connection', {
      provider: providerInput.value,
      model: modelInput.value,
      apiKey: testKey
    });
    
    let formattedResponse = response;
    try {
      const parsed = JSON.parse(response);
      formattedResponse = JSON.stringify(parsed, null, 2);
    } catch {
      // Ignore parsing error
    }
    
    await dialog.showAlert(`Response:\n${formattedResponse}`, 'AI Connection Succeeded');
  } catch (error: any) {
    await dialog.showAlert(error.toString(), 'AI Connection Failed');
  } finally {
    isTestingConnection.value = false;
  }
};

const handleSave = async () => {
  isSaving.value = true;
  saveError.value = '';
  
  try {
    // 1. If user typed a new key, save it
    if (apiKeyInput.value.trim() !== '') {
      await store.saveApiKey(providerInput.value, apiKeyInput.value.trim());
    }
    
    // 2. Save the provider/model choice, utilizing custom model if specified
    const finalModel = customModelInput.value.trim() !== '' ? customModelInput.value.trim() : modelInput.value;
    await store.saveModelConfig(
      providerInput.value, 
      finalModel, 
      customBaseUrlInput.value.trim(), 
      customModelInput.value.trim()
    );
    
    // 3. Re-sync everything and show success
    await syncFromStore();
    showSuccess.value = true;
    setTimeout(() => { showSuccess.value = false; }, 3000);
  } catch (error: any) {
    saveError.value = error.message || 'Failed to save configuration.';
  } finally {
    isSaving.value = false;
  }
};
</script>

<template>
  <div class="settings-container">
    <div class="header">
      <h2>Engine Configuration</h2>
      <p class="subtitle">Customize how the AI intelligence layer behaves.</p>
    </div>

    <div class="settings-grid">
      <!-- UI Customization -->
      <div class="settings-card">
        <div class="card-header">
          <div class="title-row">
            <h3>Visual Persona</h3>
            <div class="header-btns">
              <button class="text-btn secondary" @click="store.setTheme('github-dark')">
                <RotateCcw :size="14" /> Reset
              </button>
              <button class="text-btn secondary" @click="copyDemoTheme">
                <Download :size="14" /> Copy Demo
              </button>
              <button class="text-btn" @click="isImportingTheme = !isImportingTheme">
                <Plus :size="14" /> {{ isImportingTheme ? 'Cancel' : 'Import Theme' }}
              </button>
            </div>
          </div>
          <p>Choose a premium built-in theme or import your own surgical palette.</p>
        </div>

        <div class="theme-selector-row">
          <div class="input-group">
            <label>Active Theme</label>
            <div class="theme-picker-wrapper" style="width: 100%;">
              <CustomSelect 
                :model-value="store.activeThemeId" 
                @change="handleThemeChange" 
                :options="sortedThemes.map(theme => ({ value: theme.id, label: theme.name + (theme.is_builtin ? ' (Built-in)' : '') }))"
              >
                <template #icon>
                  <Palette :size="16" style="color: var(--accent);" />
                </template>
              </CustomSelect>
            </div>
          </div>
          
          <button 
            v-if="!store.availableThemes.find(t => t.id === store.activeThemeId)?.is_builtin"
            class="delete-theme-btn"
            @click="handleDeleteTheme(store.activeThemeId)"
          >
            <Trash2 :size="16" />
          </button>
        </div>

        <AnimatePresence>
          <Motion
            v-if="isImportingTheme"
            :initial="{ height: 0, opacity: 0 }"
            :animate="{ height: 'auto', opacity: 1 }"
            :exit="{ height: 0, opacity: 0 }"
            class="import-theme-area"
          >
            <div class="import-header">
              <label>Theme JSON Configuration</label>
              <button class="help-link-btn" @click.prevent="showThemeSchema">View Schema</button>
            </div>
            <textarea 
              v-model="customThemeJson" 
              placeholder='{ "name": "Deep Ocean", "colors": { "--bg": "#000b1e", ... } }'
              class="theme-textarea"
            ></textarea>
            <div class="import-actions-row">
              <span v-if="themeError" class="error-inline">{{ themeError }}</span>
              <button class="btn-import-confirm" @click="handleImportTheme">Import & Apply</button>
            </div>
          </Motion>
        </AnimatePresence>
      </div>

      <!-- Typography Settings -->
      <div class="settings-card">
        <div class="card-header">
          <div class="title-row">
            <h3>Typography</h3>
            <button class="text-btn secondary" @click="handleResetTypography">
              <RotateCcw :size="14" /> Reset
            </button>
          </div>
          <p>Adjust the interface fonts to suit your surgical workflow.</p>
        </div>

        <div class="typography-row">
          <div class="input-group">
            <label>Font Family</label>
            <div class="theme-picker-wrapper" style="width: 100%;">
              <CustomSelect 
                :model-value="store.fontFamily" 
                @change="handleFontFamilyChange" 
                :options="fontFamilies.map(font => ({ value: font.id, label: font.name }))"
              >
                <template #icon>
                  <Type :size="16" style="color: var(--accent);" />
                </template>
              </CustomSelect>
            </div>
          </div>

          <div class="input-group">
            <label>Font Weight</label>
            <CustomSelect 
              :model-value="store.fontWeight" 
              @change="handleFontWeightChange" 
              :options="fontWeights.map(weight => ({ value: weight.id, label: weight.name }))"
            />
          </div>

          <div class="input-group">
            <label>Font Style</label>
            <div class="theme-picker-wrapper" style="width: 100%;">
              <CustomSelect 
                :model-value="store.fontStyle" 
                @change="handleFontStyleChange" 
                :options="fontStyles.map(style => ({ value: style.id, label: style.name }))"
              >
                <template #icon>
                  <Italic :size="16" style="color: var(--accent);" />
                </template>
              </CustomSelect>
            </div>
          </div>

          <div class="input-group size-group">
            <label>Font Size ({{ store.fontSize }}px)</label>
            <input 
              type="range" 
              min="12" 
              max="20" 
              step="1" 
              :value="store.fontSize" 
              @input="handleFontSizeChange" 
              class="font-size-slider"
            />
          </div>
        </div>
      </div>

      <!-- Intelligence Engine -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Intelligence Engine</h3>
          <p>The neural network used for parsing and tailoring.</p>
        </div>
        
        <div class="input-row">
          <div class="input-group">
            <div class="label-row" @mouseenter="activeTooltip = 'provider'" @mouseleave="activeTooltip = null">
              <label>Provider</label>
              <div class="tooltip-trigger">
                <Info :size="12" />
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'provider'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    :transition="{ duration: 0.15 }"
                    class="flying-message tooltip-top"
                  >
                    Select AI Service
                  </Motion>
                </AnimatePresence>
              </div>
            </div>
            <CustomSelect 
              v-model="providerInput" 
              :options="providers.map(p => ({ value: p.id, label: p.name }))" 
            />
          </div>

          <div class="input-group">
            <div class="label-row" @mouseenter="activeTooltip = 'model'" @mouseleave="activeTooltip = null">
              <label>Active Model</label>
              <div class="tooltip-trigger">
                <Info :size="12" />
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'model'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    :transition="{ duration: 0.15 }"
                    class="flying-message tooltip-top"
                  >
                    Choose Model Logic
                  </Motion>
                </AnimatePresence>
              </div>
            </div>
            <CustomSelect 
              v-model="modelInput" 
              :options="currentModels.map(m => ({ value: m.id, label: m.name }))" 
            />
          </div>
        </div>

        <!-- Custom Overrides -->
        <div class="input-row" style="margin-top: 16px;">
          <div class="input-group">
            <label>Custom Endpoint URL (Optional)</label>
            <input 
              v-model="customBaseUrlInput" 
              type="text" 
              :placeholder="providerInput === 'ollama' ? 'e.g. http://localhost:11434' : 'e.g. https://api.deepseek.com/v1'"
              spellcheck="false"
              class="form-input"
            />
            <span class="setup-tip" style="font-size: 0.8rem; color: var(--muted); margin-top: 4px; display: block; line-height: 1.4;">
              {{ providerInput === 'ollama' 
                ? 'Override the Ollama service URL (defaults to http://localhost:11434 if blank).' 
                : 'Override the API base URL for this provider (ideal for local or custom OpenAI-compatible endpoints).' }}
            </span>
          </div>

          <div class="input-group">
            <label>Custom Model Name (Optional)</label>
            <input 
              v-model="customModelInput" 
              type="text" 
              placeholder="e.g. deepseek-chat"
              spellcheck="false"
              class="form-input"
            />
            <span class="setup-tip" style="font-size: 0.8rem; color: var(--muted); margin-top: 4px; display: block; line-height: 1.4;">
              Type a custom model string to override the dropdown selection above.
            </span>
          </div>
        </div>
      </div>

      <!-- API Credentials -->
      <div class="settings-card">
        <div class="card-header">
          <div class="title-row">
            <h3>API Credentials</h3>
            <button 
              v-if="apiKeyInput" 
              class="text-btn" 
              @click="apiKeyInput = ''"
            >Clear Input</button>
          </div>
          <p v-if="providerInput === 'bedrock'">
            AWS Bedrock uses your AWS IAM credentials. Please enter them below to save them securely in your local vault.
          </p>
          <p v-else-if="providerInput === 'ollama'">
            Ollama runs locally and does not require an API key by default. You can leave this blank.
          </p>
          <p v-else>Your {{ providerName }} key is stored in an encrypted vault. It is never sent to our servers.</p>
        </div>
        
        <div class="credentials-content">
          <div class="input-group">
            <label>{{ providerInput === 'bedrock' ? 'AWS Bedrock Credentials' : providerName + ' Secret Key' }}</label>
            <input 
              v-model="apiKeyInput" 
              type="password" 
              :placeholder="providerInput === 'bedrock' 
                ? (store.hasSecureKey ? '•••••••••••••••• (Credentials saved)' : 'access_key_id:secret_access_key:region')
                : providerInput === 'ollama'
                  ? (store.hasSecureKey ? '•••••••••••••••• (Key saved)' : 'Optional (leave blank for local)...')
                  : (store.hasSecureKey ? '•••••••••••••••• (Key saved)' : 'Enter API Key...')"
              spellcheck="false"
              class="form-input"
            />
            <span v-if="providerInput === 'bedrock'" class="setup-tip" style="margin-top: 8px; font-size: 0.85rem; color: var(--muted); display: block; line-height: 1.4;">
              Format: <code>ACCESS_KEY_ID:SECRET_ACCESS_KEY:REGION</code>. If region is omitted, it defaults to <code>us-east-1</code>.
            </span>
          </div>

          <div class="credentials-actions">
            <div class="status-area-inline">
              <span v-if="saveError" class="error-msg">{{ saveError }}</span>
              <transition name="fade">
                <span v-if="showSuccess" class="success-msg">
                  <CheckCircle :size="16" /> Saved
                </span>
              </transition>
            </div>
            
            <div class="button-group">
              <button 
                class="btn-test-connection" 
                @click="handleTestConnection" 
                :disabled="isTestingConnection || isSaving"
              >
                <Play v-if="!isTestingConnection" :size="14" />
                <RotateCcw v-else :size="14" class="spinner" />
                Test Connection
              </button>

              <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'discard'" @mouseleave="activeTooltip = null">
                <button 
                  v-if="hasChanges" 
                  class="btn-action secondary" 
                  @click="syncFromStore" 
                  :disabled="isSaving"
                >
                  <RotateCcw :size="16" />
                </button>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'discard' && hasChanges"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    :transition="{ duration: 0.15 }"
                    class="flying-message tooltip-bottom-left"
                  >
                    Discard Changes
                  </Motion>
                </AnimatePresence>
              </div>
              
              <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'save'" @mouseleave="activeTooltip = null">
                <button 
                  class="btn-action primary" 
                  @click="handleSave" 
                  :disabled="isSaving || !hasChanges"
                >
                  <Save v-if="!isSaving" :size="16" />
                  <RotateCcw v-else :size="16" class="spinner" />
                </button>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'save'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    :transition="{ duration: 0.15 }"
                    class="flying-message tooltip-bottom-left"
                  >
                    Save Configuration
                  </Motion>
                </AnimatePresence>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Backup & Export -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Backup & Export</h3>
          <p>Export your jobs, tailored resumes, and compiler state to a secure JSON file.</p>
        </div>
        
        <div class="export-row">
          <button class="btn-export" @click="exportData" :disabled="isExporting">
            <div class="export-btn-content">
              <Database v-if="!isExporting" :size="18" />
              <RotateCcw v-else :size="18" class="spinner" />
              <div class="export-text">
                <span class="main-text">Generate Full Backup</span>
                <span class="sub-text">Includes all relational data in JSON format</span>
              </div>
            </div>
            <Download :size="18" class="download-icon" />
          </button>
        </div>
      </div>

      <!-- Vault Synchronization -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Vault Synchronization</h3>
          <p>Import data from a backup file to merge with current data or perform a full restore.</p>
        </div>
        
        <div class="import-actions">
          <button class="btn-import-option" @click="handleImport('merge')" :disabled="isImporting">
            <RefreshCw :size="18" :class="{ 'spinner': isImporting }" />
            <div class="option-text">
              <span class="option-title">Smart Sync (Merge)</span>
              <span class="option-desc">Add new data without deleting current records</span>
            </div>
          </button>

          <button class="btn-import-option danger" @click="handleImport('overwrite')" :disabled="isImporting">
            <Upload :size="18" />
            <div class="option-text">
              <span class="option-title">Full Restore (Overwrite)</span>
              <span class="option-desc">Replace all current data with the backup file</span>
            </div>
          </button>
        </div>
      </div>

      <!-- Cloud Backup Configuration -->
      <div class="settings-card">
        <div class="card-header">
          <h3>S3 Cloud Backup</h3>
          <p>Configure automated cloud backups to any S3-compatible storage.</p>
        </div>
        
        <div class="input-row">
          <div class="input-group">
            <label>Endpoint URL</label>
            <input type="password" v-model="s3Endpoint" class="form-input" placeholder="e.g. https://s3.us-east-1.amazonaws.com" />
          </div>
          <div class="input-group">
            <label>Bucket Name</label>
            <input type="password" v-model="s3Bucket" class="form-input" placeholder="my-roletect-backup-bucket" />
          </div>
        </div>

        <div class="input-row">
          <div class="input-group">
            <label>Region</label>
            <input type="password" v-model="s3Region" class="form-input" placeholder="us-east-1" />
          </div>
          <div class="input-group">
            <label>Path Style Access</label>
            <label class="checkbox-label" style="display: flex; align-items: center; gap: 8px; margin-top: 8px; cursor: pointer; color: var(--ink); font-size: 0.9rem; font-weight: normal; text-transform: none; letter-spacing: normal;">
              <input type="checkbox" v-model="s3ForcePathStyle" style="width: 18px; height: 18px; accent-color: var(--accent); cursor: pointer;" />
              <span>Enable (Required for MinIO/R2)</span>
            </label>
          </div>
        </div>
        
        <div class="input-row">
          <div class="input-group">
            <label>Access Key ID</label>
            <input type="password" v-model="s3AccessKey" class="form-input" :placeholder="hasS3AccessKey ? '•••••••••••••••• (Saved)' : 'Enter AWS/S3 Access Key ID'" />
          </div>
          <div class="input-group">
            <label>Secret Access Key</label>
            <input type="password" v-model="s3SecretKey" class="form-input" :placeholder="hasS3SecretKey ? '•••••••••••••••• (Saved)' : 'Enter AWS/S3 Secret Access Key'" />
          </div>
        </div>
        
        <div class="credentials-actions" style="margin-top: 24px;">
          <div class="button-group">
            <button class="btn-test-connection" @click="handleTestS3(false)" :disabled="isTestingS3 || isSavingS3">
              <RefreshCw v-if="isTestingS3" :size="14" class="spinner" />
              <Play v-else :size="14" />
              Test Connection
            </button>
            <button class="btn-action primary" style="width: auto; padding: 0 16px; font-weight: 700; gap: 8px;" @click="handleSaveS3" :disabled="isSavingS3">
              <Save :size="14" />
              Save S3 Settings
            </button>
          </div>
          <div class="status-area-inline">
            <span v-if="s3TestSuccess" class="success-msg"><CheckCircle :size="14"/> {{ isSavingS3 ? '' : 'Settings saved.' }}</span>
            <span v-if="s3TestError" class="error-msg">{{ s3TestError }}</span>
            <span v-if="!s3TestError && !s3TestSuccess && s3SetupOk" style="font-size: 0.75rem; color: var(--muted); display: flex; flex-direction: column; text-align: right;">
              <strong style="color: var(--accent);">Auto-Backup: Active</strong>
              <span>Last Upload: {{ s3LastUpload }}</span>
            </span>
          </div>
        </div>
      </div>

      <!-- Cloud Restore Configuration -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Restore from Cloud</h3>
          <p>Fetch and restore your data from your S3 backup vault. <strong>Warning:</strong> Restoring will modify local data.</p>
        </div>
        
        <div class="credentials-actions" style="margin-top: 16px; border-top: none; padding-top: 0;">
          <div class="button-group">
            <button class="btn-test-connection" @click="handleFetchBackups" :disabled="isFetchingBackups || !s3SetupOk">
              <DownloadCloud v-if="!isFetchingBackups" :size="14" />
              <RefreshCw v-else :size="14" class="spinner" />
              Fetch Available Backups
            </button>
          </div>
          <div class="status-message">
            <span v-if="fetchBackupsError" class="error-msg">{{ fetchBackupsError }}</span>
            <span v-if="!s3SetupOk" class="warning-msg" style="color: var(--warning); font-size: 0.85rem;">S3 not active.</span>
          </div>
        </div>

        <div v-if="availableBackups.length > 0" class="input-row" style="margin-top: 16px;">
          <div class="input-group">
            <label>Select Backup to Restore</label>
            <CustomSelect
              v-model="selectedBackup"
              :options="availableBackupsOptions"
              class="custom-select"
              placement="top"
            />
          </div>
          <div class="input-group">
            <label>Restore Mode</label>
            <CustomSelect
              v-model="restoreMode"
              :options="restoreModeOptions"
              class="custom-select"
              placement="top"
            />
          </div>
        </div>
        
        <div v-if="availableBackups.length > 0" class="credentials-actions" style="margin-top: 24px;">
          <button class="btn-action primary" style="width: 100%; justify-content: center; background: var(--warning); color: white; border: none; font-weight: 700; gap: 8px;" @click="handleRestoreBackup" :disabled="isRestoringBackup || !selectedBackup">
            <RotateCcw v-if="!isRestoringBackup" :size="16" />
            <RefreshCw v-else :size="16" class="spinner" />
            {{ isRestoringBackup ? 'Restoring & Reloading...' : 'Restore Selected Backup' }}
          </button>
        </div>
      </div>

      <!-- Backup Automation Configuration -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Backup Automation</h3>
          <p>Control what happens automatically when you close RoleTect.</p>
        </div>
        
        <div class="input-row" style="margin-top: 16px;">
          <div class="input-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="autoLocalBackup" class="custom-checkbox" />
              Auto Local Backup on Exit
            </label>
            <p style="font-size: 0.75rem; color: var(--muted); margin-top: 4px; margin-left: 24px;">Automatically exports an unencrypted snapshot to your <strong>Documents/RoleTect-Backups</strong> folder.</p>
          </div>
        </div>

        <div class="input-row">
          <div class="input-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="autoCloudBackup" class="custom-checkbox" :disabled="!s3SetupOk" />
              Auto Cloud Backup on Exit
            </label>
            <p style="font-size: 0.75rem; color: var(--muted); margin-top: 4px; margin-left: 24px;">Automatically uploads an encrypted snapshot to your configured S3 bucket. Requires active S3 setup.</p>
          </div>
        </div>
      </div>

      <!-- Maintenance -->
      <div class="settings-card">
        <div class="card-header">
          <h3>Maintenance</h3>
          <p>Advanced tools to repair the engine or resolve environmental issues.</p>
        </div>
        
        <div class="maintenance-row">
          <button class="btn-maintenance" @click="handleClearCache" :disabled="isClearingCache">
            <div class="maintenance-btn-content">
              <RotateCcw v-if="!isClearingCache" :size="18" />
              <RefreshCw v-else :size="18" class="spinner" />
              <div class="maintenance-text">
                <span class="main-text">Purge LaTeX Cache</span>
                <span class="sub-text">Resolves "fatal format file error" by forcing a fresh engine rebuild</span>
              </div>
            </div>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  padding: 40px;
  max-width: 900px;
  margin: 0 auto;
}

.header { margin-bottom: 32px; }
.header h2 { font-size: 2rem; margin: 0; color: var(--ink); }
.subtitle { color: var(--muted); margin: 8px 0 0; }

.settings-grid { display: flex; flex-direction: column; gap: 24px; padding-bottom: 100px; }

.settings-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 24px;
  box-shadow: var(--shadow);
}

.credentials-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
  margin-top: 20px;
}

.credentials-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 16px;
  border-top: 1px solid var(--line);
}

.status-area-inline {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title-row { display: flex; justify-content: space-between; align-items: center; }

.header-btns {
  display: flex;
  gap: 12px;
}

.text-btn {
  background: none; border: none;
  color: var(--accent); font-weight: 700; font-size: 0.75rem;
  text-transform: uppercase; cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}

.text-btn.secondary {
  color: var(--muted);
}

.text-btn.secondary:hover {
  color: var(--ink);
}

.text-btn:hover {
  opacity: 0.8;
}

.typography-row {
  display: flex;
  gap: 24px;
  margin-top: 20px;
  align-items: flex-end;
}

.size-group {
  flex: 1.5;
}

.font-size-slider {
  width: 100%;
  margin-top: 12px;
  cursor: pointer;
  accent-color: var(--accent);
}

.input-row { display: flex; gap: 20px; margin-top: 20px; }
.input-group { flex: 1; display: flex; flex-direction: column; gap: 8px; }

.label-row {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: help;
}

.tooltip-trigger {
  color: var(--muted);
  display: flex;
  align-items: center;
  position: relative;
}

.flying-message {
  position: absolute;
  background: var(--surface-soft);
  color: var(--ink);
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.65rem;
  font-weight: 700;
  white-space: nowrap;
  pointer-events: none;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  border: 1px solid var(--line);
}

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
}

label {
  color: var(--accent); font-weight: 700; font-size: 0.7rem;
  text-transform: uppercase; letter-spacing: 0.1em;
}

.form-input, .custom-select {
  width: 100%;
  padding: 12px 16px;
  font-size: 1rem;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  color: var(--ink);
  border-radius: 8px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.form-input:focus, .custom-select:focus {
  border-color: var(--accent);
}

.custom-select {
  cursor: pointer;
}

.theme-selector-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  margin-top: 20px;
}

.theme-picker-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.picker-icon {
  position: absolute;
  left: 12px;
  color: var(--accent);
  pointer-events: none;
}

.custom-select.with-icon {
  padding-left: 36px;
}

.delete-theme-btn {
  height: 42px;
  width: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.2);
  color: var(--warning);
  border-radius: 8px;
  cursor: pointer;
  transition: 0.2s;
}

.delete-theme-btn:hover {
  background: var(--warning);
  color: white;
  border-color: var(--warning);
}

.import-theme-area {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
}

.import-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.help-link-btn {
  background: none;
  border: none;
  color: var(--accent);
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  cursor: pointer;
  padding: 0;
}

.theme-textarea {
  width: 100%;
  height: 120px;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 12px;
  color: var(--ink);
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.8rem;
  resize: vertical;
}

.import-actions-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.btn-import-confirm {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 6px;
  padding: 8px 16px;
  font-size: 0.8rem;
  font-weight: 700;
  cursor: pointer;
}

.error-inline {
  color: var(--warning);
  font-size: 0.75rem;
  font-weight: 600;
}

.export-row {
  margin-top: 24px;
}

.btn-export {
  width: 100%;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  color: var(--ink);
}

.btn-export:hover:not(:disabled) {
  background: var(--surface);
  border-color: var(--accent);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
}

.btn-export:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.export-btn-content {
  display: flex;
  align-items: center;
  gap: 16px;
  text-align: left;
}

.export-text {
  display: flex;
  flex-direction: column;
}

.main-text {
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--ink);
}

.sub-text {
  font-size: 0.7rem;
  color: var(--muted);
}

.download-icon {
  color: var(--accent);
  opacity: 0.8;
}

.btn-export:hover .download-icon {
  opacity: 1;
  transform: translateY(2px);
}

.import-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-top: 24px;
}

.btn-import-option {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
  color: var(--ink);
}

.btn-import-option:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--surface);
  transform: translateY(-2px);
}

.btn-import-option.danger:hover:not(:disabled) {
  border-color: var(--warning);
  background: rgba(248, 81, 73, 0.05);
}

.option-text {
  display: flex;
  flex-direction: column;
}

.option-title {
  font-size: 0.85rem;
  font-weight: 700;
}

.option-desc {
  font-size: 0.65rem;
  color: var(--muted);
}

.maintenance-row {
  margin-top: 24px;
}

.btn-maintenance {
  width: 100%;
  background: rgba(248, 81, 73, 0.05);
  border: 1px solid rgba(248, 81, 73, 0.1);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  transition: all 0.2s ease;
  color: var(--warning);
}

.btn-maintenance:hover:not(:disabled) {
  background: rgba(248, 81, 73, 0.1);
  border-color: var(--warning);
  transform: translateY(-2px);
}

.maintenance-btn-content {
  display: flex;
  align-items: center;
  gap: 16px;
  text-align: left;
}

.maintenance-text {
  display: flex;
  flex-direction: column;
}

.btn-import-option .spinner {
  color: var(--accent);
}

.button-group { display: flex; gap: 12px; }

.btn-test-connection {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-accent);
  border: 1px solid var(--line);
  color: var(--accent);
  font-weight: 700;
  font-size: 0.75rem;
  text-transform: uppercase;
  border-radius: 10px;
  padding: 0 16px;
  height: 42px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-test-connection:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--surface-soft);
}

.btn-test-connection:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-action {
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  cursor: pointer;
  transition: 0.2s;
  border: 1px solid var(--line);
}

.btn-action.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn-action.primary:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-action.secondary { background: none; color: var(--muted); }
.btn-action.secondary:hover { border-color: var(--ink); color: var(--ink); }

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.success-msg { color: var(--accent); font-weight: 600; display: flex; align-items: center; gap: 8px; font-size: 0.8rem; }
.error-msg { color: var(--warning); font-weight: 600; font-size: 0.8rem; }

@media (max-width: 600px) {
  .input-row { flex-direction: column; }
  .typography-row { flex-direction: column; align-items: stretch; }
  .credentials-actions { flex-direction: column; gap: 20px; align-items: flex-start; }
}
</style>
