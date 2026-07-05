<script setup lang="ts">
import { ref, onMounted, computed, watch, nextTick } from 'vue';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { save, message, ask } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Motion, AnimatePresence } from 'motion-v';
import { useSettingsStore } from '../store/settings';
import { useResumesStore } from '../store/resumes';
import { useCoverLettersStore } from '../store/cover_letters';
import { useDialogStore } from '../store/dialog';
import { useJobsStore, Job } from '../store/jobs';
import CustomSelect from './CustomSelect.vue';
import VuePdfEmbed from 'vue-pdf-embed';
// Codemirror imports
import { Codemirror } from 'vue-codemirror';
import { latex, latexLanguage, autoCloseTags } from 'codemirror-lang-latex';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';

import { 
  ArrowLeft, 
  Trash2, 
  ExternalLink, 
  Save, 
  Hammer, 
  Download, 
  Wand2, 
  Play,
  RotateCw,
  Loader2,
  Info,
  ListChecks,
  Settings,
  Briefcase,
  Layers,
  Activity,
  Mail,
  FileText,
  Columns
} from '@lucide/vue';

interface TemplateItem {
  id: string;
  name: string;
}

interface TailoredContent {
  id: string;
  base_template_id: string;
  content: string;
}

const router = useRouter();
const settingsStore = useSettingsStore();
const resumesStore = useResumesStore();
const clStore = useCoverLettersStore();
const dialog = useDialogStore();
const jobsStore = useJobsStore();

const props = defineProps<{ id: string }>();

// Tracking tailored IDs
const tailoredResumeId = ref<string | null>(null);
const tailoredClId = ref<string | null>(null);

// Dirty State Tracking
const isResumeDirty = ref(false);
const isClDirty = ref(false);

const hasUnsavedChanges = computed(() => {
  return activeMode.value === 'resume' ? isResumeDirty.value : isClDirty.value;
});

// Codemirror Extensions
const extensions = [
  latex(),
  latexLanguage,
  ...autoCloseTags,
  oneDark,
  EditorView.lineWrapping
];

// Tooltip State
const activeTooltip = ref<string | null>(null);

// Global State
const isLoading = ref(true);
const isGeneratingResume = ref(false);
const isGeneratingCl = ref(false);
const isGenerating = computed(() => activeMode.value === 'resume' ? isGeneratingResume.value : isGeneratingCl.value);

const isCompilingResume = ref(false);
const isCompilingCl = ref(false);
const isCompilingPDF = computed(() => activeMode.value === 'resume' ? isCompilingResume.value : isCompilingCl.value);

const error = ref<string | null>(null);
const activeMode = ref<'resume' | 'cl'>('resume');
const jobDetails = ref<Job | null>(null);
const editorContainer = ref<HTMLElement | null>(null);

// Resume Specific State
const resumeSelectedId = ref<string | null>(null);
const resumeInstruction = ref('');
const resumeLatex = ref('');
const resumePdfUrl = ref<any>(null);
const resumePdfBytes = ref<Uint8Array | null>(null);
const resumeCompError = ref<string | null>(null);

// Cover Letter Specific State
const clSelectedId = ref<string | null>(null);
const clInstruction = ref('');
const clLatex = ref('');
const clPdfUrl = ref<any>(null);
const clPdfBytes = ref<Uint8Array | null>(null);
const clCompError = ref<string | null>(null);

// Common Editor/Preview State (Active)
const isDownloading = ref(false);
const isFixingResume = ref(false);
const isFixingCl = ref(false);
const isFixing = computed(() => activeMode.value === 'resume' ? isFixingResume.value : isFixingCl.value);

const isRefiningResume = ref(false);
const isRefiningCl = ref(false);
const isRefining = computed(() => activeMode.value === 'resume' ? isRefiningResume.value : isRefiningCl.value);

const refinementInstruction = ref('');

const isResumeCompiled = ref(false);
const isClCompiled = ref(false);

// Compare State
const isComparing = ref(false);
const basePdfUrl = ref<any>(null);
const isCompilingBase = ref(false);

const toggleCompare = async () => {
  isComparing.value = !isComparing.value;
  if (isComparing.value && !basePdfUrl.value) {
    isCompilingBase.value = true;
    try {
      const baseId = activeMode.value === 'resume' ? resumeSelectedId.value : clSelectedId.value;
      if (!baseId) throw new Error("No base template selected");

      let baseLatex = '';
      if (activeMode.value === 'resume') {
        const r = await resumesStore.getResumeById(baseId);
        baseLatex = r.latex_content || '';
      } else {
        const c = await clStore.getCoverLetterById(baseId);
        baseLatex = c.latex_content || '';
      }
      
      const baseFilename = activeMode.value === 'resume' ? 'base_resume_compiled_roletect.pdf' : 'base_cover_letter_compiled_roletect.pdf';
      await invoke<number[]>('compile_resume_to_pdf', { 
        latexCode: baseLatex,
        filename: baseFilename
      });
      
      const port = await invoke<string>('get_setting', { key: 'active_server_port', default_value: '1420' });
      basePdfUrl.value = {
        url: `http://127.0.0.1:${port}/static-pdf/${baseFilename}?cache-bust=${Date.now()}`,
        disableRange: false,
        disableStream: false,
        rangeChunkSize: 1024 * 1024
      };
    } catch (err: any) {
      console.error("Failed to compile base PDF:", err);
      isComparing.value = false;
      error.value = `Compare Error: ${err.toString()}`;
    } finally {
      isCompilingBase.value = false;
    }
  }
};

// Resizer State
const previewWidth = ref(450);
const isResizingPreview = ref(false);
const splitPaneRef = ref<HTMLElement | null>(null);

const startResizingPreview = (_e: MouseEvent) => {
  isResizingPreview.value = true;
  document.addEventListener('mousemove', handlePreviewMouseMove);
  document.addEventListener('mouseup', stopResizingPreview);
};

const handlePreviewMouseMove = (e: MouseEvent) => {
  if (!isResizingPreview.value || !splitPaneRef.value) return;
  const rect = splitPaneRef.value.getBoundingClientRect();
  const newWidth = rect.right - e.clientX;
  if (newWidth < 100) {
    previewWidth.value = 100;
    return;
  }
  const minWidth = 100;
  const maxWidth = rect.width - 200; // leave space for editor
  previewWidth.value = Math.max(minWidth, Math.min(maxWidth, newWidth));
};

