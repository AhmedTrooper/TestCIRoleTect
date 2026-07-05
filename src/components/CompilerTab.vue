<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open as openDialog } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { join } from '@tauri-apps/api/path';
import { 
  writeFile, 
  readDir, 
  readTextFile, 
  mkdir, 
  remove, 
  exists,
  rename
} from '@tauri-apps/plugin-fs';
import { Motion, AnimatePresence } from 'motion-v';
import { useSettingsStore } from '../store/settings';
import { useDialogStore } from '../store/dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { 
  Hammer, 
  Download, 
  Wand2, 
  RotateCw, 
  Loader2,
  X,
  FileCode,
  Terminal,
  FolderOpen,
  Plus,
  FolderPlus,
  Files,
  Save,
  Layout,
  BookOpen,
  Copy,
  Check,
  PanelRight,
  FileUp,
  ExternalLink,
  Zap
} from '@lucide/vue';

import { Codemirror } from 'vue-codemirror';
import VuePdfEmbed from 'vue-pdf-embed';
import { latex, latexLanguage, autoCloseTags } from 'codemirror-lang-latex';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';
import FileTreeItem from './FileTreeItem.vue';

const settingsStore = useSettingsStore();
const dialog = useDialogStore();

// Codemirror Extensions
const extensions = [
  latex(),
  latexLanguage,
  ...autoCloseTags,
  oneDark,
  EditorView.lineWrapping
];

// Types
interface FileItem {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileItem[];
  isOpen?: boolean;
}

// State
const workspacePath = ref<string | null>(null);

const workspaceName = computed(() => {
  if (!workspacePath.value) return '';
  const parts = workspacePath.value.split(/[/\\]/);
  return parts.filter(p => p).pop()?.toUpperCase() || 'EXPLORER';
});
const mainFilePath = ref<string | null>(null);
const fileTree = ref<FileItem[]>([]);
const activeFilePath = ref<string | null>(null);
const latexCode = ref('');

const isSidebarVisible = ref(true);
const sidebarWidth = ref(240);
const isResizing = ref(false);

const isPreviewVisible = ref(true);
const previewWidth = ref(500);
const isResizingPreview = ref(false);
const splitPaneRef = ref<HTMLElement | null>(null);
const fileTreeContainerRef = ref<HTMLElement | null>(null);
const compilerContainerRef = ref<HTMLElement | null>(null);

const pdfUrl = ref<any>(null);
const pdfBytesBuffer = ref<Uint8Array | null>(null);
const isCompiling = ref(false);
const isFixing = ref(false);
const isRefining = ref(false);
const refinementInstruction = ref('');
const isDownloading = ref(false);
const compilationError = ref<string | null>(null);
const isCopyingError = ref(false);
const isDirty = ref(false);
const isProgrammaticChange = ref(false);

const handleCopyError = async () => {
  if (!compilationError.value) return;
  isCopyingError.value = true;
  try {
    await writeText(compilationError.value);
    setTimeout(() => { isCopyingError.value = false; }, 2000);
  } catch (err) {
    console.error('Failed to copy error:', err);
    isCopyingError.value = false;
  }
};
const editorContainer = ref<HTMLElement | null>(null);
const isLoadingWorkspace = ref(false);

const isTemplatesVisible = ref(false);
const activeTooltip = ref<string | null>(null);

const resumeTemplates = [
  {
    name: 'Initial Cacher',
    description: 'Run this to download and cache 85+ major scientific packages.',
    content: `\\documentclass[12pt, a4paper]{article}

% ==========================================
% 1. CORE ENGINE & TYPOGRAPHY
% ==========================================
\\usepackage[utf8]{inputenc}
\\usepackage[T1]{fontenc}
\\usepackage[english]{babel}
\\usepackage{lmodern}
\\usepackage{microtype}
\\usepackage{setspace}
\\usepackage{xparse}
\\usepackage{etoolbox}

% ==========================================
% 2. MATHEMATICS & THEOREMS
% ==========================================
\\usepackage{amsmath}
\\usepackage{amssymb}
\\usepackage{amsfonts}
\\usepackage{amsthm}
\\usepackage{mathtools}
\\usepackage{bm}
\\usepackage{esint}
\\usepackage{cancel}
\\usepackage{mathrsfs}
\\usepackage{thmtools}

% ==========================================
% 3. PHYSICS, CHEMISTRY, & ENGINEERING
% ==========================================
\\usepackage{siunitx}
\\usepackage{physics}
\\usepackage{mhchem}
\\usepackage{chemfig}
\\usepackage{circuitikz}
\\usepackage{bohr}
\\usepackage{modiagram}

% ==========================================
% 4. COMPUTER SCIENCE & LOGIC
% ==========================================
\\usepackage{listings}
\\usepackage{algorithm}
\\usepackage{algpseudocode}
\\usepackage{qtree}
\\usepackage{bussproofs}
\\usepackage{bytefield}

% ==========================================
% 5. GRAPHICS, PLOTS, & DRAWING
% ==========================================
\\usepackage{graphicx}
\\usepackage{xcolor}
\\usepackage{tikz}
\\usetikzlibrary{shapes, arrows, positioning, calc}
\\usepackage{pgfplots}
\\pgfplotsset{compat=1.18}
\\usepackage{float}
\\usepackage{wrapfig}
\\usepackage{caption}
\\usepackage{subcaption}
\\usepackage{adjustbox}
\\usepackage{transparent}

% ==========================================
% 6. TABLES & DATA HANDLING
% ==========================================
\\usepackage{booktabs}
\\usepackage{multirow}
\\usepackage{tabularx}
\\usepackage{array}
\\usepackage{longtable}
\\usepackage{makecell}
\\usepackage{dcolumn}
\\usepackage{pdflscape}

% ==========================================
% 7. PAGE LAYOUT & FORMATTING
% ==========================================
\\usepackage{geometry}
\\usepackage{fancyhdr}
\\usepackage{enumitem}
\\usepackage{multicol}
\\usepackage{parskip}
\\usepackage{titlesec}
\\usepackage{tocloft}
\\usepackage{emptypage}
\\usepackage{anyfontsize}
\\usepackage{lettrine}

% ==========================================
% 8. UTILITIES, DRAFTS, & ANNOTATIONS
% ==========================================
\\usepackage{tcolorbox}
\\usepackage{pdfpages}
\\usepackage{lipsum}
\\usepackage{blindtext}
\\usepackage{csquotes}
\\usepackage{todonotes}
\\usepackage{appendix}
\\usepackage{epigraph}
\\usepackage{soul}
\\usepackage{lineno}
\\usepackage{gitinfo2}

% ==========================================
% 9. BIBLIOGRAPHY & REFERENCING
% ==========================================
% (Note: Using natbib as it is the most robust legacy standard. 
% If you use biblatex in the future, it will just download separately).
\\usepackage[numbers]{natbib} 
\\usepackage{url}

% ==========================================
% 10. HYPERLINKS & CROSS-REFERENCING
% (CRITICAL: These must be loaded last!)
% ==========================================
\\usepackage{hyperref}
\\usepackage{bookmark}
\\usepackage{cleveref} 

\\begin{document}

\\section*{The Ultimate Cache is Complete}
If you are looking at this PDF, congratulations! Your LaTeX distribution has successfully reached out to the servers and downloaded the underlying files for over 80 major scientific packages, plus all of their background dependencies. 

\\subsection*{What is now on your hard drive?}
\\begin{itemize}
    \\item \\textbf{Math \\& Physics:} Advanced symbols, integral formatting, vector notations, and SI unit alignment.
    \\item \\textbf{Chemistry:} Molecule drawing environments, orbital diagrams, and chemical equation parsers.
    \\item \\textbf{Engineering \\& CS:} Circuit drawing tools, algorithmic pseudocode environments, and code-block highlighters.
    \\item \\textbf{Formatting:} Professional table rules, landscape pages, sub-figures, and vector graphic generators (TikZ).
\\end{itemize}

\\vspace{1cm}
\\begin{center}
    \\textbf{Reminder:} Do not use this massive file as your actual writing template. Loading all 80+ packages into a standard 5-page homework assignment will make your compilation times terribly slow. Just load what you need from now on—they are cached and waiting for you!
\\end{center}

\\end{document}`
  },
  {
    name: 'Modern Minimal',
    description: 'A sleek, minimalist design with zero extra packages to download.',
    content: `\\documentclass{article}

\\begin{document}
Hello World!
\\end{document}`
  }
];

