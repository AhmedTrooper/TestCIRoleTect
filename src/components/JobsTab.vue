<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useJobsStore, Job } from '../store/jobs';
import { Motion, AnimatePresence } from 'motion-v';
import { ask } from '@tauri-apps/plugin-dialog';
import CustomSelect from './CustomSelect.vue';

import { 
  Search, 
  Settings2, 
  Trash2, 
  Plus, 
  Check, 
  X, 
  FolderOpen, 
  ChevronRight,
  Filter,
  ArrowUpDown
} from '@lucide/vue';

const router = useRouter();
const jobsStore = useJobsStore();

const allJobs = ref<Job[]>([]);
const searchQuery = ref('');
const statusFilter = ref('All');
const sortBy = ref('date-desc');

// Tooltip State
const activeTooltip = ref<string | null>(null);

// Selection Mode State
const isSelectionMode = ref(false);
const selectedJobs = ref<Set<string>>(new Set());

const statuses = ['All', 'Drafting', 'Applied', 'Interviewing', 'Offer', 'Rejected'];

// Grid Layout Logic for Virtualization
const columns = ref(3);
const updateColumns = () => {
  const width = window.innerWidth;
  if (width < 768) columns.value = 1;
  else if (width < 1100) columns.value = 2;
  else columns.value = 3;
};

const loadJobs = async () => {
  allJobs.value = await jobsStore.loadAllJobs();
  selectedJobs.value.clear();
};

onMounted(() => {
  loadJobs();
  window.addEventListener('resize', updateColumns);
  updateColumns();
});

onUnmounted(() => {
  window.removeEventListener('resize', updateColumns);
});

const handleCardClick = (id: string) => {
  if (isSelectionMode.value) {
    if (selectedJobs.value.has(id)) {
      selectedJobs.value.delete(id);
    } else {
      selectedJobs.value.add(id);
    }
  } else {
    router.push(`/job/${id}`);
  }
};

const deleteSelectedJobs = async () => {
  if (selectedJobs.value.size === 0) return;
  
  const confirmed = await ask(`Are you sure you want to delete ${selectedJobs.value.size} selected applications?`, {
    title: 'Confirm Batch Deletion',
    kind: 'warning'
  });

  if (!confirmed) return;

  try {
    await jobsStore.deleteJobsBatch(Array.from(selectedJobs.value));
    await loadJobs();
    isSelectionMode.value = false;
  } catch (err: any) {
    console.error('Batch delete error:', err);
  }
};

const deleteAllJobs = async () => {
  const confirmed = await ask('CRITICAL: This will delete ALL job applications and their tailored resumes. This action is permanent. Continue?', {
    title: 'CRITICAL: Delete All Data',
    kind: 'error'
  });

  if (!confirmed) return;

  try {
    await jobsStore.deleteAllJobs();
    await loadJobs();
  } catch (err: any) {
    console.error('Delete all error:', err);
  }
};

const selectAllVisible = () => {
  filteredAndSortedJobs.value.forEach(job => {
    selectedJobs.value.add(job.id);
  });
};

const exitSelectionMode = () => {
  isSelectionMode.value = false;
  selectedJobs.value.clear();
};

const filteredAndSortedJobs = computed(() => {
  let result = [...allJobs.value];

  // Search Filter
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    result = result.filter(j => 
      j.job_title.toLowerCase().includes(q) || 
      j.company_name.toLowerCase().includes(q)
    );
  }

  // Status Filter
  if (statusFilter.value !== 'All') {
    result = result.filter(j => j.status === statusFilter.value);
  }

  // Sort
  result.sort((a, b) => {
    switch (sortBy.value) {
      case 'date-desc':
        return new Date(b.created_at || 0).getTime() - new Date(a.created_at || 0).getTime();
      case 'date-asc':
        return new Date(a.created_at || 0).getTime() - new Date(b.created_at || 0).getTime();
      case 'title':
        return a.job_title.localeCompare(b.job_title);
      case 'company':
        return a.company_name.localeCompare(b.company_name);
      default:
        return 0;
    }
  });

  return result;
});