const stopResizingPreview = () => {
  isResizingPreview.value = false;
  document.removeEventListener('mousemove', handlePreviewMouseMove);
  document.removeEventListener('mouseup', stopResizingPreview);
  nextTick(() => {
    window.dispatchEvent(new Event('resize'));
  });
};

// Computed bindings for active mode
const activeLatex = computed({
  get: () => activeMode.value === 'resume' ? resumeLatex.value : clLatex.value,
  set: (val) => {
    if (activeMode.value === 'resume') resumeLatex.value = val;
    else clLatex.value = val;
  }
});

const activePdfUrl = computed(() => activeMode.value === 'resume' ? resumePdfUrl.value : clPdfUrl.value);
const activeCompError = computed({
  get: () => activeMode.value === 'resume' ? resumeCompError.value : clCompError.value,
  set: (val) => {
    if (activeMode.value === 'resume') resumeCompError.value = val;
    else clCompError.value = val;
  }
});

const activePdfBytes = computed(() => activeMode.value === 'resume' ? resumePdfBytes.value : clPdfBytes.value);

// Template data
const standardResumes = ref<TemplateItem[]>([]);
const standardCls = ref<TemplateItem[]>([]);
const isLoadingTemplates = ref(false);

// Helper to parse JSON fields safely
const parseJsonField = (field: string | undefined | null): string[] => {
  if (!field) return [];
  try {
    return JSON.parse(field);
  } catch (e) {
    // Fallback for legacy data that was saved as newline-separated strings
    return field.split('\n').map(s => s.trim()).filter(s => s.length > 0);
  }
};

// Load job details and base templates on mount
onMounted(async () => {
  try {
    // 1. Fetch job details from backend
    jobDetails.value = await jobsStore.getJobById(props.id);
    resumeInstruction.value = jobDetails.value.custom_instruction || '';
    clInstruction.value = jobDetails.value.custom_instruction || '';
    
    // 2. Load templates
    isLoadingTemplates.value = true;
    
    // Resume Templates
    await resumesStore.loadAllResumes();
    const withResumeContent: TemplateItem[] = [];
    for (const r of resumesStore.resumes) {
      const detail = await resumesStore.getResumeById(r.id);
      if (detail.latex_content && detail.latex_content.trim().length > 0) {
        withResumeContent.push({ id: r.id, name: r.name });
      }
    }
    standardResumes.value = withResumeContent;
    if (standardResumes.value.length > 0) resumeSelectedId.value = standardResumes.value[0].id;

    // Cover Letter Templates
    await clStore.loadAllCoverLetters();
    const withClContent: TemplateItem[] = [];
    for (const c of clStore.coverLetters) {
      const detail = await clStore.getCoverLetterById(c.id);
      if (detail.latex_content && detail.latex_content.trim().length > 0) {
        withClContent.push({ id: c.id, name: c.name });
      }
    }
    standardCls.value = withClContent;
    if (standardCls.value.length > 0) clSelectedId.value = standardCls.value[0].id;

    isLoadingTemplates.value = false;

    // 3. Fetch latest tailored content
    const latestResume = await invoke<TailoredContent | null>('get_latest_tailored_resume', { jobId: props.id });
    if (latestResume) {
      resumeLatex.value = latestResume.content;
      tailoredResumeId.value = latestResume.id;
      resumeSelectedId.value = latestResume.base_template_id;
    }

    const latestCl = await invoke<TailoredContent | null>('get_latest_tailored_cover_letter', { jobId: props.id });
    if (latestCl) {
      clLatex.value = latestCl.content;
      tailoredClId.value = latestCl.id;
      clSelectedId.value = latestCl.base_template_id;
    }

    // Initialize dirty state tracking after initial load
    setTimeout(() => {
      watch(resumeLatex, () => { isResumeDirty.value = true; });
      watch(clLatex, () => { isClDirty.value = true; });
    }, 500);

  } catch (err: any) {
    error.value = err.toString();
    isLoadingTemplates.value = false;
  } finally {
    isLoading.value = false;
  }
});

import { onUnmounted } from 'vue';
onUnmounted(() => {
  isResumeCompiled.value = false;
  isClCompiled.value = false;
  resumePdfUrl.value = null;
  clPdfUrl.value = null;
});

// Trigger AI Generation
const generateContent = async () => {
  const targetMode = activeMode.value;
  const isResume = targetMode === 'resume';
  const selectedTemplate = isResume ? resumeSelectedId.value : clSelectedId.value;
  
  if (!jobDetails.value || !selectedTemplate) return;
  
  if (isResume) isGeneratingResume.value = true;
  else isGeneratingCl.value = true;
  
  error.value = null;
  
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");
    
    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    if (isResume) {
      const tailoredId = await invoke<string>('tailor_resume', {
        provider,
        model,
        apiKey,
        jobId: props.id,
        baseResumeId: selectedTemplate,
        customInstruction: resumeInstruction.value || null,
      });
      tailoredResumeId.value = tailoredId;
      resumeLatex.value = await invoke<string>('get_tailored_resume', { id: tailoredId });
      isResumeDirty.value = false;
    } else {
      const tailoredId = await invoke<string>('tailor_cover_letter', {
        provider,
        model,
        apiKey,
        jobId: props.id,
        baseClId: selectedTemplate,
        customInstruction: clInstruction.value || null,
      });
      tailoredClId.value = tailoredId;
      clLatex.value = await invoke<string>('get_tailored_cover_letter', { id: tailoredId });
      isClDirty.value = false;
    }
  } catch (err: any) {
    console.error("Tailoring Error:", err);
    error.value = err.toString();
  } finally {
    if (isResume) isGeneratingResume.value = false;
    else isGeneratingCl.value = false;
  }
};

