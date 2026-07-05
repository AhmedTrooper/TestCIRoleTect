<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useResumesStore, type ResumeDetail } from '../store/resumes';
import { Motion, AnimatePresence } from 'motion-v';
import { ask, message } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { 
  ArrowLeft, 
  Edit, 
  Trash2, 
  X, 
  Save, 
  RotateCw 
} from '@lucide/vue';

// Codemirror imports
import { Codemirror } from 'vue-codemirror';
import { latex, latexLanguage, autoCloseTags } from 'codemirror-lang-latex';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';

const router = useRouter();
const resumesStore = useResumesStore();

// Codemirror Extensions
const extensions = [
  latex(),
  latexLanguage,
  ...autoCloseTags,
  oneDark,
  EditorView.lineWrapping
];

const props = defineProps<{ id: string }>();

interface UsageRecord {
  job_id: string;
  company_name: string;
  job_title: string;
}

// Tooltip State
const activeTooltip = ref<string | null>(null);

const isLoading = ref(true);
const isEditing = ref(false);
const isSaving = ref(false);
const isDeleting = ref(false);
const error = ref<string | null>(null);

const resume = ref<ResumeDetail | null>(null);
const editedName = ref('');
const editedCategory = ref('');
const editedLatex = ref('');

onMounted(async () => {
  try {
    resume.value = await resumesStore.getResumeById(props.id);
    editedName.value = resume.value.name;
    editedCategory.value = resume.value.category;
    editedLatex.value = resume.value.latex_content;
  } catch (err: any) {
    error.value = err.toString();
  } finally {
    isLoading.value = false;
  }
});

const goBack = () => router.push('/resumes');

const toggleEditMode = () => {
  if (isEditing.value) {
    // Reset to current values if cancelling
    editedName.value = resume.value?.name || '';
    editedCategory.value = resume.value?.category || '';
    editedLatex.value = resume.value?.latex_content || '';
  }
  isEditing.value = !isEditing.value;
};

const handleSave = async () => {
  if (!resume.value || !editedName.value.trim() || !editedCategory.value.trim()) {
    error.value = 'Name and category are required';
    return;
  }

  isSaving.value = true;
  error.value = null;

  try {
    await resumesStore.updateResume(
      resume.value.id,
      editedName.value,
      editedCategory.value,
      editedLatex.value
    );

    // Reload the resume
    const updated = await resumesStore.getResumeById(props.id);
    resume.value = updated;
    isEditing.value = false;
  } catch (err: any) {
    error.value = err.toString();
  } finally {
    isSaving.value = false;
  }
};

const handleDelete = async () => {
  if (!resume.value) return;

  try {
    // 1. Check for usage in tailored resumes
    const usages = await invoke<UsageRecord[]>('check_resume_usage', { resumeId: resume.value.id });

    if (usages.length > 0) {
      const jobList = usages.map(u => `• ${u.company_name} (${u.job_title})`).join('\n');
      await message(
        `This template cannot be deleted because it is currently used by tailored resumes for the following jobs:\n\n${jobList}\n\nPlease delete these tailored versions or the jobs themselves before deleting this base template.`,
        { title: 'Template In Use', kind: 'error' }
      );
      return;
    }

    // 2. Proceed with normal confirmation
    const confirmed = await ask('Delete this resume template? This cannot be undone.', {
      title: 'Confirm Deletion',
      kind: 'warning'
    });
    if (!confirmed) return;

    isDeleting.value = true;
    error.value = null;

    await resumesStore.deleteResume(resume.value.id);
    router.push('/resumes');
  } catch (err: any) {
    error.value = err.toString();
  } finally {
    isDeleting.value = false;
  }
};

const hasLatexContent = () => {
  const content = resume.value?.latex_content || '';
  return content.trim().length > 0;
};
</script>

