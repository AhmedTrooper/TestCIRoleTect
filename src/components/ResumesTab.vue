<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useResumesStore } from '../store/resumes';
import { useDialogStore } from '../store/dialog';
import { 
  Plus, 
  Tag, 
  Calendar, 
  Hash, 
  FileText, 
  X, 
  Save, 
  RotateCw, 
  CheckSquare, 
  Square, 
  Trash2,
  Settings2,
  Check
} from '@lucide/vue';
import { Motion, AnimatePresence } from 'motion-v';

const router = useRouter();
const resumesStore = useResumesStore();
const dialog = useDialogStore();

// Tooltip State
const activeTooltip = ref<string | null>(null);

// Selection State
const isSelectionMode = ref(false);
const selectedIds = ref<Set<string>>(new Set());

const showNewResumeForm = ref(false);
const newResumeName = ref('');
const newResumeCategory = ref('');
const isCreating = ref(false);

onMounted(async () => {
  await resumesStore.loadAllResumes();
});

const navigateToResume = (resumeId: string) => {
  if (isSelectionMode.value) {
    toggleSelection(resumeId);
    return;
  }
  router.push(`/resume/${resumeId}`);
};

const toggleSelection = (id: string) => {
  if (selectedIds.value.has(id)) {
    selectedIds.value.delete(id);
  } else {
    selectedIds.value.add(id);
  }
};

const toggleSelectAll = () => {
  if (selectedIds.value.size === resumesStore.resumes.length) {
    selectedIds.value.clear();
  } else {
    selectedIds.value = new Set(resumesStore.resumes.map(r => r.id));
  }
};

const exitSelectionMode = () => {
  isSelectionMode.value = false;
  selectedIds.value.clear();
};

const handleBatchDelete = async () => {
  if (selectedIds.value.size === 0) return;
  
  const confirmed = await dialog.showConfirm(
    `Are you sure you want to delete ${selectedIds.value.size} templates? This action cannot be undone.`,
    'Delete Templates'
  );
  
  if (confirmed) {
    try {
      const ids = Array.from(selectedIds.value);
      for (const id of ids) {
        await resumesStore.deleteResume(id);
      }
      selectedIds.value.clear();
      await resumesStore.loadAllResumes();
      await dialog.showAlert('Templates deleted successfully.', 'Success');
    } catch (err: any) {
      console.error(err);
      await dialog.showAlert('Failed to delete some templates.', 'Error');
    }
  }
};

const toggleNewForm = () => {
  showNewResumeForm.value = !showNewResumeForm.value;
  if (!showNewResumeForm.value) {
    newResumeName.value = '';
    newResumeCategory.value = '';
  }
};

const handleCreateResume = async () => {
  if (!newResumeName.value.trim() || !newResumeCategory.value.trim()) {
    return;
  }
  
  isCreating.value = true;
  try {
    const resumeId = await resumesStore.createNewResume(
      newResumeName.value,
      newResumeCategory.value,
      ''
    );
    showNewResumeForm.value = false;
    newResumeName.value = '';
    newResumeCategory.value = '';
    router.push(`/resume/${resumeId}`);
  } catch (err: any) {
    console.error(err);
  } finally {
    isCreating.value = false;
  }
};
</script>