const refineWithAi = async () => {
  const targetMode = activeMode.value;
  const currentLatex = targetMode === 'resume' ? resumeLatex.value : clLatex.value;
  const instruction = refinementInstruction.value.trim();
  const isCurrentlyRefining = targetMode === 'resume' ? isRefiningResume.value : isRefiningCl.value;
  
  if (!currentLatex || !instruction || isCurrentlyRefining) return;
  
  if (targetMode === 'resume') isRefiningResume.value = true;
  else isRefiningCl.value = true;
  
  error.value = null;

  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const refinedCode = await invoke<string>('refine_latex_with_ai', {
      provider,
      model,
      apiKey,
      currentLatex: currentLatex,
      instruction: instruction
    });

    if (targetMode === 'resume') resumeLatex.value = refinedCode;
    else clLatex.value = refinedCode;
    
    refinementInstruction.value = '';
    error.value = `AI has refined the ${targetMode}. Re-compiling...`;
    
    await doCompilePdf(targetMode);
  } catch (err: any) {
    console.error("AI Refinement Error:", err);
    error.value = `AI Refinement failed: ${err.toString()}`;
  } finally {
    if (targetMode === 'resume') isRefiningResume.value = false;
    else isRefiningCl.value = false;
  }
};

const doSaveLatexContent = async (targetMode: 'resume' | 'cl', silent = false) => {
  try {
    if (targetMode === 'resume') {
      await invoke('update_tailored_resume', {
        jobId: props.id,
        baseResumeId: resumeSelectedId.value,
        latexContent: resumeLatex.value
      });
      isResumeDirty.value = false;
    } else {
      await invoke('update_tailored_cover_letter', {
        jobId: props.id,
        baseClId: clSelectedId.value,
        latexContent: clLatex.value
      });
      isClDirty.value = false;
    }
    if (!silent) await message('Content saved successfully.', { title: 'Success', kind: 'info' });
  } catch (err: any) {
    console.error("Save Error:", err);
    if (!silent) await message(`Failed to save changes: ${err.toString()}`, { title: 'Save Failed', kind: 'error' });
  }
};

const saveLatexContent = (silent = false) => doSaveLatexContent(activeMode.value, typeof silent === 'boolean' ? silent : false);

const doCompilePdf = async (targetMode: 'resume' | 'cl') => {
  const currentLatex = targetMode === 'resume' ? resumeLatex.value : clLatex.value;
  if (!currentLatex) return;
  
  if (targetMode === 'resume') isCompilingResume.value = true;
  else isCompilingCl.value = true;
  
  error.value = null;
  if (targetMode === 'resume') resumeCompError.value = null;
  else clCompError.value = null;
  
  try {
    const pdfFilename = targetMode === 'resume' ? 'resume_compiled_roletect.pdf' : 'cover_compiled_roletect.pdf';
    const pdfBytes = await invoke<number[]>('compile_resume_to_pdf', { 
      latexCode: currentLatex,
      filename: pdfFilename
    });
    
    const bytes = new Uint8Array(pdfBytes);
    
    // Fetch port from DB
    const port = await invoke<string>('get_setting', { key: 'active_server_port', default_value: '1420' });
    const sourceObj = {
      url: `http://127.0.0.1:${port}/static-pdf/${pdfFilename}?cache-bust=${Date.now()}`,
      disableRange: false,
      disableStream: false,
      rangeChunkSize: 1024 * 1024 // 1MB chunks
    };
    
    if (targetMode === 'resume') {
      resumePdfBytes.value = bytes;
      resumePdfUrl.value = sourceObj;
      isResumeCompiled.value = true;
    } else {
      clPdfBytes.value = bytes;
      clPdfUrl.value = sourceObj;
      isClCompiled.value = true;
    }

    await doSaveLatexContent(targetMode, true); // Silent save after successful compilation
  } catch (err: any) {
    console.error("PDF Compilation Error:", err);
    if (targetMode === 'resume') resumeCompError.value = err.toString();
    else clCompError.value = err.toString();
    error.value = "LaTeX Compilation Failed. You can try 'AI Fix' or manually edit and Save.";
  } finally {
    if (targetMode === 'resume') isCompilingResume.value = false;
    else isCompilingCl.value = false;
  }
};

const compilePdf = () => doCompilePdf(activeMode.value);

const onPdfError = (err: any) => {
  console.error("PDF Rendering Error:", err);
  const errMsg = "Frontend Rendering Error: Failed to stream or parse PDF chunks from the backend. " + (err.message || err.toString());
  if (activeMode.value === 'resume') {
    resumeCompError.value = errMsg;
  } else {
    clCompError.value = errMsg;
  }
};

const handleTabSwitch = async (mode: 'resume' | 'cl') => {
  if (activeMode.value === mode) return;
  
  if (hasUnsavedChanges.value) {
    const confirmed = await ask(
      `You have unsaved changes in your tailored ${activeMode.value === 'resume' ? 'resume' : 'cover letter'}. Are you sure you want to switch tabs? Changes will be lost unless saved.`,
      { title: 'Unsaved Changes', kind: 'warning' }
    );
    if (!confirmed) return;
  }
  
  isComparing.value = false;
  basePdfUrl.value = null;
  
  isResumeCompiled.value = false;
  resumePdfUrl.value = null;
  resumePdfBytes.value = null;
  isClCompiled.value = false;
  clPdfUrl.value = null;
  clPdfBytes.value = null;
  
  activeMode.value = mode;
};

const fixWithAi = async () => {
  const targetMode = activeMode.value;
  const currentLatex = targetMode === 'resume' ? resumeLatex.value : clLatex.value;
  const currentCompError = targetMode === 'resume' ? resumeCompError.value : clCompError.value;
  const isCurrentlyFixing = targetMode === 'resume' ? isFixingResume.value : isFixingCl.value;

  if (!currentLatex || !currentCompError || isCurrentlyFixing) return;
  
  if (targetMode === 'resume') isFixingResume.value = true;
  else isFixingCl.value = true;
  
  error.value = null;

  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const fixedCode = await invoke<string>('fix_latex_with_ai', {
      provider,
      model,
      apiKey,
      brokenLatex: currentLatex,
      errorLogs: currentCompError
    });

    if (targetMode === 'resume') resumeLatex.value = fixedCode;
    else clLatex.value = fixedCode;
    
    error.value = "AI has suggested a fix. Trying to re-compile...";
    await doCompilePdf(targetMode);
  } catch (err: any) {
    console.error("AI Fix Error:", err);
    error.value = `AI Fix failed: ${err.toString()}`;
  } finally {
    if (targetMode === 'resume') isFixingResume.value = false;
    else isFixingCl.value = false;
  }
};