// Workspace Management
const toggleSidebar = () => {
  isSidebarVisible.value = !isSidebarVisible.value;
};

const startResizing = (_e: MouseEvent) => {
  isResizing.value = true;
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', stopResizing);
};

const handleMouseMove = (e: MouseEvent) => {
  if (!isResizing.value || !splitPaneRef.value) return;
  const rect = splitPaneRef.value.getBoundingClientRect();
  const newWidth = e.clientX - rect.left;
  if (newWidth < 100) {
    isSidebarVisible.value = false;
    stopResizing();
    sidebarWidth.value = 240;
    return;
  }
  sidebarWidth.value = Math.max(180, Math.min(500, newWidth));
};

const stopResizing = () => {
  isResizing.value = false;
  document.removeEventListener('mousemove', handleMouseMove);
  document.removeEventListener('mouseup', stopResizing);
};

const togglePreview = () => {
  isPreviewVisible.value = !isPreviewVisible.value;
};

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
    isPreviewVisible.value = false;
    stopResizingPreview();
    previewWidth.value = 450;
    return;
  }
  const currentSidebar = isSidebarVisible.value ? sidebarWidth.value : 0;
  const minWidth = 180;
  const maxWidth = rect.width - currentSidebar - 180;
  previewWidth.value = Math.max(minWidth, Math.min(maxWidth, newWidth));
};

const stopResizingPreview = () => {
  isResizingPreview.value = false;
  document.removeEventListener('mousemove', handlePreviewMouseMove);
  document.removeEventListener('mouseup', stopResizingPreview);
  
  // Force vue-pdf-embed to re-render to the new pane width
  nextTick(() => {
    window.dispatchEvent(new Event('resize'));
  });
};

const setMainFile = async (path: string) => {
  if (!workspacePath.value) return;
  
  // Toggle logic: if clicking the current main file, unset it
  if (mainFilePath.value === path) {
    mainFilePath.value = null;
    await invoke('save_setting', { key: `main_file:${workspacePath.value}`, value: '' });
  } else {
    mainFilePath.value = path;
    await invoke('save_setting', { key: `main_file:${workspacePath.value}`, value: path });
  }
};

const loadMainFile = async () => {
  if (!workspacePath.value) return;
  const saved = await invoke<string>('get_setting', { key: `main_file:${workspacePath.value}`, default_value: '' });
  if (saved && await exists(saved)) {
    mainFilePath.value = saved;
  } else {
    mainFilePath.value = null;
  }
};

const useTemplate = async (template: typeof resumeTemplates[0]) => {
  const confirmed = await dialog.showConfirm(`Overwrite current editor content with the "${template.name}" template?`, 'Use Template');
  if (confirmed) {
    latexCode.value = template.content;
    isTemplatesVisible.value = false;
    isDirty.value = true;
    await saveActiveFile();
  }
};

// Persistence & Initialization
onMounted(async () => {
  try {
    const savedWorkspace = await invoke<string | null>('get_workspace_path');
    if (savedWorkspace && await exists(savedWorkspace)) {
      workspacePath.value = savedWorkspace;
      await refreshFileTree();
      await loadMainFile();

      const lastFile = await invoke<string | null>('get_last_opened_file');
      if (lastFile && await exists(lastFile)) {
        await selectFile({ name: lastFile.split(/[/\\]/).pop() || '', path: lastFile, isDir: false });
      }
    } else {
      const savedCode = await invoke<string | null>('get_compiler_state');
      if (savedCode) {
        latexCode.value = savedCode;
      }
    }
    // Ensure initial load doesn't mark it as dirty
    setTimeout(() => { isDirty.value = false; }, 100);
  } catch (err) {
    console.error('Failed to initialize compiler:', err);
  }
});

onUnmounted(async () => {
  pdfUrl.value = null; // Reset on unmount
  if (isDirty.value && settingsStore.isAutoCompileEnabled) {
    await saveActiveFile();
  }
});