<template>
  <div class="resumes-container">
    <header class="page-header">
      <div class="title-group">
        <h1>Resume Templates</h1>
        <p class="subtitle" v-if="!isSelectionMode">Your blueprint collection for high-performance CVs.</p>
        <p class="subtitle selection-hint" v-else>Click items to select/deselect them.</p>
      </div>
      
      <div class="header-actions">
        <!-- Default Actions -->
        <template v-if="!isSelectionMode">
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'selection-mode'" @mouseleave="activeTooltip = null">
            <button class="btn-icon" :class="{ 'active': isSelectionMode }" @click="isSelectionMode = true">
              <Settings2 :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'selection-mode'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Selection Mode
              </Motion>
            </AnimatePresence>
          </div>
          
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'new-template'" @mouseleave="activeTooltip = null">
            <button class="btn-icon btn-icon-primary" @click="toggleNewForm">
              <Plus :size="18" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'new-template'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Add Base Template
              </Motion>
            </AnimatePresence>
          </div>
        </template>

        <!-- Selection Mode Actions -->
        <template v-else>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'select-all'" @mouseleave="activeTooltip = null">
            <button class="btn-icon" @click="toggleSelectAll">
              <Check :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'select-all'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                {{ selectedIds.size === resumesStore.resumes.length ? 'Unselect All' : 'Select All' }}
              </Motion>
            </AnimatePresence>
          </div>

          <div class="btn-tooltip-wrapper" v-if="selectedIds.size > 0" @mouseenter="activeTooltip = 'delete-batch'" @mouseleave="activeTooltip = null">
            <button class="btn-icon btn-icon-danger" @click="handleBatchDelete">
              <Trash2 :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'delete-batch'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Delete Selected ({{ selectedIds.size }})
              </Motion>
            </AnimatePresence>
          </div>

          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'exit-selection'" @mouseleave="activeTooltip = null">
            <button class="btn-icon btn-icon-primary" @click="exitSelectionMode">
              <X :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'exit-selection'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                class="floating-message tooltip-bottom-left"
              >
                Done
              </Motion>
            </AnimatePresence>
          </div>
        </template>
      </div>
    </header>

    <div v-if="resumesStore.error" class="error-banner">
      {{ resumesStore.error }}
    </div>

    <transition name="slide-down">
      <div v-if="showNewResumeForm" class="form-card">
        <div class="form-header">
          <h3>Create New Template</h3>
          <button class="close-btn" @click="toggleNewForm"><X :size="18" /></button>
        </div>
        
        <div class="form-grid">
          <div class="form-group">
            <label>Template Name</label>
            <input 
              v-model="newResumeName" 
              type="text" 
              placeholder="e.g., Senior Full-Stack Base"
              class="form-input"
            />
          </div>
          <div class="form-group">
            <label>Category</label>
            <input 
              v-model="newResumeCategory" 
              type="text" 
              placeholder="e.g., Engineering"
              class="form-input"
            />
          </div>
        </div>

        <div class="form-actions">
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'initialize-template'" @mouseleave="activeTooltip = null">
            <button class="btn-save" @click="handleCreateResume" :disabled="isCreating || !newResumeName || !newResumeCategory">
              <RotateCw v-if="isCreating" :size="16" class="spinner" />
              <Save v-else :size="16" />
              <span>{{ isCreating ? 'Initializing...' : 'Initialize Template' }}</span>
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'initialize-template'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-top-left"
              >
                Create and Start Editing
              </Motion>
            </AnimatePresence>
          </div>
        </div>
      </div>
    </transition>

    <div v-if="resumesStore.isLoading" class="loading-state">
      Accessing blueprint vault...
    </div>

    <div v-else-if="resumesStore.resumes.length === 0" class="empty-state">
      <FileText :size="48" class="empty-icon" />
      <h3>No templates found</h3>
      <p>Create your first base resume to start tailoring.</p>
      <div class="btn-tooltip-wrapper" style="margin: 0 auto;" @mouseenter="activeTooltip = 'init-first'" @mouseleave="activeTooltip = null">
        <button class="btn-primary" @click="toggleNewForm">
          <Plus :size="18" />
          <span>Initialize First Template</span>
        </button>
        <AnimatePresence>
          <Motion
            v-if="activeTooltip === 'init-first'"
            :initial="{ opacity: 0, y: 5, scale: 0.9 }"
            :animate="{ opacity: 1, y: 0, scale: 1 }"
            :exit="{ opacity: 0, y: 5, scale: 0.9 }"
            :transition="{ duration: 0.15 }"
            class="floating-message tooltip-top"
          >
            Start Your Collection
          </Motion>
        </AnimatePresence>
      </div>
    </div>

    <div v-else class="resumes-grid">
      <div 
        v-for="resume in resumesStore.resumes" 
        :key="resume.id"
        class="resume-card"
        :class="{ 'selected': selectedIds.has(resume.id) }"
        @click="navigateToResume(resume.id)"
      >
        <div class="resume-card-top">
          <div class="selection-overlay" @click.stop="toggleSelection(resume.id)">
            <CheckSquare v-if="selectedIds.has(resume.id)" :size="20" class="select-icon active" />
            <Square v-else :size="20" class="select-icon" />
          </div>
          
          <div class="category-badge">
            <Tag :size="12" /> {{ resume.category }}
          </div>
        </div>

        <h3 class="resume-name">{{ resume.name }}</h3>
        
        <div class="resume-meta">
          <div class="meta-item">
            <Calendar :size="14" />
            <span>{{ new Date(resume.created_at).toLocaleDateString() }}</span>
          </div>
          <div class="meta-item id-meta">
            <Hash :size="14" />
            <span>{{ resume.id }}</span>
          </div>
        </div>

        <div class="card-footer">
          <span class="edit-link">Edit Template &rarr;</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.resumes-container {
  padding: 40px;
  max-width: 1200px;
  margin: 0 auto;
  overflow-y: auto;
  height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 32px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.btn-icon {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  cursor: pointer;
  transition: 0.2s;
  background: var(--surface-soft);
  color: var(--ink);
  border: 1px solid var(--line);
  padding: 0;
}

.btn-icon:hover { background: var(--surface); border-color: var(--accent); }
.btn-icon.active { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }

.btn-icon-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.btn-icon-danger {
  background: transparent;
  color: var(--warning);
  border: 1px solid var(--warning);
}
.btn-icon-danger:hover { background: var(--warning); color: white; }

.btn-primary {
  background: var(--accent);
  color: white;
  border: none;
  padding: 12px 24px;
  border-radius: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: 0.2s;
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-primary:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(11, 123, 107, 0.2); }

.btn-secondary {
  background: var(--surface-soft);
  color: var(--ink);
  border: 1px solid var(--line);
  padding: 10px 20px;
  border-radius: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: 0.2s;
}

.btn-secondary:hover { background: var(--surface); border-color: var(--accent); }

.btn-danger-outline {
  background: transparent;
  color: var(--warning);
  border: 1px solid var(--warning);
  padding: 10px 20px;
  border-radius: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: 0.2s;
}

.btn-danger-outline:hover { background: var(--warning); color: white; }

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
}