const downloadPdf = async () => {
  if (!activePdfBytes.value) return;
  isDownloading.value = true;
  
  try {
    const now = new Date();
    const timestamp = `${now.getFullYear()}${(now.getMonth() + 1).toString().padStart(2, '0')}${now.getDate().toString().padStart(2, '0')}_${now.getHours().toString().padStart(2, '0')}${now.getMinutes().toString().padStart(2, '0')}${now.getSeconds().toString().padStart(2, '0')}`;
    
    const typeLabel = activeMode.value === 'resume' ? 'resume' : 'cover_letter';
    const defaultName = `${typeLabel}_${timestamp}.pdf`;

    const filePath = await save({
      filters: [{ name: 'PDF Document', extensions: ['pdf'] }],
      defaultPath: defaultName
    });

    if (filePath) {
      await writeFile(filePath, activePdfBytes.value);
      
      const filename = filePath.split(/[/\\]/).pop() || defaultName;
      const downloadType = activeMode.value === 'resume' ? 'tailored_resume' : 'tailored_cover_letter';
      const contentId = activeMode.value === 'resume' ? tailoredResumeId.value : tailoredClId.value;

      await invoke('record_download', {
        filename,
        downloadType,
        jobId: jobDetails.value?.id,
        contentId: contentId || null
      });

      await message('PDF saved successfully.', { title: 'Success', kind: 'info' });
    }
  } catch (err: any) {
    console.error("Download Error:", err);
    error.value = `Failed to save PDF: ${err.toString()}`;
  } finally {
    isDownloading.value = false;
  }
};

const openJobUrl = async () => {
  if (jobDetails.value?.job_url) {
    try {
      await openUrl(jobDetails.value.job_url);
    } catch (err: any) {
      console.error("Failed to open URL:", err);
      error.value = `Failed to open URL: ${err.toString()}`;
    }
  }
};

const goBack = () => router.push('/');

const updateStatus = async (newStatus: string) => {
  if (jobDetails.value?.status === newStatus) return;

  let metadata: Record<string, string> = {};
  const today = new Date().toISOString().split('T')[0];

  try {
    let datePrompt = '';
    let metaKey = '';

    if (newStatus === 'Applied') {
      datePrompt = 'Select application date:';
      metaKey = 'applied_date';
    } else if (newStatus === 'Interviewing') {
      datePrompt = 'Select interview date:';
      metaKey = 'interview_date';
    } else if (newStatus === 'Offer') {
      datePrompt = 'Select offer received date:';
      metaKey = 'offer_date';
      await dialog.showAlert('Amazing! You received an offer. You can add the salary details in the job info section.', 'Offer Received');
    } else if (newStatus === 'Rejected') {
      datePrompt = 'Select rejection date:';
      metaKey = 'rejected_date';
    } else if (newStatus === 'Joined') {
      datePrompt = 'Select start date:';
      metaKey = 'joining_date';
    }

    if (metaKey) {
      const result = await dialog.showDatePicker(datePrompt, today, 'Record Milestone');
      if (result === null) return; // User cancelled
      metadata[metaKey] = result;
    }

    await jobsStore.updateJobStatus(props.id, newStatus, Object.keys(metadata).length > 0 ? metadata : undefined);
    
    // Refresh local data
    jobDetails.value = await jobsStore.getJobById(props.id);
  } catch (err: any) {
    error.value = `Failed to update status: ${err.toString()}`;
  }
};

const editSalary = async () => {
  const result = await dialog.showPrompt('Enter the salary (e.g. $120k/yr):', jobDetails.value?.salary || '', 'Update Salary');

  if (result !== null) {
    try {
      await jobsStore.updateJobMetadata(props.id, 'salary', result);
      jobDetails.value = await jobsStore.getJobById(props.id);
    } catch (err: any) {
      error.value = `Failed to update salary: ${err.toString()}`;
    }
  }
};

const deleteJob = async () => {
  const confirmed = await dialog.showConfirm('Are you sure you want to delete this job application? This action cannot be undone.', 'Confirm Deletion');

  if (!confirmed) return;
  
  try {
    await jobsStore.deleteJob(props.id);
    router.push('/jobs');
  } catch (err: any) {
    error.value = err.toString();
  }
};
</script>