<template>
  <div class="detail-container" v-if="!isLoading">
    <header class="detail-header">
      <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'back'" @mouseleave="activeTooltip = null">
        <button class="back-btn" @click="goBack"><ArrowLeft :size="16" /></button>
        <AnimatePresence>
          <Motion
            v-if="activeTooltip === 'back'"
            :initial="{ opacity: 0, y: 5, scale: 0.9 }"
            :animate="{ opacity: 1, y: 0, scale: 1 }"
            :exit="{ opacity: 0, y: 5, scale: 0.9 }"
            :transition="{ duration: 0.15 }"
            class="flying-message header-tooltip"
          >
            Back to Templates
          </Motion>
        </AnimatePresence>
      </div>
      
      <div class="header-main">
        <div class="title-group" v-if="!isEditing">
          <h1>{{ resume?.name }}</h1>
          <span class="category-tag">{{ resume?.category }}</span>
        </div>
        <div class="edit-group" v-else>
          <input v-model="editedName" class="edit-input name-input" placeholder="Template Name" />
          <input v-model="editedCategory" class="edit-input category-input" placeholder="Category" />
        </div>
      </div>

      <div class="header-actions">
        <template v-if="!isEditing">
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'edit-tpl'" @mouseleave="activeTooltip = null">
            <button class="action-btn" @click="toggleEditMode"><Edit :size="16" /></button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'edit-tpl'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="flying-message header-tooltip"
              >
                Edit Template
              </Motion>
            </AnimatePresence>
          </div>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'delete-tpl'" @mouseleave="activeTooltip = null">
            <button class="action-btn delete-btn" @click="handleDelete" :disabled="isDeleting">
              <RotateCw v-if="isDeleting" :size="16" class="spinner" />
              <Trash2 v-else :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'delete-tpl'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="flying-message header-tooltip delete-tooltip"
              >
                Delete Template
              </Motion>
            </AnimatePresence>
          </div>
        </template>
        <template v-else>
          <button class="action-btn cancel-btn" @click="toggleEditMode"><X :size="16" /></button>
          <button class="action-btn save-btn" @click="handleSave" :disabled="isSaving">
            <RotateCw v-if="isSaving" :size="16" class="spinner" />
            <Save v-else :size="16" />
          </button>
        </template>
      </div>
    </header>

    <div class="content-wrapper">
      <div v-if="error" class="error-banner">{{ error }}</div>

      <div class="latex-section">
        <div class="section-header">
          <h2>LATEX SOURCE</h2>
          <div class="editor-actions">
            <span class="status-indicator" v-if="isEditing">Editing Mode</span>
          </div>
        </div>
        
        <codemirror
          v-if="isEditing"
          v-model="editedLatex"
          placeholder="Enter your LaTeX code here..."
          :style="{ minHeight: '400px', height: 'auto' }"
          :autofocus="true"
          :indent-with-tab="true"
          :tab-size="2"
          :extensions="extensions"
          class="latex-editor-cm"
        />
        <div v-else-if="hasLatexContent()" class="latex-preview">
          <pre><code>{{ resume?.latex_content }}</code></pre>
        </div>
        <div v-else class="empty-latex">
          <p>This template has no LaTeX content yet.</p>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'add-latex'" @mouseleave="activeTooltip = null">
            <button class="btn-edit" @click="toggleEditMode"><Edit :size="16" /></button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'add-latex'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="flying-message"
              >
                Add LaTeX
              </Motion>
            </AnimatePresence>
          </div>
        </div>
      </div>

      <div class="meta-info" v-if="!isEditing">
        <div class="meta-item">
          <label>CREATED</label>
          <span>{{ new Date(resume?.created_at || '').toLocaleString() }}</span>
        </div>
        <div class="meta-item">
          <label>LAST UPDATED</label>
          <span>{{ new Date(resume?.updated_at || '').toLocaleString() }}</span>
        </div>
      </div>
    </div>
  </div>
  <div class="loading" v-else>
    Loading resume details...
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
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 24px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
  gap: 20px;
}

.back-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  display: flex;
  transition: 0.2s;
}
.back-btn:hover { background: var(--surface); color: var(--ink); }