.error-banner {
  background: rgba(248, 51, 73, 0.1);
  border: 1px solid rgba(248, 51, 73, 0.2);
  border-radius: 12px;
  padding: 12px 16px;
  margin-bottom: 24px;
  color: #f85149;
  font-size: 0.9rem;
}

.form-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 20px;
  padding: 32px;
  margin-bottom: 40px;
  box-shadow: var(--shadow);
  max-width: 100%;
}

.form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.form-header h3 { margin: 0; font-size: 1.25rem; color: var(--ink); }

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  transition: 0.2s;
}

.close-btn:hover { color: var(--ink); }

.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 24px;
  margin-bottom: 32px;
}

.form-group { display: flex; flex-direction: column; gap: 8px; }

.form-group label {
  font-size: 0.7rem;
  font-weight: 800;
  text-transform: uppercase;
  color: var(--accent);
  letter-spacing: 0.05em;
}

.form-input {
  width: 100%;
  padding: 12px 16px;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 10px;
  color: var(--ink);
  font-size: 1rem;
  outline: none;
  transition: 0.2s;
}

.form-input:focus {
  border-color: var(--accent);
}

.form-actions { display: flex; justify-content: flex-end; }
.btn-save {
  background: var(--accent);
  color: white;
  border: none;
  padding: 12px 32px;
  border-radius: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-width: 180px;
  white-space: nowrap;
}

.btn-save:disabled { opacity: 0.5; cursor: not-allowed; }

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.resumes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 24px;
  padding-bottom: 40px;
}

.resume-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 24px;
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow);
  overflow: hidden;
  min-width: 0;
  position: relative;
}

.resume-card.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.resume-card:hover {
  transform: translateY(-4px);
  border-color: var(--accent);
  box-shadow: 0 8px 24px rgba(0,0,0,0.06);
}

.resume-card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.selection-overlay {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  transition: 0.2s;
}

.select-icon.active {
  color: var(--accent);
}

.category-badge {
  padding: 4px 10px;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 999px;
  font-size: 0.7rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 6px;
}

.resume-name {
  font-size: 1.25rem;
  margin: 0 0 16px 0;
  color: var(--ink);
  font-weight: 800;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.resume-meta {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: auto;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--muted);
  font-size: 0.8rem;
}

.id-meta { font-family: monospace; opacity: 0.7; }

.card-footer {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
}

.edit-link {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--muted);
  transition: 0.2s;
}

.resume-card:hover .edit-link { color: var(--accent); }

.loading-state, .empty-state {
  text-align: center;
  padding: 80px 0;
  color: var(--muted);
}

.empty-icon { margin-bottom: 16px; opacity: 0.3; color: var(--muted); }
.empty-state h3 { color: var(--ink); margin-bottom: 8px; }
.empty-state p { margin-bottom: 24px; }
.empty-state .btn-primary { margin: 0 auto; }

/* Transitions */
.slide-down-enter-active, .slide-down-leave-active { transition: all 0.3s ease-out; }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; transform: translateY(-20px); }

@media (max-width: 768px) {
  .resumes-container { padding: 16px; }
  .page-header { flex-direction: column; gap: 16px; align-items: stretch; margin-bottom: 20px; }
  .header-actions { justify-content: space-between; }
  .form-grid { grid-template-columns: 1fr; }
}
</style>