<template>
  <div class="workspace" v-if="!isLoading">
    <header class="workspace-header">
      <div class="header-left">
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
              Back to List
            </Motion>
          </AnimatePresence>
        </div>
        <div class="job-info">
          <h1 class="title">{{ jobDetails?.job_title }}</h1>
          <span class="company">{{ jobDetails?.company_name }}</span>
          <div class="btn-tooltip-wrapper" v-if="jobDetails?.job_url" @mouseenter="activeTooltip = 'job-link'" @mouseleave="activeTooltip = null">
            <button class="link-btn" @click="openJobUrl"><ExternalLink :size="14" /></button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'job-link'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="flying-message header-tooltip"
              >
                Open Job Link
              </Motion>
            </AnimatePresence>
          </div>
        </div>
      </div>
      <div class="header-actions">
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'delete-job'" @mouseleave="activeTooltip = null">
          <button class="delete-btn" @click="deleteJob"><Trash2 :size="16" /></button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'delete-job'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="flying-message header-tooltip delete-tooltip"
            >
              Delete Application
            </Motion>
          </AnimatePresence>
        </div>
      </div>
    </header>

    <AnimatePresence>
      <Motion
        v-if="error"
        :initial="{ height: 0, opacity: 0 }"
        :animate="{ height: 'auto', opacity: 1 }"
        :exit="{ height: 0, opacity: 0 }"
        class="error-banner"
      >
        <span>{{ error }}</span>
        <button @click="error = null">✕</button>
      </Motion>
    </AnimatePresence>

    <div class="split-view">
      <aside class="panel info-panel">
        <div class="section">
          <div class="section-header-icon" @mouseenter="activeTooltip = 'info-sec'" @mouseleave="activeTooltip = null">
            <Info :size="16" />
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'info-sec'"
                :initial="{ opacity: 0, x: 5, scale: 0.9 }"
                :animate="{ opacity: 1, x: 12, scale: 1 }"
                :exit="{ opacity: 0, x: 5, scale: 0.9 }"
                class="flying-message sidebar-tooltip"
              >
                Information
              </Motion>
            </AnimatePresence>
          </div>
          <div class="meta-grid">
            <div class="meta-icon-wrapper" @mouseenter="activeTooltip = 'work-model'" @mouseleave="activeTooltip = null">
              <Briefcase :size="14" />
              <AnimatePresence>
                <Motion v-if="activeTooltip === 'work-model'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Work Model</Motion>
              </AnimatePresence>
            </div>
            <span class="value">{{ jobDetails?.work_model }}</span>
            
            <div class="meta-icon-wrapper" @mouseenter="activeTooltip = 'emp-type'" @mouseleave="activeTooltip = null">
              <Layers :size="14" />
              <AnimatePresence>
                <Motion v-if="activeTooltip === 'emp-type'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Employment Type</Motion>
              </AnimatePresence>
            </div>
            <span class="value">{{ jobDetails?.employment_type }}</span>
            
            <div class="meta-icon-wrapper" @mouseenter="activeTooltip = 'status-meta'" @mouseleave="activeTooltip = null">
              <Activity :size="14" />
              <AnimatePresence>
                <Motion v-if="activeTooltip === 'status-meta'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Application Status</Motion>
              </AnimatePresence>
            </div>
            <CustomSelect 
              :model-value="jobDetails?.status" 
              @change="updateStatus"
              :options="['Drafting', 'Applied', 'Interviewing', 'Offer', 'Rejected', 'Joined'].map(s => ({ value: s, label: s }))"
              style="width: 140px;"
            />
          </div>

          <!-- Milestones Section (Context Aware) -->
          <div class="milestones-section" v-if="jobDetails?.status !== 'Drafting'">
            <!-- Show Applied Date only if Applied -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Applied' && jobDetails?.applied_date">
              <span class="milestone-label">Applied On</span>
              <span class="milestone-value">{{ jobDetails.applied_date }}</span>
            </div>

            <!-- Show Interview Date only if Interviewing -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Interviewing' && jobDetails?.interview_date">
              <span class="milestone-label">Interview On</span>
              <span class="milestone-value">{{ jobDetails.interview_date }}</span>
            </div>

            <!-- Show Offer Date only if Offer Recv -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Offer' && jobDetails?.offer_date">
              <span class="milestone-label">Offer Recv</span>
              <span class="milestone-value">{{ jobDetails.offer_date }}</span>
            </div>

            <!-- Show Rejected Date only if Rejected -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Rejected' && jobDetails?.rejected_date">
              <span class="milestone-label">Rejected On</span>
              <span class="milestone-value">{{ jobDetails.rejected_date }}</span>
            </div>

            <!-- Show Start Date only if Joined -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Joined' && jobDetails?.joining_date">
              <span class="milestone-label">Start Date</span>
              <span class="milestone-value">{{ jobDetails.joining_date }}</span>
            </div>

            <!-- Salary (Shown for Offer or Joined) -->
            <div class="milestone-row" v-if="jobDetails?.status === 'Offer' || jobDetails?.status === 'Joined'">
              <span class="milestone-label">Salary</span>
              <button 
                v-if="jobDetails?.salary" 
                class="salary-badge" 
                @click="editSalary"
                title="Click to edit salary"
              >
                {{ jobDetails.salary }}
              </button>
              <button 
                v-else
                class="edit-salary-btn" 
                @click="editSalary"
              >
                + Add Salary
              </button>
            </div>
          </div>
        </div>

        <div class="section scroll-section" v-if="jobDetails?.requirements">
          <div class="section-header-icon" @mouseenter="activeTooltip = 'req-sec'" @mouseleave="activeTooltip = null">
            <ListChecks :size="16" />
            <AnimatePresence>
              <Motion v-if="activeTooltip === 'req-sec'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Requirements</Motion>
            </AnimatePresence>
          </div>
          <ul class="tight-list">
            <li v-for="req in parseJsonField(jobDetails.requirements)" :key="req">{{ req }}</li>
          </ul>
        </div>

        <div class="section scroll-section" v-if="jobDetails?.core_responsibilities">
          <div class="section-header-icon" @mouseenter="activeTooltip = 'resp-sec'" @mouseleave="activeTooltip = null">
            <Briefcase :size="16" />
            <AnimatePresence>
              <Motion v-if="activeTooltip === 'resp-sec'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Responsibilities</Motion>
            </AnimatePresence>
          </div>
          <ul class="tight-list">
            <li v-for="resp in parseJsonField(jobDetails.core_responsibilities)" :key="resp">{{ resp }}</li>
          </ul>
        </div>

        <div class="section footer-section">
          <div class="section-header-icon" @mouseenter="activeTooltip = 'config-sec'" @mouseleave="activeTooltip = null">
            <Settings :size="16" />
            <AnimatePresence>
              <Motion v-if="activeTooltip === 'config-sec'" class="flying-message sidebar-tooltip" :initial="{ opacity: 0, x: 5 }" :animate="{ opacity: 1, x: 12 }">Configuration ({{ activeMode === 'resume' ? 'Resume' : 'CL' }})</Motion>
            </AnimatePresence>
          </div>
          
          <div class="form-group">
            <label>Base Template</label>
            <CustomSelect 
              v-if="activeMode === 'resume'" 
              v-model="resumeSelectedId" 
              :options="standardResumes.map(r => ({ value: r.id, label: r.name }))" 
            />
            <CustomSelect 
              v-else 
              v-model="clSelectedId" 
              :options="standardCls.map(c => ({ value: c.id, label: c.name }))" 
            />
          </div>

          <div class="form-group">
            <label>Tailor Logic</label>
            <textarea 
              v-if="activeMode === 'resume'"
              v-model="resumeInstruction" 
              class="compact-textarea" 
              placeholder="Resume tailoring rules..."
            ></textarea>
            <textarea 
              v-else
              v-model="clInstruction" 
              class="compact-textarea" 
              placeholder="Cover letter rules..."
            ></textarea>
          </div>

          <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'run-intelligence'" @mouseleave="activeTooltip = null">
            <button class="btn-accent w-full" @click="generateContent" :disabled="isGenerating || (activeMode === 'resume' ? !resumeSelectedId : !clSelectedId)">
              <Play v-if="!isGenerating" :size="14" />
              <RotateCw v-else :size="14" class="spinner" />
            </button>
            <AnimatePresence>
              <Motion
                v-if="activeTooltip === 'run-intelligence'"
                :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                :animate="{ opacity: 1, y: 0, scale: 1 }"
                :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                :transition="{ duration: 0.15 }"
                class="flying-message info-tooltip"
              >
                {{ isGenerating ? 'Tailoring...' : 'Run Intelligence' }}
              </Motion>
            </AnimatePresence>
          </div>
        </div>
      </aside>

      <div class="panel main-panel">
        <div class="panel-tabs">
          <div class="left-tabs">
            <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'resume-mode'" @mouseleave="activeTooltip = null">
              <button 
                class="tab-btn-mode" 
                :class="{ active: activeMode === 'resume' }" 
                @click="handleTabSwitch('resume')"
              >
                <FileText :size="14" />
                <span>RESUME</span>
              </button>
              <AnimatePresence>
                <Motion v-if="activeTooltip === 'resume-mode'" class="flying-message tab-tooltip" :initial="{ opacity: 0, y: 5 }" :animate="{ opacity: 1, y: 0 }">Resume Workspace</Motion>
              </AnimatePresence>
            </div>
            <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'cl-mode'" @mouseleave="activeTooltip = null">
              <button 
                class="tab-btn-mode" 
                :class="{ active: activeMode === 'cl' }" 
                @click="handleTabSwitch('cl')"
              >
                <Mail :size="14" />
                <span>COVER LETTER</span>
              </button>
              <AnimatePresence>
                <Motion v-if="activeTooltip === 'cl-mode'" class="flying-message tab-tooltip" :initial="{ opacity: 0, y: 5 }" :animate="{ opacity: 1, y: 0 }">Cover Letter Workspace</Motion>
              </AnimatePresence>
            </div>
            <div class="divider"></div>
            <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'save-content'" @mouseleave="activeTooltip = null">
              <button class="tab-btn" @click="saveLatexContent(false)"><Save :size="14" /></button>
              <AnimatePresence>
                <Motion
                  v-if="activeTooltip === 'save-content'"
                  :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                  :animate="{ opacity: 1, y: 0, scale: 1 }"
                  :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                  :transition="{ duration: 0.15 }"
                  class="flying-message tab-tooltip"
                >
                  Save LaTeX
                </Motion>
              </AnimatePresence>
            </div>
          </div>
          <div class="right-tabs">
            <AnimatePresence>
              <div class="btn-tooltip-wrapper" v-if="activeCompError" @mouseenter="activeTooltip = 'ai-fix'" @mouseleave="activeTooltip = null">
                <Motion
                  :initial="{ scale: 0.9, opacity: 0 }"
                  :animate="{ scale: 1, opacity: 1 }"
                  class="tab-btn ai-btn"
                  @click="fixWithAi"
                  :disabled="isFixing"
                >
                  <Wand2 :size="14" />
                </Motion>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'ai-fix'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    :transition="{ duration: 0.15 }"
                    class="flying-message tab-tooltip"
                  >
                    AI Debug & Fix
                  </Motion>
                </AnimatePresence>
              </div>
            </AnimatePresence>
            <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'build-pdf'" @mouseleave="activeTooltip = null">
              <button class="tab-btn accent-btn" @click="compilePdf" :disabled="!activeLatex || isCompilingPDF">
                <Hammer v-if="!isCompilingPDF" :size="14" />
                <RotateCw v-else :size="14" class="spinner" />
              </button>
              <AnimatePresence>
                <Motion
                  v-if="activeTooltip === 'build-pdf'"
                  :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                  :animate="{ opacity: 1, y: 0, scale: 1 }"
                  :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                  :transition="{ duration: 0.15 }"
                  class="flying-message tab-tooltip"
                >
                  Compile PDF
                </Motion>
              </AnimatePresence>
            </div>
            <div class="btn-tooltip-wrapper" v-if="activeMode === 'resume' ? isResumeCompiled : isClCompiled" @mouseenter="activeTooltip = 'compare'" @mouseleave="activeTooltip = null">
              <button class="tab-btn" :class="{ 'active': isComparing }" @click="toggleCompare" :disabled="isCompilingBase">
                <Columns v-if="!isCompilingBase && !isComparing" :size="14" />
                <FileText v-else-if="!isCompilingBase && isComparing" :size="14" />
                <RotateCw v-else :size="14" class="spinner" />
              </button>
              <AnimatePresence>
                <Motion
                  v-if="activeTooltip === 'compare'"
                  :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                  :animate="{ opacity: 1, y: 0, scale: 1 }"
                  :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                  :transition="{ duration: 0.15 }"
                  class="flying-message tab-tooltip"
                >
                  {{ isComparing ? 'Exit Compare' : 'Compare with Base' }}
                </Motion>
              </AnimatePresence>
            </div>
            <div class="btn-tooltip-wrapper" v-if="activePdfBytes" @mouseenter="activeTooltip = 'export-pdf'" @mouseleave="activeTooltip = null">
              <button class="tab-btn" @click="downloadPdf" :disabled="isDownloading">
                <Download v-if="!isDownloading" :size="14" />
                <RotateCw v-else :size="14" class="spinner" />
              </button>
              <AnimatePresence>
                <Motion
                  v-if="activeTooltip === 'export-pdf'"
                  :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                  :animate="{ opacity: 1, y: 0, scale: 1 }"
                  :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                  :transition="{ duration: 0.15 }"
                  class="flying-message tab-tooltip"
                >
                  Download PDF
                </Motion>
              </AnimatePresence>
            </div>
          </div>
        </div>

        <AnimatePresence>
          <Motion
            v-if="activeCompError"
            :initial="{ height: 0 }"
            :animate="{ height: 'auto' }"
            :exit="{ height: 0 }"
            class="error-log"
          >
            <header>
              <span>COMPILATION ERROR</span>
              <button @click="activeCompError = null">✕</button>
            </header>
            <pre>{{ activeCompError }}</pre>
          </Motion>
        </AnimatePresence>

        <div class="split-pane" ref="splitPaneRef" :class="{ 'is-resizing': isResizingPreview }">
          
          <!-- Base PDF Viewer (Compare Mode) -->
          <div v-if="isComparing" class="pdf-viewer base-pdf-viewer" style="flex: 1; border-right: 1px solid var(--line); display: flex; flex-direction: column; overflow-y: auto;">
            <div class="compare-header" style="padding: 8px; text-align: center; font-size: 0.8rem; font-weight: 800; background: var(--surface-soft); border-bottom: 1px solid var(--line); display: flex; align-items: center; justify-content: center; gap: 8px;">
              BASE TEMPLATE
              <RotateCw v-if="!basePdfUrl && isCompilingBase" :size="14" class="spinner" />
            </div>
            <VuePdfEmbed v-if="basePdfUrl" :source="basePdfUrl" class="pdf-embed-component" />
          </div>

          <!-- Code Editor -->
          <div v-show="!isComparing" class="editor-container" ref="editorContainer">
            <!-- AI Loading Overlay (Scoped to the editor so it doesn't block tabs) -->
            <AnimatePresence>
              <Motion
                v-if="isGenerating || isFixing"
                :initial="{ opacity: 0 }"
                :animate="{ opacity: 1 }"
                :exit="{ opacity: 0 }"
                class="loading-overlay"
              >
                <div class="loader-content">
                  <RotateCw :size="32" class="spinner" />
                  <h3>{{ isGenerating ? (activeMode === 'resume' ? 'TAILORING RESUME...' : 'CRAFTING COVER LETTER...') : 'DEBUGGING...' }}</h3>
                </div>
              </Motion>
            </AnimatePresence>
            <codemirror
              v-if="activeMode === 'resume'"
              v-model="resumeLatex"
              placeholder="Tailored Resume LaTeX content will appear here..."
              :style="{ height: '100%' }"
              :autofocus="true"
              :indent-with-tab="true"
              :tab-size="2"
              :extensions="extensions"
              class="latex-editor-cm"
            />
            <codemirror
              v-else
              v-model="clLatex"
              placeholder="Tailored Cover Letter LaTeX content will appear here..."
              :style="{ height: '100%' }"
              :autofocus="true"
              :indent-with-tab="true"
              :tab-size="2"
              :extensions="extensions"
              class="latex-editor-cm"
            />
          </div>

          <AnimatePresence>
            <Motion 
              v-if="activeLatex && (isComparing || activePdfUrl)"
              class="refinement-bar"
              drag
              :drag-constraints="splitPaneRef || undefined"
              :drag-elastic="0.1"
              :initial="{ opacity: 0, y: -10, x: '-50%' }"
              :animate="{ opacity: 1, y: 0, x: '-50%' }"
              :exit="{ opacity: 0, y: -10, x: '-50%' }"
              style="z-index: 100; position: absolute; left: 50%; top: 20px;"
            >
              <input 
                v-model="refinementInstruction" 
                :placeholder="`Refine tailored ${activeMode === 'resume' ? 'resume' : 'cover letter'}...`"
                @keyup.enter="refineWithAi"
              />
              <button @click="refineWithAi" :disabled="isRefining">
                <Loader2 v-if="isRefining" :size="14" class="spinner" />
                <span v-else>→</span>
              </button>
            </Motion>
          </AnimatePresence>

          <div v-if="activePdfUrl && (activeMode === 'resume' ? isResumeCompiled : isClCompiled) && !isComparing" class="preview-resizer" @mousedown="startResizingPreview"></div>

          <div v-if="activePdfUrl && (activeMode === 'resume' ? isResumeCompiled : isClCompiled)" class="pdf-viewer tailored-pdf-viewer" :style="isComparing ? { flex: 1, width: 'auto' } : { width: previewWidth + 'px', flex: 'none' }">
            <div v-if="isComparing" class="compare-header" style="padding: 8px; text-align: center; font-size: 0.8rem; font-weight: 800; background: var(--surface-soft); border-bottom: 1px solid var(--line); color: var(--accent);">TAILORED VERSION</div>
            <VuePdfEmbed :key="isComparing.toString()" :source="activePdfUrl" class="pdf-embed-component" @error="onPdfError" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg);
}