// Workspace Management
const selectWorkspace = async () => {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: 'Select LaTeX Workspace'
    });

    if (selected && typeof selected === 'string') {
      workspacePath.value = selected;
      
      // Clear previously opened standalone or cross-workspace file
      activeFilePath.value = null;
      isProgrammaticChange.value = true;
      latexCode.value = '';

      await invoke('save_workspace_path', { path: selected });
      await refreshFileTree();
      await loadMainFile();

      // Attempt to load main file automatically if one is set
      if (mainFilePath.value && await exists(mainFilePath.value)) {
        await selectFile({ name: mainFilePath.value.split(/[/\\]/).pop() || '', path: mainFilePath.value, isDir: false });
      }
    }
  } catch (err) {
    console.error('Failed to select workspace:', err);
  }
};

const openSingleFile = async () => {
  try {
    const selected = await openDialog({
      directory: false,
      multiple: false,
      title: 'Open LaTeX File',
      filters: [{ name: 'LaTeX File', extensions: ['tex'] }]
    });

    if (selected && typeof selected === 'string') {
      await selectFile({ name: selected.split(/[/\\]/).pop() || '', path: selected, isDir: false });
    }
  } catch (err) {
    console.error('Failed to open file:', err);
  }
};

const openWorkspaceInExplorer = async () => {
  if (workspacePath.value) {
    await revealItemInDir(workspacePath.value);
  }
};

const refreshFileTree = async () => {
  if (!workspacePath.value) return;
  const savedScrollLeft = fileTreeContainerRef.value?.scrollLeft || 0;
  const savedScrollTop = fileTreeContainerRef.value?.scrollTop || 0;
  try {
    // Recursively scan the directory while preserving open states
    fileTree.value = await scanDirectoryRecursive(workspacePath.value, fileTree.value);
    await nextTick();
    if (fileTreeContainerRef.value) {
      // Use setTimeout to ensure DOM has fully painted its new width/height before scrolling
      setTimeout(() => {
        if (fileTreeContainerRef.value) {
          fileTreeContainerRef.value.scrollLeft = savedScrollLeft;
          fileTreeContainerRef.value.scrollTop = savedScrollTop;
        }
      }, 50);
    }
  } catch (err) {
    console.error('Failed to scan workspace:', err);
  }
};

const scanDirectoryRecursive = async (dir: string, oldItems: FileItem[]): Promise<FileItem[]> => {
  const entries = await readDir(dir);
  const items: FileItem[] = [];

  for (const entry of entries) {
    const fullPath = await join(dir, entry.name);
    const isDir = entry.isDirectory;
    const oldItem = oldItems.find(i => i.path === fullPath);
    
    const item: FileItem = {
      name: entry.name,
      path: fullPath,
      isDir: isDir,
      isOpen: oldItem ? oldItem.isOpen : false,
      children: []
    };

    if (item.isDir && item.isOpen) {
      item.children = await scanDirectoryRecursive(fullPath, oldItem?.children || []);
    }

    items.push(item);
  }

  return items.sort((a, b) => {
    if (a.isDir && !b.isDir) return -1;
    if (!a.isDir && b.isDir) return 1;
    return a.name.localeCompare(b.name);
  });
};

const toggleFolder = async (item: FileItem) => {
  item.isOpen = !item.isOpen;
  if (item.isOpen && (!item.children || item.children.length === 0)) {
    item.children = await scanDirectoryRecursive(item.path, []);
  }
};

const selectFile = async (item: FileItem) => {
  if (item.isDir) return;
  
  if (isDirty.value && activeFilePath.value) {
    await saveActiveFile();
  }

  try {
    // Bulletproof: Check existence before reading
    const fileExists = await exists(item.path);
    if (!fileExists) {
      await dialog.showAlert(`The file "${item.name}" no longer exists on disk.`, 'File Not Found');
      await refreshFileTree();
      return;
    }

    const content = await readTextFile(item.path);
    isProgrammaticChange.value = true;
    latexCode.value = content;
    activeFilePath.value = item.path;
    isDirty.value = false;
    await invoke('save_last_opened_file', { path: item.path });
    pdfUrl.value = null; // Reset preview for new file
  } catch (err: any) {
    console.error('Failed to read file:', err);
    await dialog.showAlert(`Failed to open file: ${err.message || err.toString()}`, 'Read Error');
  }
};

const saveActiveFile = async () => {
  if (!activeFilePath.value) {
    // Fallback to standalone state
    await invoke('save_compiler_state', { latexContent: latexCode.value });
    return;
  }

  try {
    // Bulletproof: Check if parent directory still exists
    const lastSlash = Math.max(activeFilePath.value.lastIndexOf('/'), activeFilePath.value.lastIndexOf('\\'));
    const dirPath = lastSlash !== -1 ? activeFilePath.value.substring(0, lastSlash) : null;
    
    if (dirPath && !(await exists(dirPath))) {
      await dialog.showAlert("The parent directory for this file is missing.", "Save Failed");
      return;
    }

    await writeFile(activeFilePath.value, new TextEncoder().encode(latexCode.value));
    isDirty.value = false;
  } catch (err: any) {
    console.error('Failed to save file:', err);
    await dialog.showAlert(`Save failed: ${err.message || err.toString()}`, 'Write Error');
  }
};

const createNewFile = async (parent: FileItem | null = null) => {
  const dir = parent ? parent.path : workspacePath.value;
  if (!dir) return;

  let fileName = await dialog.showPrompt('Enter file name (e.g. main.tex):', '', 'New File');
  if (!fileName) return;

  // Auto-append .tex if no extension provided
  if (!fileName.includes('.')) {
    fileName += '.tex';
  }

  const fullPath = await join(dir, fileName);

  if (await exists(fullPath)) {
    await dialog.showAlert(`A file or folder named "${fileName}" already exists.`, 'Create Failed');
    return;
  }
  
  // Prevent fatal format file error by providing a minimal valid TeX document
  const isTex = fileName.endsWith('.tex');
  const initialContent = isTex 
    ? '\\documentclass{article}\n\\begin{document}\n\nStart writing here...\n\n\\end{document}' 
    : '';

  try {
    await writeFile(fullPath, new TextEncoder().encode(initialContent));
    
    // Always refresh the full tree to ensure UI is in sync
    await refreshFileTree();

    // Auto-select the newly created file
    await selectFile({ name: fileName, path: fullPath, isDir: false });

  } catch (err: any) {
    await dialog.showAlert(err.toString(), 'Failed to create file');
  }
};