.header-main { flex: 1; min-width: 0; }
.title-group { display: flex; align-items: center; gap: 12px; }
.title-group h1 { font-size: 1.1rem; font-weight: 700; color: var(--ink); margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.category-tag {
  background: var(--surface);
  color: var(--muted);
  font-size: 0.6rem;
  font-weight: 800;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid var(--line);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.edit-group { display: flex; gap: 12px; }
.edit-input {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 6px;
  color: var(--ink);
  padding: 6px 12px;
  font-size: 0.9rem;
  outline: none;
}
.name-input { font-weight: 700; flex: 1; }
.category-input { width: fit-content; }

.edit-input:focus {
  border-color: var(--accent);
}

.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.action-btn {
  background: var(--surface);
  border: 1px solid var(--line);
  color: var(--muted);
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  cursor: pointer;
  transition: 0.2s;
}
.action-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.delete-btn:hover:not(:disabled) { border-color: var(--warning); color: var(--warning); }
.save-btn { background: var(--accent); color: white; border: none; }
.save-btn:hover:not(:disabled) { background: var(--accent-hover); color: white; }

.content-wrapper {
  flex: 1;
  overflow-y: auto;
  padding: 32px 24px;
  max-width: 1000px;
  width: 100%;
  margin: 0 auto;
}

.error-banner {
  background: rgba(248, 81, 73, 0.1);
  color: var(--warning);
  padding: 12px 16px;
  border-radius: 8px;
  font-size: 0.85rem;
  margin-bottom: 24px;
}

.latex-section {
  margin-bottom: 32px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.section-header h2 {
  font-size: 0.7rem;
  font-weight: 900;
  color: var(--muted);
  letter-spacing: 0.1em;
  margin: 0;
}

.status-indicator {
  font-size: 0.65rem;
  color: var(--accent);
  font-weight: 700;
}

.latex-editor-cm {
  width: 100%;
  background-color: #282c34; /* One Dark background */
  border: 1px solid var(--line);
  border-radius: 12px;
  overflow: hidden;
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 0.9rem;
}

:deep(.cm-editor) {
  outline: none !important;
}

:deep(.cm-scroller) {
  font-family: inherit;
}

:deep(.cm-content) {
  padding: 16px 0;
}

:deep(.cm-gutters) {
  background-color: #282c34 !important;
  border-right: 1px solid #3e4451 !important;
  color: #abb2bf !important;
}

.latex-preview {
  width: 100%;
  min-height: 340px;
  max-height: 520px;
  background-color: var(--surface);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 16px;
  color: var(--ink);
  font-family: 'Monaco', 'Menlo', monospace;
  font-size: 0.8rem;
  line-height: 1.6;
  overflow: auto;
}

.latex-preview pre { margin: 0; white-space: pre-wrap; }

.empty-latex {
  width: 100%;
  min-height: 220px;
  background-color: var(--surface);
  border: 1px dashed var(--line);
  border-radius: 12px;
  padding: 20px;
  color: var(--muted);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}
.empty-latex p { margin: 0; font-size: 0.9rem; }

.btn-edit {
  background: var(--accent);
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 8px;
  cursor: pointer;
}

.meta-info {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--line);
}

.meta-item label {
  display: block;
  font-size: 0.6rem;
  font-weight: 800;
  color: var(--muted);
  margin-bottom: 4px;
}

.meta-item span {
  font-size: 0.85rem;
  color: var(--ink);
}

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.flying-message {
  position: absolute;
  bottom: 140%;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: white;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.65rem;
  font-weight: 700;
  white-space: nowrap;
  pointer-events: none;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

.flying-message::after {
  content: '';
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 4px solid transparent;
  border-top-color: var(--accent);
}

.header-tooltip { bottom: auto; top: 140%; }
.header-tooltip::after { top: auto; bottom: 100%; border-top-color: transparent; border-bottom-color: var(--accent); }
.delete-tooltip { background: var(--warning); left: auto; right: 0; transform: none; }
.delete-tooltip::after { border-bottom-color: var(--warning); left: auto; right: 8px; transform: none; }

.loading {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