.workspace-header {
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
}

.header-left { display: flex; align-items: center; gap: 12px; }
.back-btn { background: none; border: none; color: var(--muted); cursor: pointer; font-size: 1.2rem; padding: 0 4px; }
.back-btn:hover { color: var(--ink); }

.job-info { display: flex; align-items: center; gap: 8px; }
.title { font-size: 0.8rem; font-weight: 600; color: var(--ink); margin: 0; }
.company { font-size: 0.8rem; color: var(--muted); }
.link-btn { background: none; border: none; cursor: pointer; padding: 2px; font-size: 0.8rem; opacity: 0.7; }
.link-btn:hover { opacity: 1; }

.header-actions { display: flex; gap: 8px; }
.delete-btn { background: none; border: none; color: var(--warning); font-size: 0.7rem; font-weight: 600; cursor: pointer; text-transform: uppercase; }

.error-banner {
  background: var(--warning);
  color: #fff;
  padding: 4px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.75rem;
  z-index: 10;
}
.error-banner button { background: none; border: none; color: #fff; cursor: pointer; }

.split-view {
  flex: 1;
  display: flex;
  min-height: 0;
}

.panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.section-header-icon {
  color: var(--accent);
  margin-bottom: 12px;
  display: flex;
  position: relative;
  cursor: help;
}

.meta-icon-wrapper {
  color: var(--muted);
  display: flex;
  align-items: center;
  position: relative;
  cursor: help;
}

.sidebar-tooltip {
  left: 100%;
  top: 50%;
  bottom: auto;
  transform: translateY(-50%);
  margin-left: 12px;
  z-index: 2000;
}

.sidebar-tooltip::after {
  top: 50%;
  right: 100%;
  left: auto;
  bottom: auto;
  transform: translateY(-50%);
  border-top-color: transparent;
  border-right-color: var(--accent);
}

.info-panel {
  width: 260px;
  background: var(--bg-accent);
  border-right: 1px solid var(--line);
  padding: 12px;
  gap: 20px;
  overflow-y: auto;
}

.section h3 {
  font-size: 0.65rem;
  text-transform: uppercase;
  color: var(--muted);
  letter-spacing: 0.05em;
  margin: 0 0 8px 0;
}

.meta-grid {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 6px;
  font-size: 0.75rem;
}
.meta-grid .label { color: var(--muted); }
.meta-grid .value { color: var(--ink); font-weight: 500; }

.tight-list {
  padding-left: 12px;
  margin: 0;
  font-size: 0.75rem;
  color: var(--ink);
  opacity: 0.85;
}
.tight-list li { margin-bottom: 4px; }

.form-group { margin-bottom: 12px; }
.form-group label { display: block; font-size: 0.65rem; color: var(--muted); margin-bottom: 4px; }

.compact-select, .compact-textarea {
  width: 100%;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  color: var(--ink);
  font-size: 0.75rem;
  padding: 6px;
  outline: none;
}

.status-select {
  font-weight: 700;
  color: var(--accent);
}

.status-select option {
  background: var(--bg-accent);
  color: var(--ink);
  font-weight: normal;
}

.milestones-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

.milestone-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.7rem;
}