const createNewFolder = async (parent: FileItem | null = null) => {
  const dir = parent ? parent.path : workspacePath.value;
  if (!dir) return;

  const folderName = await dialog.showPrompt('Enter folder name:', '', 'New Folder');
  if (!folderName) return;

  const fullPath = await join(dir, folderName);

  if (await exists(fullPath)) {
    await dialog.showAlert(`A file or folder named "${folderName}" already exists.`, 'Create Failed');
    return;
  }

  try {
    await mkdir(fullPath);
    // Always refresh the full tree
    await refreshFileTree();
  } catch (err: any) {
    await dialog.showAlert(err.toString(), 'Failed to create folder');
  }
};

const deleteItem = async (item: FileItem) => {
  const confirmed = await dialog.showConfirm(`Are you sure you want to delete "${item.name}"?`, 'Delete Item');
  if (!confirmed) return;

  try {
    await remove(item.path, { recursive: item.isDir });
    
    // Bulletproof: If the active file (or its parent directory) was deleted, clear the editor
    if (activeFilePath.value) {
      const isSelf = activeFilePath.value === item.path;
      const isParent = activeFilePath.value.startsWith(item.path + '/') || activeFilePath.value.startsWith(item.path + '\\');
      
      if (isSelf || isParent) {
        activeFilePath.value = null;
        latexCode.value = '';
        isDirty.value = false;
        pdfUrl.value = null;
      }
    }

    if (mainFilePath.value === item.path) {
      mainFilePath.value = null;
    }
    
    await refreshFileTree();
  } catch (err: any) {
    await dialog.showAlert(err.toString(), 'Failed to delete item');
  }
};

const renameItem = async (item: FileItem) => {
  const oldPath = item.path;
  const oldName = item.name;
  
  const newName = await dialog.showPrompt('Enter new name:', oldName, 'Rename');
  if (!newName || newName === oldName) return;

  // Resolve new path
  const lastSlash = Math.max(oldPath.lastIndexOf('/'), oldPath.lastIndexOf('\\'));
  const parentDir = lastSlash !== -1 ? oldPath.substring(0, lastSlash) : workspacePath.value;
  if (!parentDir) return;

  const newPath = await join(parentDir, newName);

  if (await exists(newPath)) {
    await dialog.showAlert(`A file or folder named "${newName}" already exists.`, 'Rename Failed');
    return;
  }

  try {
    await rename(oldPath, newPath);

    // Sync active and main file paths
    if (activeFilePath.value === oldPath) {
      activeFilePath.value = newPath;
      await invoke('save_last_opened_file', { path: newPath });
    } else if (activeFilePath.value && (activeFilePath.value.startsWith(oldPath + '/') || activeFilePath.value.startsWith(oldPath + '\\'))) {
      const rel = activeFilePath.value.substring(oldPath.length);
      activeFilePath.value = newPath + rel;
      await invoke('save_last_opened_file', { path: newPath + rel });
    }

    if (mainFilePath.value === oldPath) {
      mainFilePath.value = newPath;
      await invoke('save_setting', { key: `main_file:${workspacePath.value}`, value: newPath });
    } else if (mainFilePath.value && (mainFilePath.value.startsWith(oldPath + '/') || mainFilePath.value.startsWith(oldPath + '\\'))) {
      const rel = mainFilePath.value.substring(oldPath.length);
      mainFilePath.value = newPath + rel;
      await invoke('save_setting', { key: `main_file:${workspacePath.value}`, value: newPath + rel });
    }

    await refreshFileTree();
  } catch (err: any) {
    await dialog.showAlert(err.toString(), 'Failed to rename');
  }
};

const closeWorkspace = async () => {
  const confirmed = await dialog.showConfirm('Close workspace and return to standalone mode?', 'Close Workspace');
  if (!confirmed) return;

  workspacePath.value = null;
  fileTree.value = [];
  activeFilePath.value = null;
  mainFilePath.value = null;
  await invoke('save_workspace_path', { path: '' });
  
  const savedCode = await invoke<string | null>('get_compiler_state');
  if (savedCode) {
    latexCode.value = savedCode;
  }
};

// Auto-save & Compile logic
watch(latexCode, () => {
  if (isProgrammaticChange.value) {
    isProgrammaticChange.value = false;
    return;
  }
  isDirty.value = true;
});

const handleBlur = async () => {
  if (isDirty.value) {
    if (settingsStore.isAutoCompileEnabled) {
      // compilePdf internally calls and awaits saveActiveFile()
      await compilePdf();
    } else {
      await saveActiveFile();
    }
  }
};

// AI Refinement
const refineWithAi = async () => {
  if (!latexCode.value || !refinementInstruction.value.trim() || isRefining.value) return;
  
  isRefining.value = true;
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const refinedCode = await invoke<string>('refine_latex_with_ai', {
      provider,
      model,
      apiKey,
      currentLatex: latexCode.value,
      instruction: refinementInstruction.value.trim()
    });

    latexCode.value = refinedCode;
    refinementInstruction.value = '';
    await saveActiveFile();
    await compilePdf();
  } catch (err: any) {
    console.error("AI Refinement Error:", err);
    await dialog.showAlert(err.toString(), 'AI Refinement Failed');
  } finally {
    isRefining.value = false;
  }
};