const chunkedJobs = computed(() => {
  const jobs = filteredAndSortedJobs.value;
  const chunks = [];
  for (let i = 0; i < jobs.length; i += columns.value) {
    chunks.push({
      id: `row-${i}`,
      items: jobs.slice(i, i + columns.value)
    });
  }
  return chunks;
});

const getStatusClass = (status: string) => {
  return `status-badge ${status.toLowerCase()}`;
};
</script>

<template>
  <div class="jobs-container">
    <header class="page-header">
      <div class="title-group">
        <h1>Application Vault</h1>
        <p class="subtitle" v-if="!isSelectionMode">Track and manage your professional opportunities.</p>
        <p class="subtitle selection-hint" v-else>Click items to select/deselect them.</p>
      </div>
      <div class="header-actions">
        <!-- Default Actions -->
        <template v-if="!isSelectionMode">
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'selection-mode'" @mouseleave="activeTooltip = null">
            <button class="btn-secondary" @click="isSelectionMode = true">
              <Settings2 :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'selection-mode'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                Selection Mode
              </Motion>
            </AnimatePresence>
          </div>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'delete-all'" @mouseleave="activeTooltip = null">
            <button class="btn-danger-outline" @click="deleteAllJobs">
              <Trash2 :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'delete-all'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                Delete All
              </Motion>
            </AnimatePresence>
          </div>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'new-app'" @mouseleave="activeTooltip = null">
            <button class="btn-primary" @click="$router.push('/parse')">
              <Plus :size="18" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'new-app'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                New Application
              </Motion>
            </AnimatePresence>
          </div>
        </template>

        <!-- Selection Mode Actions -->
        <template v-else>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'select-all'" @mouseleave="activeTooltip = null">
            <button class="btn-secondary" @click="selectAllVisible">
              <Check :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'select-all'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                Select All
              </Motion>
            </AnimatePresence>
          </div>
          <div class="btn-tooltip-wrapper" v-if="selectedJobs.size > 0" @mouseenter="activeTooltip = 'delete-batch'" @mouseleave="activeTooltip = null">
            <button class="btn-delete-batch" @click="deleteSelectedJobs">
              <Trash2 :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'delete-batch'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                Delete Selected ({{ selectedJobs.size }})
              </Motion>
            </AnimatePresence>
          </div>
          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'exit-selection'" @mouseleave="activeTooltip = null">
            <button class="btn-primary" @click="exitSelectionMode">
              <X :size="16" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'exit-selection'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="floating-message tooltip-bottom-left"
              >
                Done
              </Motion>
            </AnimatePresence>
          </div>
        </template>
      </div>
    </header>

    <div class="filters-bar" v-if="!isSelectionMode">
      <div class="search-box">
        <Search :size="18" class="search-icon" />
        <input v-model="searchQuery" placeholder="Search by title or company..." />
      </div>

      <div class="controls">
        <div 
          class="filter-group"
          @mouseenter="activeTooltip = 'status'"
          @mouseleave="activeTooltip = null"
          style="position: relative;"
        >
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'status'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
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
          @mouseenter="activeTooltip = 'sort'"
          @mouseleave="activeTooltip = null"
          style="position: relative;"
        >
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'sort'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-top"
            >
              Sort Order
            </Motion>
          </AnimatePresence>
          <CustomSelect
            v-model="sortBy"
            :options="[
              { value: 'date-desc', label: 'Newest First' },
              { value: 'date-asc', label: 'Oldest First' },
              { value: 'title', label: 'Job Title' },
              { value: 'company', label: 'Company' }
            ]"
            style="min-width: 150px;"
          >
            <template #icon>
              <ArrowUpDown :size="14" style="color: var(--muted);" />
            </template>
          </CustomSelect>
        </div>
      </div>
    </div>

    <div class="jobs-list-wrapper">
      <div v-if="jobsStore.isLoading" class="loading-state">
        Scanning vault...
      </div>
      <div v-else-if="filteredAndSortedJobs.length === 0" class="empty-state">
        <FolderOpen :size="48" class="empty-icon" />
        <h3>No applications found</h3>
        <p>Try adjusting your search or filters.</p>
      </div>
      
      <RecycleScroller
        v-else
        class="scroller"
        :items="chunkedJobs"
        :item-size="300"
        key-field="id"
        v-slot="{ item }"
      >
        <div class="job-row">
          <div 
            v-for="job in item.items" 
            :key="job.id"
            class="job-card"
            :class="{ 'selected': selectedJobs.has(job.id), 'selection-mode': isSelectionMode }"
            @click="handleCardClick(job.id)"
          >
            <div class="card-top">
              <div v-if="isSelectionMode" class="checkbox-indicator" :class="{ 'checked': selectedJobs.has(job.id) }">
                <span v-if="selectedJobs.has(job.id)">✓</span>
              </div>
              <span :class="getStatusClass(job.status)">{{ job.status }}</span>
              <span class="date">{{ job.created_at?.split(' ')[0] }}</span>
            </div>
            
            <h2 class="job-title">{{ job.job_title }}</h2>
            <p class="company-name">{{ job.company_name }}</p>
            
            <div class="tags">
              <span class="tag">{{ job.work_model }}</span>
              <span class="tag">{{ job.employment_type }}</span>
            </div>
            
            <div class="card-footer">
              <span class="view-link">View Details <ChevronRight :size="14" /></span>
            </div>
          </div>
          <!-- Spacer for grid alignment if row is not full -->
          <div v-for="n in (columns - item.items.length)" :key="'spacer-' + n" class="job-card-spacer"></div>
        </div>
      </RecycleScroller>
    </div>
  </div>