.milestone-label {
  color: var(--muted);
  font-weight: 600;
  text-transform: uppercase;
}

.milestone-value {
  color: var(--accent);
  font-weight: 700;
}

.salary-badge {
  background: var(--accent-soft);
  color: var(--accent);
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 800;
}

.edit-salary-btn {
  background: none;
  border: 1px dashed var(--line);
  color: var(--muted);
  font-size: 0.65rem;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
}

.edit-salary-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.compact-textarea { height: 60px; resize: none; }

.btn-accent {
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: var(--radius-sm);
  padding: 6px;
  font-weight: 600;
  font-size: 0.75rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
.btn-accent:disabled { opacity: 0.5; }

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

.tab-tooltip { bottom: 140%; left: 50%; }
.info-tooltip { bottom: 140%; left: 50%; }

.right-tabs .tab-tooltip {
  left: auto;
  right: 0;
  transform: none;
}

.right-tabs .tab-tooltip::after {
  left: auto;
  right: 14px;
  transform: none;
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(13, 17, 23, 0.9);
  backdrop-filter: blur(4px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.loader-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.loader-content h3 {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--accent);
  letter-spacing: 0.1em;
  margin: 0;
}

.main-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.panel-tabs {
  height: 32px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--bg-accent);
  padding: 0 4px;
  border-bottom: 1px solid var(--line);
}

.left-tabs, .right-tabs { display: flex; align-items: center; }

.tab {
  height: 32px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--ink);
  background: var(--bg);
  border: none;
  border-top: 1px solid var(--accent);
}