// Compile PDF
const compilePdf = async () => {
  if (!latexCode.value.trim() && !mainFilePath.value) return;
  
  isCompiling.value = true;
  compilationError.value = null;
  
  try {
    // Bulletproof: Force save before compile so the disk is in sync with the editor
    await saveActiveFile();

    let pdfBytes: number[];
    
    let compileTarget = activeFilePath.value;
    if (compileTarget && !compileTarget.toLowerCase().endsWith('.tex')) {
      await dialog.showAlert(
        `The active file "${compileTarget.split(/[/\\\\]/).pop()}" is not a LaTeX (.tex) file. Only .tex files can be compiled.`,
        'Compilation Blocked'
      );
      return;
    }

    if (!compileTarget && workspacePath.value) {
      compileTarget = mainFilePath.value;
    }

    if (compileTarget) {
      let targetWorkspace = workspacePath.value;
      let relativePath = '';

      if (targetWorkspace && compileTarget.startsWith(targetWorkspace)) {
        // File is inside the current workspace
        relativePath = compileTarget.substring(targetWorkspace.length);
        if (relativePath.startsWith('/') || relativePath.startsWith('\\')) {
          relativePath = relativePath.substring(1);
        }
      } else {
        // File is outside the workspace OR no workspace is open
        // Use the file's parent directory as its isolated workspace context
        const lastSlash = Math.max(compileTarget.lastIndexOf('/'), compileTarget.lastIndexOf('\\'));
        targetWorkspace = compileTarget.substring(0, lastSlash);
        relativePath = compileTarget.substring(lastSlash + 1);
      }

      pdfBytes = await invoke<number[]>('compile_workspace_to_pdf', { 
        workspaceDir: targetWorkspace,
        mainFileName: relativePath,
        filename: 'latex_workspace_roletect.pdf'
      });
      
      if (workspacePath.value) {
        await refreshFileTree();
        const port = await invoke<string>('get_setting', { key: 'active_server_port', default_value: '1420' });
        pdfUrl.value = {
          url: `http://127.0.0.1:${port}/static-pdf/latex_workspace_roletect.pdf?cache-bust=${Date.now()}`,
          disableRange: false,
          disableStream: false,
          rangeChunkSize: 1024 * 1024
        };
      }
    } else {
      // True standalone string compilation (scratchpad mode)
      if (!latexCode.value.trim()) {
        throw new Error("No .tex file selected and no content to compile.");
      }
      pdfBytes = await invoke<number[]>('compile_resume_to_pdf', { 
        latexCode: latexCode.value,
        filename: 'latex_workspace_roletect.pdf'
      });
    }
    
    pdfBytesBuffer.value = new Uint8Array(pdfBytes);
    
    // Fetch port from DB
    const port = await invoke<string>('get_setting', { key: 'active_server_port', default_value: '1420' });
    
    // Pass configuration object to vue-pdf-embed for chunking
    pdfUrl.value = {
      url: `http://127.0.0.1:${port}/static-pdf/latex_workspace_roletect.pdf?cache-bust=${Date.now()}`,
      disableRange: false,
      disableStream: false,
      rangeChunkSize: 1024 * 1024 // 1MB chunks
    };
    
    compilationError.value = null;
  } catch (err: any) {
    console.error("Compilation Error:", err);
    compilationError.value = err.message || err.toString();
  } finally {
    isCompiling.value = false;
  }
};

const onPdfError = (err: any) => {
  console.error("PDF Rendering Error:", err);
  compilationError.value = "Frontend Rendering Error: Failed to stream or parse PDF chunks from the backend. " + (err.message || err.toString());
};

// AI Fix
const fixWithAi = async () => {
  if (!latexCode.value || !compilationError.value || isFixing.value) return;
  
  isFixing.value = true;
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const fixedCode = await invoke<string>('fix_latex_with_ai', {
      provider,
      model,
      apiKey,
      brokenLatex: latexCode.value,
      errorLogs: compilationError.value
    });

    latexCode.value = fixedCode;
    compilationError.value = null;
    await saveActiveFile();
    await compilePdf();
  } catch (err: any) {
    console.error("AI Fix Error:", err);
    await dialog.showAlert(err.toString(), 'AI Fix Failed');
  } finally {
    isFixing.value = false;
  }
};

// Download PDF
const downloadPdf = async () => {
  if (!pdfBytesBuffer.value) return;
  isDownloading.value = true;
  
  try {
    const now = new Date();
    const timestamp = `${now.getFullYear()}${(now.getMonth() + 1).toString().padStart(2, '0')}${now.getDate().toString().padStart(2, '0')}_${now.getHours().toString().padStart(2, '0')}${now.getMinutes().toString().padStart(2, '0')}${now.getSeconds().toString().padStart(2, '0')}`;
    const defaultName = activeFilePath.value 
      ? activeFilePath.value.split(/[/\\]/).pop()?.replace('.tex', '.pdf') || `doc_${timestamp}.pdf`
      : `document_${timestamp}.pdf`;

    const filePath = await save({
      filters: [{ name: 'PDF Document', extensions: ['pdf'] }],
      defaultPath: defaultName
    });

    if (filePath) {
      await writeFile(filePath, pdfBytesBuffer.value);
      const filename = filePath.split(/[/\\]/).pop() || defaultName;
      await invoke('record_download', {
        filename,
        downloadType: 'compiler',
        jobId: null,
        contentId: null
      });
      await dialog.showAlert('PDF downloaded successfully.', 'Success');
    }
  } catch (err: any) {
    console.error("Download Error:", err);
    await dialog.showAlert(err.toString(), 'Download Failed');
  } finally {
    isDownloading.value = false;
  }
};

const activeFileName = computed(() => {
  if (!activeFilePath.value) return 'unsaved.tex';
  return activeFilePath.value.split(/[/\\]/).pop() || 'file.tex';
});
</script>