</template>

<style scoped>
.jobs-container {
  padding: 40px;
  max-width: 1200px;
  margin: 0 auto;
  min-height: 100%;
  display: flex;
  flex-direction: column;
}

.jobs-list-wrapper {
  flex: 1;
  min-height: 400px;
}

.scroller {
  height: 100%;
  padding: 0 12px;
}

.job-row {
  display: flex;
  gap: 24px;
  padding: 8px 12px 32px;
}

.job-card, .job-card-spacer {
  flex: 1;
  min-width: 0;
}

.job-card-spacer {
  visibility: hidden;
  pointer-events: none;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 32px;
}

.page-header h1 { font-size: 2.2rem; margin: 0; color: var(--ink); }
.subtitle { color: var(--muted); margin: 8px 0 0; }
.selection-hint { color: var(--accent); font-weight: 700; text-transform: uppercase; font-size: 0.75rem; letter-spacing: 0.05em; }

.header-actions { display: flex; gap: 12px; align-items: center; }

.btn-primary, .btn-secondary, .btn-danger-outline, .btn-delete-batch {
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: 0.2s;
  padding: 0;
}

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
}

.btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.btn-secondary {
  background: var(--surface-soft);
  color: var(--ink);
  border: 1px solid var(--line);
}

.btn-secondary:hover { background: var(--surface); border-color: var(--accent); }

.btn-danger-outline {
  background: transparent;
  color: var(--warning);
  border: 1px solid var(--warning);
}

.btn-danger-outline:hover { background: var(--warning); color: white; }

.btn-delete-batch {
  background: var(--warning);
  color: white;
  border: none;
}

.btn-delete-batch:hover { background: #e63946; }

.btn-primary:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(11, 123, 107, 0.2); }

.filters-bar {
  background: var(--surface);
  border: 1px solid var(--line);
  padding: 20px;
  border-radius: 16px;
  display: flex;
  gap: 24px;
  margin-bottom: 32px;
  align-items: center;
  box-shadow: var(--shadow);
}

.search-box {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
  background: var(--surface-soft);
  border-radius: 10px;
  padding: 0 16px;
  border: 1px solid var(--line);
  transition: all 0.2s ease;
}

.search-box:focus-within {
  border-color: #484f58;
}

.search-icon { color: var(--muted); }

.search-box input {
  width: 100%;
  padding: 12px 8px;
  background: none;
  border: none;
  color: var(--ink);
  outline: none;
  font-size: 0.95rem;
}