.tab-btn-mode {
  height: 32px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.65rem;
  font-weight: 800;
  color: var(--muted);
  background: none;
  border: none;
  cursor: pointer;
  transition: 0.2s;
  letter-spacing: 0.05em;
  position: relative;
}

.tab-btn-mode.active {
  color: var(--ink);
  background: var(--bg);
}

.tab-btn-mode.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: var(--accent);
}

.tab-btn-mode:hover:not(.active) {
  color: var(--ink);
  background: var(--surface-soft);
}

.divider {
  width: 1px;
  height: 16px;
  background: var(--line);
  margin: 0 8px;
}

.tab-btn {
  padding: 0 10px;
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--muted);
  background: none;
  border: none;
  cursor: pointer;
  transition: 0.15s;
}
.tab-btn:hover { color: var(--ink); }

.accent-btn { color: var(--accent); }
.ai-btn { color: #a371f7; }

.error-log {
  background: #1e1e1e;
  border-bottom: 1px solid var(--warning);
  max-height: 200px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.error-log header {
  padding: 4px 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.65rem;
  color: var(--warning);
  font-weight: 700;
}
.error-log header button { background: none; border: none; color: var(--muted); cursor: pointer; }
.error-log pre {
  margin: 0;
  padding: 8px 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.7rem;
  color: #f85149;
  overflow: auto;
}

.editor-container {
  flex: 1;
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 200px;
  background: #282c34; /* One Dark background */
}

.split-pane {
  display: flex;
  flex-direction: row;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.split-pane.is-resizing {
  cursor: col-resize;
  user-select: none;
}

.preview-resizer {
  width: 4px;
  background: var(--bg-accent);
  cursor: col-resize;
  transition: background 0.2s;
  z-index: 10;
  border-left: 1px solid var(--line);
  border-right: 1px solid var(--line);
}

.preview-resizer:hover, .preview-resizer:active {
  background: var(--accent);
}

.latex-editor-cm {
  flex: 1;
  width: 100%;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 0.85rem;
}

:deep(.cm-editor) {
  height: 100%;
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

.refinement-bar {
  position: absolute;
  top: 16px;
  left: 50%;
  width: 440px;
  background: var(--surface-soft);
  border: 1px solid var(--accent-soft);
  border-radius: 20px;
  display: flex;
  padding: 4px 14px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.5);
  z-index: 20;
  cursor: grab;
  touch-action: none;
}

.refinement-bar:active {
  cursor: grabbing;
}

.refinement-bar input {
  flex: 1;
  background: none;
  border: none;
  color: var(--ink);
  font-size: 0.8rem;
  padding: 6px 0;
  outline: none;
}

.refinement-bar button {
  background: none;
  border: none;
  color: var(--accent);
  font-size: 1rem;
  cursor: pointer;
}

.pdf-viewer {
  flex: 1;
  display: block;
  background: var(--bg);
  position: relative;
  overflow: auto; /* Enable scrolling for vue-pdf-embed */
}

.pdf-embed-component {
  width: 100%;
  height: 100%;
  display: block;
  background: white;
}

.w-full { width: 100%; }

@media (max-width: 960px) {
  .info-panel { display: none; }
  .workspace-header { height: 44px; }
}
</style>