<template>
  <div class="compiler-container" ref="compilerContainerRef">
    <header class="compiler-header">
      <div class="header-left">
        <button class="toggle-sidebar-btn" @click="toggleSidebar" title="Toggle Sidebar">
          <Layout :size="18" />
        </button>
        <button class="toggle-sidebar-btn" @click="togglePreview" title="Toggle PDF Preview">
          <PanelRight :size="18" />
        </button>
        <Files :size="20" class="header-icon" />
        <h1>LaTeX IDE</h1>
        <span v-if="workspacePath" class="workspace-label">
          {{ workspacePath.split(/[/\\]/).pop() }}
        </span>
      </div>
      
      <div class="header-actions">
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'templates'" @mouseleave="activeTooltip = null">
          <button class="action-btn" @click="isTemplatesVisible = true">
            <BookOpen :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'templates'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Gallery
            </Motion>
          </AnimatePresence>
        </div>

        <div class="divider-v"></div>

        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'auto-compile'" @mouseleave="activeTooltip = null">
          <button 
            class="action-btn auto-compile-btn" 
            :class="{ active: settingsStore.isAutoCompileEnabled }"
            @click="settingsStore.setAutoCompile(!settingsStore.isAutoCompileEnabled)"
          >
            <Zap :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'auto-compile'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              {{ settingsStore.isAutoCompileEnabled ? 'Auto Compile: Enabled' : 'Auto Compile: Disabled' }}
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'save'" @mouseleave="activeTooltip = null">
          <button 
            v-if="isDirty"
            class="action-btn save-btn"
            @click="saveActiveFile"
          >
            <Save :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'save' && isDirty"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Save Changes
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'ai-fix'" @mouseleave="activeTooltip = null">
          <button 
            v-if="compilationError" 
            class="action-btn ai-btn" 
            @click="fixWithAi" 
            :disabled="isFixing"
          >
            <Wand2 :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'ai-fix' && compilationError"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Fix with AI
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'compile'" @mouseleave="activeTooltip = null">
          <button 
            class="action-btn compile-btn" 
            @click="compilePdf" 
            :disabled="isCompiling || !latexCode"
          >
            <Hammer :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'compile'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Compile LaTeX
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'download'" @mouseleave="activeTooltip = null">
          <button 
            v-if="pdfUrl" 
            class="action-btn download-btn" 
            @click="downloadPdf" 
            :disabled="isDownloading"
          >
            <Download :size="16" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'download' && pdfUrl"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Download PDF
            </Motion>
          </AnimatePresence>
        </div>
      </div>
    </header>

    <main class="compiler-main">
      <div class="split-pane" ref="splitPaneRef" :class="{ 'is-resizing': isResizing || isResizingPreview }">
        <!-- Sidebar File Explorer -->
        <aside v-if="isSidebarVisible" class="workspace-sidebar" :style="{ width: sidebarWidth + 'px' }">
          <div class="sidebar-header">
            <div class="sidebar-header-top" :title="workspacePath || 'Workspace'">
              <div class="workspace-name-row">
                <FolderOpen :size="14" class="workspace-folder-icon" />
                <span class="workspace-title">{{ workspaceName || 'EXPLORER' }}</span>
                <button v-if="workspacePath" @click="closeWorkspace" title="Close Workspace" class="close-workspace-btn"><X :size="14" /></button>
              </div>
              <span v-if="workspacePath" class="workspace-path-subtext">{{ workspacePath }}</span>
            </div>
            <div class="sidebar-header-tools">
              <button class="header-tool-btn" @click="openSingleFile" title="Open File..."><FileUp :size="14" /></button>
              <button class="header-tool-btn" @click="selectWorkspace" title="Open / Switch Folder..."><FolderOpen :size="14" /></button>
              <button v-if="workspacePath" class="header-tool-btn" @click="openWorkspaceInExplorer" title="Reveal in System Explorer"><ExternalLink :size="14" /></button>
              <button class="header-tool-btn" @click="refreshFileTree" title="Refresh"><RotateCw :size="14" /></button>
              <button class="header-tool-btn" @click="createNewFile()" title="New File"><Plus :size="14" /></button>
              <button class="header-tool-btn" @click="createNewFolder()" title="New Folder"><FolderPlus :size="14" /></button>
            </div>
          </div>

          <div v-if="!workspacePath" class="sidebar-empty">
            <FolderOpen :size="32" />
            <p>No workspace selected</p>
            <div class="empty-actions">
              <button class="btn-primary-sm" @click="selectWorkspace">Open Folder</button>
              <button class="btn-secondary-sm" @click="openSingleFile">Open File</button>
            </div>
          </div>

          <div v-else class="file-tree" ref="fileTreeContainerRef">
            <div v-if="isLoadingWorkspace" class="tree-loading">
              <RotateCw :size="16" class="spinner" />
            </div>
            
            <template v-else>
              <!-- Recursive File Tree -->
              <FileTreeItem 
                v-for="item in fileTree" 
                :key="item.path"
                :item="item"
                :active-file-path="activeFilePath"
                :main-file-path="mainFilePath"
                :on-toggle="toggleFolder"
                :on-select="selectFile"
                :on-set-main="setMainFile"
                :on-create-file="createNewFile"
                :on-create-folder="createNewFolder"
                :on-delete="deleteItem"
                :on-rename="renameItem"
              />
            </template>
          </div>
        </aside>

        <!-- Sidebar Resizer -->
        <div v-if="isSidebarVisible" class="sidebar-resizer" @mousedown="startResizing"></div>

        <!-- Editor Section -->
        <section class="editor-section">
          <div class="pane-header">
            <div class="pane-header-left">
              <FileCode :size="14" />
              <span>{{ activeFileName }}</span>
              <span v-if="isDirty" class="dirty-indicator">●</span>
            </div>
            <div class="pane-header-actions" v-if="activeFilePath">
              <button 
                @click="saveActiveFile" 
                class="save-icon-btn" 
                :class="{ 'dirty': isDirty }" 
                title="Save Changes"
              >
                <Save :size="14" />
              </button>
            </div>
          </div>
          <div class="editor-relative-wrapper" ref="editorContainer">
            <codemirror
              v-model="latexCode"
              placeholder="Select a file or start typing here..."
              :style="{ height: '100%' }"
              :autofocus="true"
              :indent-with-tab="true"
              :tab-size="2"
              :extensions="extensions"
              @blur="handleBlur"
              class="latex-editor-cm"
            />
          </div>
        </section>

        <!-- Preview Resizer -->
        <div v-if="isPreviewVisible" class="preview-resizer" @mousedown="startResizingPreview"></div>

        <!-- Preview Section -->
        <section v-if="isPreviewVisible" class="preview-section" :style="{ width: previewWidth + 'px', flex: 'none' }">
          <!-- Loading Overlay (Scoped to Preview) -->
          <AnimatePresence>
            <Motion
              v-if="isCompiling || isFixing"
              :initial="{ opacity: 0 }"
              :animate="{ opacity: 1 }"
              :exit="{ opacity: 0 }"
              class="loading-overlay"
            >
              <div class="loader-content">
                <RotateCw :size="32" class="spinner" />
                <h3>{{ isFixing ? 'DEBUGGING...' : 'COMPILING...' }}</h3>
              </div>
            </Motion>
          </AnimatePresence>

          <div class="pane-header">
            <Terminal :size="14" />
            <span>PDF PREVIEW</span>
          </div>
          <div v-if="pdfUrl" class="pdf-viewer">
            <VuePdfEmbed :source="pdfUrl" class="pdf-embed-component" @error="onPdfError" />
          </div>
          <div v-else class="empty-preview">
            <div class="placeholder-content">
              <Hammer :size="48" />
              <h3>No PDF generated</h3>
              <p>Click "Compile" to generate a preview of your LaTeX code.</p>
            </div>
          </div>
        </section>
      </div>

      <!-- Error Console -->
      <AnimatePresence>
        <Motion
          v-if="compilationError"
          :initial="{ y: 100, opacity: 0 }"
          :animate="{ y: 0, opacity: 1 }"
          :exit="{ y: 100, opacity: 0 }"
          class="error-console"
        >
          <div class="console-header">
            <div class="title">
              <X :size="14" class="error-icon" />
              <span>COMPILATION ERROR</span>
            </div>
            <div class="console-actions">
              <button class="action-btn-inline" @click="handleCopyError" :title="isCopyingError ? 'Copied!' : 'Copy to Clipboard'">
                <Check v-if="isCopyingError" :size="14" class="success-icon" />
                <Copy v-else :size="14" />
              </button>
              <button class="action-btn-inline close-btn" @click="compilationError = null">
                <X :size="14" />
              </button>
            </div>
          </div>
          <div class="error-logs-container">
            <pre class="error-logs">{{ compilationError }}</pre>
          </div>
        </Motion>
      </AnimatePresence>
    </main>

    <!-- Floating Refinement AI Bar (Draggable across entire workspace) -->
    <AnimatePresence>
      <Motion 
        v-if="latexCode"
        class="refinement-bar"
        drag
        :drag-constraints="compilerContainerRef || undefined"
        :drag-elastic="0.05"
        :initial="{ opacity: 0, y: -10, x: '-50%' }"
        :animate="{ opacity: 1, y: 0, x: '-50%' }"
        :exit="{ opacity: 0, y: -10, x: '-50%' }"
      >
        <input 
          v-model="refinementInstruction" 
          placeholder="Refine code (e.g. 'Add a table of contents')..."
          @keyup.enter="refineWithAi"
        />
        <button @click="refineWithAi" :disabled="isRefining">
          <Loader2 v-if="isRefining" :size="14" class="spinner" />
          <span v-else>→</span>
        </button>
      </Motion>
    </AnimatePresence>

    <!-- Template Modal -->
    <AnimatePresence>
      <Motion
        v-if="isTemplatesVisible"
        :initial="{ opacity: 0 }"
        :animate="{ opacity: 1 }"
        :exit="{ opacity: 0 }"
        class="modal-backdrop"
        @click="isTemplatesVisible = false"
      >
        <Motion
          :initial="{ scale: 0.9, opacity: 0 }"
          :animate="{ scale: 1, opacity: 1 }"
          :exit="{ scale: 0.9, opacity: 0 }"
          class="template-modal"
          @click.stop
        >
          <div class="modal-header">
            <h3>Resume Templates</h3>
            <button @click="isTemplatesVisible = false"><X :size="18" /></button>
          </div>
          <div class="template-grid">
            <div v-for="temp in resumeTemplates" :key="temp.name" class="template-card" @click="useTemplate(temp)">
              <div class="temp-icon"><FileCode :size="32" /></div>
              <h4>{{ temp.name }}</h4>
              <p>{{ temp.description }}</p>
            </div>
          </div>
        </Motion>
      </Motion>
    </AnimatePresence>
  </div>