.controls { display: flex; gap: 12px; }

.filter-group { display: flex; flex-direction: column; gap: 6px; position: relative; }

.icon-select {
  flex-direction: row;
  align-items: center;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 10px;
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
  padding: 10px 32px 10px 4px;
  font-weight: 700;
  cursor: pointer;
  background: transparent;
  border: none;
  font-size: 0.8rem;
  color: var(--ink);
}

.filter-group select:focus {
  box-shadow: none;
}

.job-card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 24px;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow);
  overflow: hidden; /* Prevent content overflow */
  position: relative;
  min-width: 0; /* Allow card to shrink if needed */
  z-index: 1;
}

.job-title { 
  font-size: 1.25rem; 
  margin: 0; 
  color: var(--ink); 
  font-weight: 800;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
}

.company-name { 
  color: var(--accent); 
  font-weight: 700; 
  margin: 4px 0 16px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
}

.job-card:hover {
  transform: translateY(-4px) scale(1.01);
  border-color: var(--accent);
  box-shadow: 0 12px 32px rgba(0,0,0,0.12);
  z-index: 10;
}

.job-card.selection-mode:hover {
  transform: scale(1.01);
}

.job-card.selected {
  border-color: var(--accent);
  background: rgba(32, 201, 151, 0.08);
  box-shadow: 0 0 0 2px var(--accent);
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  position: relative;
}

.checkbox-indicator {
  width: 20px;
  height: 20px;
  border: 2px solid var(--line);
  border-radius: 6px;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--surface);
  color: white;
  font-weight: 900;
  transition: 0.2s;
}

.checkbox-indicator.checked {
  background: var(--accent);
  border-color: var(--accent);
}

.status-badge {
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
}

.status-badge.drafting { background: rgba(139, 148, 158, 0.15); color: #8b949e; }
.status-badge.applied { background: rgba(56, 139, 253, 0.15); color: #58a6ff; }
.status-badge.interviewing { background: rgba(210, 153, 34, 0.15); color: #d29922; }
.status-badge.offer { background: rgba(47, 129, 50, 0.15); color: #3fb950; }
.status-badge.rejected { background: rgba(248, 51, 73, 0.15); color: #f85149; }

.date { font-size: 0.8rem; color: var(--muted); font-family: monospace; }
.tags { display: flex; gap: 8px; margin-bottom: auto; }
.tag {
  background: var(--surface-soft);
  color: var(--muted);
  font-size: 0.75rem;
  padding: 4px 8px;
  border-radius: 6px;
  font-weight: 600;
}

.card-footer {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
}

.view-link {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--muted);
  transition: 0.2s;
  display: flex;
  align-items: center;
  gap: 4px;
}

.job-card:hover .view-link { color: var(--accent); }

.loading-state, .empty-state {
  grid-column: 1 / -1;
  text-align: center;
  padding: 80px 0;
  color: var(--muted);
}

.empty-icon { margin-bottom: 16px; opacity: 0.3; color: var(--muted); }

@media (max-width: 768px) {
  .jobs-container {
    padding: 16px;
  }
  .page-header { 
    flex-direction: row; 
    align-items: center; 
    margin-bottom: 20px; 
    gap: 12px;
  }
  .page-header h1 { font-size: 1.5rem; }
  .subtitle { display: none; }
  
  .filters-bar { 
    padding: 12px; 
    gap: 12px; 
    margin-bottom: 16px;
    flex-direction: column;
    align-items: stretch;
  }
  
  .controls { 
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  
  .search-box input {
    padding: 10px 8px;
    font-size: 0.85rem;
  }
  
  .icon-select {
    padding: 0 4px 0 8px;
  }
  
  .filter-group select {
    padding: 8px 24px 8px 4px;
    font-size: 0.75rem;
  }

  .btn-primary, .btn-secondary, .btn-danger-outline, .btn-delete-batch {
    width: 36px;
    height: 36px;
    border-radius: 10px;
  }
}

@media (max-width: 480px) {
  .jobs-container {
    padding: 16px;
  }
}
</style>