</template>

<style scoped>
.compiler-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg);
}

.compiler-header {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toggle-sidebar-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
  transition: 0.15s;
}

.toggle-sidebar-btn:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.header-icon {
  color: var(--accent);
}

.header-left h1 {
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--ink);
  margin: 0;
  letter-spacing: 0.02em;
}

.workspace-label {
  font-size: 0.7rem;
  background: var(--surface-soft);
  color: var(--muted);
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.divider-v {
  width: 1px;
  height: 20px;
  background: var(--line);
  margin: 0 4px;
}

.auto-compile-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  cursor: pointer;
  transition: 0.2s;
  border: 1px solid var(--line);
  background: var(--surface-soft);
  color: var(--ink);
}

.action-btn:hover:not(:disabled) {
  border-color: var(--muted);
  background: var(--surface);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.compile-btn {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.compile-btn:hover:not(:disabled) {
  opacity: 0.9;
  background: var(--accent);
}

.save-btn {
  border-color: var(--accent-soft);
  background: var(--accent-soft);
  color: var(--accent);
}

.save-btn:hover:not(:disabled) {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.ai-btn {
  color: #a371f7;
  border-color: rgba(163, 113, 247, 0.3);
}

.ai-btn:hover:not(:disabled) {
  background: rgba(163, 113, 247, 0.1);
  border-color: #a371f7;
}

.btn-tooltip-wrapper {
  position: relative;
  display: flex;
  align-items: center;
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

.compiler-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.split-pane {
  flex: 1;
  display: flex;
  min-height: 0;
  position: relative;
}

.split-pane.is-resizing object {
  pointer-events: none !important;
}

.split-pane.is-resizing {
  user-select: none !important;
  cursor: col-resize !important;
}

.workspace-sidebar {
  background: var(--bg-accent);
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  min-width: 180px;
  max-width: 500px;
}

.sidebar-resizer, .preview-resizer {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  transition: background 0.2s;
  z-index: 10;
  margin-left: -2px;
  margin-right: -2px;
}

.sidebar-resizer:hover, .sidebar-resizer:active,
.preview-resizer:hover, .preview-resizer:active {
  background: var(--accent);
}

.sidebar-header {
  display: flex;
  flex-direction: column;
  background: var(--surface);
  border-bottom: 1px solid var(--line);
  padding: 8px 10px 6px 10px;
  gap: 6px;
}

.sidebar-header-top {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.workspace-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

.workspace-folder-icon {
  color: var(--accent);
  flex-shrink: 0;
}

.workspace-title {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 0.75rem;
  font-weight: 800;
  color: var(--ink);
  letter-spacing: 0.03em;
}

.close-workspace-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.close-workspace-btn:hover {
  color: var(--warning) !important;
  background: rgba(248, 81, 73, 0.1);
}

.workspace-path-subtext {
  font-size: 0.6rem;
  font-weight: 500;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  opacity: 0.85;
}

.sidebar-header-tools {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 4px;
  padding-top: 4px;
  border-top: 1px solid var(--line-soft, rgba(255,255,255,0.05));
}

.header-tool-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.header-tool-btn:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.sidebar-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  gap: 12px;
  padding: 20px;
  text-align: center;
}

.sidebar-empty p {
  font-size: 0.75rem;
  margin: 0;
}

.empty-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.btn-primary-sm {
  background: var(--accent);
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}

.btn-secondary-sm {
  background: var(--surface-soft);
  color: var(--ink);
  border: 1px solid var(--line);
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
}

.btn-secondary-sm:hover {
  background: var(--surface);
}

.file-tree {
  flex: 1;
  overflow-y: auto;
  overflow-x: auto;
  padding: 8px 0;
}

.tree-item-wrapper {
  display: flex;
  flex-direction: column;
}

.tree-item {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  gap: 8px;
  cursor: pointer;
  transition: 0.1s;
  position: relative;
  user-select: none;
}

.tree-item:hover {
  background: var(--surface-soft);
}

.tree-item.active {
  background: var(--accent-soft);
  color: var(--accent);
}

.tree-item.main-file {
  font-weight: 700;
}

.tree-item.main-file .item-name {
  color: var(--accent);
}

.item-icon {
  display: flex;
  align-items: center;
  color: var(--muted);
}

.tree-item.active .item-icon {
  color: var(--accent);
}

.item-name {
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.1s;
}

.tree-item:hover .item-actions {
  opacity: 1;
}

.item-actions button {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
}

.item-actions button:hover {
  color: var(--ink);
}

.item-actions button.item-delete:hover {
  color: var(--warning);
}

.close-workspace-btn:hover {
  color: var(--warning) !important;
}

.tree-children {
  padding-left: 12px;
  overflow: hidden;
}

.tree-loading {
  display: flex;
  justify-content: center;
  padding: 20px;
  color: var(--accent);
}

.editor-section, .preview-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.preview-section {
  border-left: 1px solid var(--line);
}

.pane-header {
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
  font-size: 0.65rem;
  font-weight: 800;
  color: var(--muted);
  letter-spacing: 0.05em;
}

.pane-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.pane-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.save-icon-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.save-icon-btn:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.save-icon-btn.dirty {
  color: var(--accent);
}

.save-icon-btn.dirty:hover {
  background: var(--accent-soft);
}

.dirty-indicator {
  color: var(--accent);
  font-size: 10px;
  margin-left: -4px;
  text-shadow: 0 0 8px var(--accent);
  animation: pulse-dirty 2s infinite;
}

@keyframes pulse-dirty {
  0% { opacity: 0.6; }
  50% { opacity: 1; }
  100% { opacity: 0.6; }
}

.editor-relative-wrapper {
  flex: 1;
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: #282c34;
}

.latex-editor-cm {
  flex: 1;
  width: 100%;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
}

:deep(.cm-editor) {
  height: 100%;
  outline: none !important;
}

:deep(.cm-content) {
  padding: 20px 0;
}

:deep(.cm-gutters) {
  background-color: #282c34 !important;
  border-right: 1px solid #3e4451 !important;
  color: #abb2bf !important;
}

.refinement-bar {
  position: absolute;
  top: 50px;
  left: 50%;
  width: 90%;
  max-width: 440px;
  background: var(--surface-soft);
  border: 1px solid var(--accent-soft);
  border-radius: 20px;
  display: flex;
  padding: 4px 14px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.6);
  z-index: 1000;
  cursor: grab;
}

.refinement-bar:active {
  cursor: grabbing;
}

.refinement-bar input {
  flex: 1;
  background: none;
  border: none;
  color: var(--ink);
  font-size: 0.75rem;
  padding: 8px 0;
  outline: none;
  cursor: text;
}

.refinement-bar button {
  background: var(--accent);
  color: white;
  border: none;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  margin-left: 10px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  align-self: center;
}

.refinement-bar button .spinner {
  color: white !important;
}

.pdf-viewer {
  flex: 1;
  display: block;
  background: var(--bg);
  position: relative;
  overflow: auto;
}

.pdf-embed-component {
  width: 100%;
  height: 100%;
  border: none;
  display: block;
  background: white;
}

.empty-preview {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-accent);
  color: var(--muted);
  text-align: center;
  padding: 40px;
}

.placeholder-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  max-width: 300px;
}

.placeholder-content h3 {
  font-size: 1rem;
  color: var(--ink);
  margin: 0;
}

.error-console {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  max-height: 40%;
  background: var(--surface);
  border-top: 2px solid var(--warning);
  display: flex;
  flex-direction: column;
  z-index: 50;
  box-shadow: 0 -8px 24px rgba(0, 0, 0, 0.3);
}

.console-header {
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: var(--bg-accent);
  border-bottom: 1px solid var(--line);
}

.console-actions {
  display: flex;
  gap: 8px;
}

.action-btn-inline {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
  transition: 0.15s;
}

.action-btn-inline:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.success-icon {
  color: var(--accent);
}

.close-btn:hover {
  color: var(--warning);
}

.error-logs-container {
  flex: 1;
  overflow-y: auto;
  background: var(--bg);
}

.error-logs {
  margin: 0;
  padding: 16px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.8rem;
  color: var(--ink);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.loading-overlay {
  position: absolute;
  top: 32px;
  left: 0;
  width: 100%;
  height: calc(100% - 32px);
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

.spinner {
  color: var(--accent);
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.modal-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(4px);
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.template-modal {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  width: 90%;
  max-width: 600px;
  padding: 24px;
  box-shadow: var(--shadow);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.modal-header h3 { margin: 0; }
.modal-header button { background: none; border: none; color: var(--muted); cursor: pointer; }

.template-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.template-card {
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  cursor: pointer;
  transition: 0.2s;
  text-align: center;
}

.template-card:hover {
  border-color: var(--accent);
  background: var(--surface);
  transform: translateY(-2px);
}

.temp-icon { color: var(--accent); margin-bottom: 12px; }
.template-card h4 { margin: 0 0 8px; font-size: 1rem; }
.template-card p { margin: 0; font-size: 0.75rem; color: var(--muted); }

@media (max-width: 1024px) {
  .workspace-sidebar {
    width: 200px;
  }
}

@media (max-width: 768px) {
  .split-pane {
    flex-direction: column;
  }
  
  .workspace-sidebar {
    width: 100%;
    height: 200px;
    border-right: none;
    border-bottom: 1px solid var(--line);
  }
  
  .editor-section {
    border-right: none;
    border-bottom: 1px solid var(--line);
  }
}
</style>
