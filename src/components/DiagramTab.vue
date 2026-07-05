<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save } from '@tauri-apps/plugin-dialog';
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
import { useDialogStore } from '../store/dialog';
import { useSettingsStore } from '../store/settings';
import mermaid from 'mermaid';
import svgPanZoom from 'svg-pan-zoom';
import { revealItemInDir } from '@tauri-apps/plugin-opener';

// Markdown imports
import MarkdownIt from 'markdown-it';
import DOMPurify from 'dompurify';
import markdownItKatex from 'markdown-it-katex';
import highlightjs from 'highlight.js';
import 'github-markdown-css/github-markdown-dark.css';
import 'highlight.js/styles/github-dark.css';
import 'katex/dist/katex.min.css';

import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { 
  RotateCw,
  Loader2,
  X,
  FileCode,
  FolderOpen,
  Plus,
  FolderPlus,
  Share2,
  Workflow,
  Layout,
  Save,
  BookOpen,
  Info,
  Wand2,
  ArrowLeftRight,
  Copy,
  Check,
  Download,
  ImageDown,
  Maximize2,
  FileUp,
  ExternalLink,
  PanelRight
} from '@lucide/vue';

import { Codemirror } from 'vue-codemirror';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView } from '@codemirror/view';
import FileTreeItem from './FileTreeItem.vue';

const dialog = useDialogStore();
const settingsStore = useSettingsStore();

// Markdown Init
const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
  highlight: (str, lang) => {
    if (lang && highlightjs.getLanguage(lang)) {
      try {
        return highlightjs.highlight(str, { language: lang }).value;
      } catch (__) {}
    }
    return ''; // use external default escaping
  }
}).use(markdownItKatex);

// Mermaid Init
mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  securityLevel: 'loose',
  flowchart: { useMaxWidth: false, htmlLabels: false }, // Use SVG text for security
  sequence: { useMaxWidth: false, showSequenceNumbers: true },
  er: { useMaxWidth: false }
});

// Codemirror Extensions
const extensions = [
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
const fileTree = ref<FileItem[]>([]);
const activeFilePath = ref<string | null>(null);
const isTemplatesVisible = ref(false);
const isExporting = ref(false);
const isPreviewVisible = ref(true);

const togglePreview = () => {
  isPreviewVisible.value = !isPreviewVisible.value;
};

const workspaceName = computed(() => {
  if (!workspacePath.value) return '';
  return workspacePath.value.split(/[/\\]/).pop() || '';
});
const standaloneFileType = ref<'mmd' | 'md'>('mmd');
const diagramCode = ref('graph TD\n    A[Start] --> B{Process}\n    B -->|Success| C[End]\n    B -->|Failure| D[Retry]');

const isSidebarVisible = ref(true);
const sidebarWidth = ref(240);
const isResizing = ref(false);

const diagramSvg = ref('');
const markdownHtml = ref('');
const isRendering = ref(false);
const renderingError = ref<string | null>(null);
const isCopyingError = ref(false);
const isDirty = ref(false);

const handleCopyError = async () => {
  if (!renderingError.value) return;
  isCopyingError.value = true;
  try {
    await writeText(renderingError.value);
    setTimeout(() => { isCopyingError.value = false; }, 2000);
  } catch (err) {
    console.error('Failed to copy error:', err);
    isCopyingError.value = false;
  }
};
const isInitialLoad = ref(true);
const isProgrammaticChange = ref(false);
const editorContainer = ref<HTMLElement | null>(null);
const previewContainer = ref<HTMLElement | null>(null);
const isLoadingWorkspace = ref(false);
const panZoomInstance = ref<any>(null);
let renderTimeout: any = null;
let currentRenderVersion = 0;

const activeTooltip = ref<string | null>(null);

const isRefining = ref(false);
const isFixing = ref(false);
const refinementInstruction = ref('');

// Export Resolution State
const isExportModalVisible = ref(false);
const selectedExportPreset = ref('2x');
const customExportWidth = ref(3840);
const exportProgress = ref('');

interface ExportPreset {
  id: string;
  label: string;
  sublabel: string;
  targetWidth: number | null; // null = use scale, number = target width in px
  scale: number | null;       // used when targetWidth is null
  badge?: string;
}

const exportPresets: ExportPreset[] = [
  { id: '1x', label: 'Original', sublabel: 'Native SVG size', targetWidth: null, scale: 1 },
  { id: '2x', label: '2× Retina', sublabel: 'Good for screens', targetWidth: null, scale: 2, badge: 'Default' },
  { id: '4k', label: '4K UHD', sublabel: '3840px wide', targetWidth: 3840, scale: null },
  { id: '8k', label: '8K UHD', sublabel: '7680px wide', targetWidth: 7680, scale: null },
  { id: '16k', label: '16K', sublabel: '15360px wide', targetWidth: 15360, scale: null, badge: 'Ultra' },
  { id: '32k', label: '32K', sublabel: '30720px wide', targetWidth: 30720, scale: null, badge: 'Max' },
  { id: 'custom', label: 'Custom', sublabel: 'Set your own width', targetWidth: null, scale: null },
];

const selectedPresetObj = computed(() => exportPresets.find(p => p.id === selectedExportPreset.value)!);

const estimatedExportDimensions = computed(() => {
  const svgElement = previewContainer.value?.querySelector('svg');
  if (!svgElement) return { width: 0, height: 0, megapixels: '0' };
  
  const bbox = svgElement.getBBox();
  const padding = 20;
  const baseWidth = bbox.width + padding * 2;
  const baseHeight = bbox.height + padding * 2;
  
  const preset = selectedPresetObj.value;
  let finalWidth: number;
  let finalHeight: number;
  
  if (preset.id === 'custom') {
    const s = customExportWidth.value / baseWidth;
    finalWidth = customExportWidth.value;
    finalHeight = Math.round(baseHeight * s);
  } else if (preset.scale !== null) {
    finalWidth = Math.round(baseWidth * preset.scale);
    finalHeight = Math.round(baseHeight * preset.scale);
  } else if (preset.targetWidth !== null) {
    const s = preset.targetWidth / baseWidth;
    finalWidth = preset.targetWidth;
    finalHeight = Math.round(baseHeight * s);
  } else {
    finalWidth = Math.round(baseWidth);
    finalHeight = Math.round(baseHeight);
  }
  
  const mp = (finalWidth * finalHeight) / 1_000_000;
  return { width: finalWidth, height: finalHeight, megapixels: mp.toFixed(1) };
});

const isExportDangerous = computed(() => {
  const { width, height } = estimatedExportDimensions.value;
  // Most browsers cap canvas at 16384×16384
  return width > 16384 || height > 16384;
});

const openExportModal = () => {
  if (isMarkdown.value || !diagramSvg.value) return;
  isExportModalVisible.value = true;
  exportProgress.value = '';
};

const downloadAsSvg = async () => {
  if (isMarkdown.value || !diagramSvg.value) return;
  
  try {
    const svgElement = previewContainer.value?.querySelector('svg');
    if (!svgElement) throw new Error("Rendered diagram not found.");

    const serializedSvg = new XMLSerializer().serializeToString(svgElement);
    const bytes = new TextEncoder().encode(serializedSvg);

    const fileName = activeFilePath.value 
      ? activeFilePath.value.split(/[/\\]/).pop()?.replace(/\.[^/.]+$/, "") + ".svg"
      : "diagram.svg";

    const filePath = await save({
      defaultPath: fileName,
      filters: [{ name: 'SVG Image', extensions: ['svg'] }]
    });

    if (filePath) {
      await writeFile(filePath, bytes);
      await dialog.showAlert("Diagram exported as SVG successfully.", "Export Complete");
    }
  } catch (err: any) {
    renderingError.value = `SVG EXPORT FAILED: ${err.toString()}`;
  }
};

const downloadAsPng = async () => {
  if (isMarkdown.value || !diagramSvg.value || isExporting.value) return;

  isExporting.value = true;
  renderingError.value = null;
  exportProgress.value = 'Preparing SVG...';

  try {
    const svgElement = previewContainer.value?.querySelector('svg');
    if (!svgElement) throw new Error("Rendered diagram not found.");

    // 1. Get base dimensions from SVG bounding box
    const bbox = svgElement.getBBox();
    const padding = 20;
    const baseWidth = bbox.width + padding * 2;
    const baseHeight = bbox.height + padding * 2;
    
    // 2. Compute scale from the selected preset
    let scale: number;
    const preset = selectedPresetObj.value;
    
    if (preset.id === 'custom') {
      scale = customExportWidth.value / baseWidth;
    } else if (preset.scale !== null) {
      scale = preset.scale;
    } else if (preset.targetWidth !== null) {
      scale = preset.targetWidth / baseWidth;
    } else {
      scale = 2; // fallback
    }
    
    const canvasWidth = Math.round(baseWidth * scale);
    const canvasHeight = Math.round(baseHeight * scale);
    
    exportProgress.value = `Rendering ${canvasWidth} × ${canvasHeight} (${(canvasWidth * canvasHeight / 1_000_000).toFixed(1)} MP)...`;
    
    // 3. Clone and prepare SVG string
    const clonedSvg = svgElement.cloneNode(true) as SVGElement;
    clonedSvg.setAttribute('width', baseWidth.toString());
    clonedSvg.setAttribute('height', baseHeight.toString());
    clonedSvg.setAttribute('viewBox', `${bbox.x - padding} ${bbox.y - padding} ${baseWidth} ${baseHeight}`);

    // Embed basic styles
    const style = document.createElement('style');
    style.textContent = `
      svg { background-color: ${getComputedStyle(document.documentElement).getPropertyValue('--surface').trim() || '#0d1117'}; }
      text, span, div { font-family: sans-serif !important; }
    `;
    clonedSvg.prepend(style);

    const serializedSvg = new XMLSerializer().serializeToString(clonedSvg);
    
    // 4. Convert to Data URL (Bypasses many blob security issues)
    const encodedData = window.btoa(unescape(encodeURIComponent(serializedSvg)));
    const dataUrl = `data:image/svg+xml;base64,${encodedData}`;

    // 5. Check canvas size limits (most browsers cap at ~16384, some allow ~32768)
    const CANVAS_HARD_LIMIT = 32768;
    if (canvasWidth > CANVAS_HARD_LIMIT || canvasHeight > CANVAS_HARD_LIMIT) {
      throw new Error(
        `Requested resolution (${canvasWidth}×${canvasHeight}) exceeds the browser's maximum canvas dimension of ${CANVAS_HARD_LIMIT}px. ` +
        `Try a lower resolution or export as SVG (which is infinitely scalable).`
      );
    }

    // 6. Render to Canvas
    const canvas = document.createElement('canvas');
    canvas.width = canvasWidth;
    canvas.height = canvasHeight;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error("Canvas context initialization failed.");

    const img = new Image();
    img.onload = async () => {
      try {
        exportProgress.value = 'Drawing to canvas...';
        ctx.fillStyle = getComputedStyle(document.documentElement).getPropertyValue('--surface').trim() || '#0d1117';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        
        exportProgress.value = 'Encoding PNG...';
        const pngUrl = canvas.toDataURL('image/png');
        const base64Data = pngUrl.split(',')[1];
        const bytes = Uint8Array.from(atob(base64Data), c => c.charCodeAt(0));

        const presetLabel = preset.id === 'custom' ? `${canvasWidth}px` : preset.label;
        const fileName = activeFilePath.value 
          ? activeFilePath.value.split(/[/\\]/).pop()?.replace(/\.[^/.]+$/, "") + `_${presetLabel.replace(/\s+/g, '_')}.png`
          : `diagram_${presetLabel.replace(/\s+/g, '_')}.png`;

        exportProgress.value = 'Saving file...';
        const filePath = await save({
          defaultPath: fileName,
          filters: [{ name: 'PNG Image', extensions: ['png'] }]
        });

        if (filePath) {
          await writeFile(filePath, bytes);
          await dialog.showAlert(
            `Exported at ${canvasWidth} × ${canvasHeight} (${(canvasWidth * canvasHeight / 1_000_000).toFixed(1)} MP)`,
            "Export Complete"
          );
        }
      } catch (err: any) {
        console.error("PNG Conversion Error:", err);
        renderingError.value = `PNG EXPORT ERROR: ${err.message || err.toString()}\n\nTip: If PNG export is blocked, try using 'Export as SVG' instead.`;
      } finally {
        isExporting.value = false;
        isExportModalVisible.value = false;
        exportProgress.value = '';
      }
    };
    
    img.onerror = () => {
      isExporting.value = false;
      isExportModalVisible.value = false;
      exportProgress.value = '';
      renderingError.value = "PNG EXPORT FAILED: The browser security policy blocked the conversion. This usually happens if the diagram uses features (like HTML labels) that 'taint' the drawing surface. Use 'Export as SVG' for a guaranteed save.";
    };
    
    img.src = dataUrl;

  } catch (err: any) {
    renderingError.value = `EXPORT INITIATION FAILED: ${err.toString()}`;
    isExporting.value = false;
    isExportModalVisible.value = false;
    exportProgress.value = '';
  }
};

const isMarkdown = computed(() => {
  if (activeFilePath.value) {
    return activeFilePath.value.toLowerCase().endsWith('.md');
  }
  return standaloneFileType.value === 'md';
});

const toggleStandaloneType = () => {
  standaloneFileType.value = standaloneFileType.value === 'mmd' ? 'md' : 'mmd';
  // Optional: Switch default code if it looks like the other type's default
  if (standaloneFileType.value === 'md' && diagramCode.value.startsWith('graph')) {
    diagramCode.value = '# New Document\n\nWrite your markdown here...';
  } else if (standaloneFileType.value === 'mmd' && diagramCode.value.startsWith('#')) {
    diagramCode.value = 'graph TD\n    A --> B';
  }
  renderContent();
};

const diagramTemplates = [
  {
    name: 'Simple Flowchart',
    description: 'Basic logic flow with nodes and conditions.',
    content: `graph TD
    A[Start] --> B{Is it working?}
    B -- Yes --> C[Great!]
    B -- No --> D[Fix it]
    D --> B`,
    type: 'mmd'
  },
  {
    name: 'Sequence Diagram',
    description: 'Model interaction between multiple entities.',
    content: `sequenceDiagram
    participant User
    participant App
    participant DB
    User->>App: Login Request
    App->>DB: Validate Credentials
    DB-->>App: Success
    App-->>User: Auth Token`,
    type: 'mmd'
  },
  {
    name: 'Markdown Doc',
    description: 'Document with embedded Mermaid diagram.',
    content: `# Technical Documentation

This is a **Markdown** document with support for KaTeX math: $E = mc^2$

## System Architecture

\`\`\`mermaid
graph LR
    UI[Frontend] <--> API[Tauri Backend]
    API <--> DB[(SQLite)]
\`\`\`

---
*Generated by Roletect*`,
    type: 'md'
  }
];

// Persistence & Initialization
onMounted(async () => {
  try {
    const savedWorkspace = await invoke<string | null>('get_diagram_workspace_path');
    if (savedWorkspace && await exists(savedWorkspace)) {
      workspacePath.value = savedWorkspace;
      await refreshFileTree();

      const lastFile = await invoke<string | null>('get_last_opened_diagram');
      if (lastFile && await exists(lastFile)) {
        // Render the last opened file on startup for a seamless experience
        await selectFile({ name: lastFile.split(/[/\\]/).pop() || '', path: lastFile, isDir: false }, false);
      }
    }
    
    // Set initial load to false after setup
    isInitialLoad.value = false;
    
    // Ensure initial load doesn't mark it as dirty
    setTimeout(() => { isDirty.value = false; }, 150);
  } catch (err) {
    console.error('Failed to initialize Diagram Studio:', err);
    isInitialLoad.value = false;
  }
});

onUnmounted(async () => {
  if (renderTimeout) clearTimeout(renderTimeout);
  if (isDirty.value) {
    await saveActiveFile();
  }
  // Clear preview state
  diagramSvg.value = '';
  markdownHtml.value = '';
});

// Re-render diagram immediately if user changes the editor theme to match the new colors
watch(() => settingsStore.activeThemeId, () => {
  renderContent();
});

// Sidebar methods
const toggleSidebar = () => isSidebarVisible.value = !isSidebarVisible.value;
const startResizing = (_e: MouseEvent) => {
  isResizing.value = true;
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', stopResizing);
};
const handleMouseMove = (e: MouseEvent) => {
  if (!isResizing.value) return;
  if (e.clientX > 150 && e.clientX < 500) sidebarWidth.value = e.clientX;
};
const stopResizing = () => {
  isResizing.value = false;
  document.removeEventListener('mousemove', handleMouseMove);
  document.removeEventListener('mouseup', stopResizing);
};

// Workspace Management
const selectWorkspace = async () => {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: 'Select Diagram Workspace'
    });

    if (selected && typeof selected === 'string') {
      workspacePath.value = selected;
      await invoke('save_diagram_workspace_path', { path: selected });
      await refreshFileTree();
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
      title: 'Open Diagram/Markdown File',
      filters: [{ name: 'Diagram/Markdown', extensions: ['mmd', 'md'] }]
    });

    if (selected && typeof selected === 'string') {
      await selectFile({ name: selected.split(/[/\\]/).pop() || '', path: selected, isDir: false }, false);
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

const refreshOpenFolders = async (items: FileItem[]) => {
  for (const item of items) {
    if (item.isDir && item.isOpen) {
      item.children = await scanDirectory(item.path);
      if (item.children) {
        await refreshOpenFolders(item.children);
      }
    }
  }
};

const refreshFileTree = async () => {
  if (!workspacePath.value) return;
  isLoadingWorkspace.value = true;
  try {
    const newTree = await scanDirectory(workspacePath.value);
    
    // Preserve open state and recursive children for open folders
    const syncTrees = (oldItems: FileItem[], newItems: FileItem[]) => {
      for (const newItem of newItems) {
        const oldItem = oldItems.find(i => i.path === newItem.path);
        if (oldItem && oldItem.isDir) {
          newItem.isOpen = oldItem.isOpen;
          newItem.children = oldItem.children;
        }
      }
    };
    
    syncTrees(fileTree.value, newTree);
    fileTree.value = newTree;
    
    // Re-scan all open folders to ensure they are up to date
    await refreshOpenFolders(fileTree.value);
  } catch (err) {
    console.error('Failed to scan workspace:', err);
  } finally {
    isLoadingWorkspace.value = false;
  }
};

const scanDirectory = async (dir: string): Promise<FileItem[]> => {
  const entries = await readDir(dir);
  const items: FileItem[] = [];

  for (const entry of entries) {
    const fullPath = await join(dir, entry.name);
    const isDir = entry.isDirectory;
    
    // Support .mmd and .md
    if (!isDir && !entry.name.toLowerCase().endsWith('.mmd') && !entry.name.toLowerCase().endsWith('.md')) continue;

    items.push({
      name: entry.name,
      path: fullPath,
      isDir: isDir,
      isOpen: false,
      children: isDir ? [] : undefined
    });
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
    item.children = await scanDirectory(item.path);
  }
};

const selectFile = async (item: FileItem, skipRender = false) => {
  if (item.isDir) return;
  
  if (isDirty.value && activeFilePath.value) {
    await saveActiveFile();
  }

  try {
    const fileExists = await exists(item.path);
    if (!fileExists) {
      await dialog.showAlert(`The file "${item.name}" no longer exists on disk.`, 'File Not Found');
      await refreshFileTree();
      return;
    }

    const content = await readTextFile(item.path);
    
    // Set programmatic flag to prevent the watcher from triggering auto-render/dirty
    isProgrammaticChange.value = true;
    diagramCode.value = content;
    activeFilePath.value = item.path;
    isDirty.value = false;
    
    await invoke('save_last_opened_diagram', { path: item.path });
    
    if (!skipRender) {
      await renderContent();
    } else {
      // Reset preview state for the new file to remain quiet during browsing
      diagramSvg.value = '';
      markdownHtml.value = '';
      renderingError.value = null;
    }
    
    // Safely reset programmatic flag after any watcher has run, even if content was identical
    await nextTick();
    isProgrammaticChange.value = false;
  } catch (err: any) {
    console.error('Failed to read file:', err);
    await dialog.showAlert(`Failed to open file: ${err.message || err.toString()}`, 'Read Error');
  }
};

const saveActiveFile = async () => {
  if (!activeFilePath.value) return;

  try {
    const lastSlash = Math.max(activeFilePath.value.lastIndexOf('/'), activeFilePath.value.lastIndexOf('\\'));
    const dirPath = lastSlash !== -1 ? activeFilePath.value.substring(0, lastSlash) : null;
    
    if (dirPath && !(await exists(dirPath))) {
      await dialog.showAlert("The parent directory for this file is missing.", "Save Failed");
      return;
    }

    await writeFile(activeFilePath.value, new TextEncoder().encode(diagramCode.value));
    isDirty.value = false;
  } catch (err: any) {
    console.error('Failed to save file:', err);
    await dialog.showAlert(`Save failed: ${err.message || err.toString()}`, 'Write Error');
  }
};

const createNewFile = async (parent: FileItem | null = null, ext = '.mmd') => {
  const dir = parent ? parent.path : workspacePath.value;
  if (!dir) return;

  const fileName = await dialog.showPrompt(`Enter name (e.g. flow${ext}):`, '', 'New File');
  if (!fileName) return;

  // Check if user already provided an extension
  const hasExtension = fileName.toLowerCase().endsWith('.mmd') || fileName.toLowerCase().endsWith('.md');
  const finalName = hasExtension ? fileName : `${fileName}${ext}`;
  const fullPath = await join(dir, finalName);

  if (await exists(fullPath)) {
    await dialog.showAlert(`A file or folder named "${finalName}" already exists.`, 'Create Failed');
    return;
  }
  
  // Determine initial content based on the FINAL extension
  const isMd = finalName.toLowerCase().endsWith('.md');
  const initialContent = isMd ? '# New Document\n\nWrite your markdown here...' : 'graph TD\n    A --> B';
  
  try {
    await writeFile(fullPath, new TextEncoder().encode(initialContent));
    
    // Always refresh the full tree to ensure UI is in sync
    await refreshFileTree();

    // Select the new file
    await selectFile({ name: finalName, path: fullPath, isDir: false });
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
    
    if (activeFilePath.value) {
      const isSelf = activeFilePath.value === item.path;
      const isParent = activeFilePath.value.startsWith(item.path + '/') || activeFilePath.value.startsWith(item.path + '\\');
      
      if (isSelf || isParent) {
        activeFilePath.value = null;
        diagramCode.value = 'graph TD\n    A --> B';
        isDirty.value = false;
        diagramSvg.value = '';
        markdownHtml.value = '';
      }
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
      await invoke('save_last_opened_diagram', { path: newPath });
    } else if (activeFilePath.value && (activeFilePath.value.startsWith(oldPath + '/') || activeFilePath.value.startsWith(oldPath + '\\'))) {
      const rel = activeFilePath.value.substring(oldPath.length);
      activeFilePath.value = newPath + rel;
      await invoke('save_last_opened_diagram', { path: newPath + rel });
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
  await invoke('save_diagram_workspace_path', { path: '' });
};

const useTemplate = async (template: typeof diagramTemplates[0]) => {
  const confirmed = await dialog.showConfirm(`Overwrite current editor content with the "${template.name}" template?`, 'Use Template');
  if (confirmed) {
    diagramCode.value = template.content;
    isTemplatesVisible.value = false;
    isDirty.value = true;
    
    // For safety, we just render and mark dirty.
    await renderContent();
  }
};

// AI Logic
const refineWithAi = async () => {
  if (!diagramCode.value || !refinementInstruction.value.trim() || isRefining.value) return;
  
  isRefining.value = true;
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const refinedCode = await invoke<string>('refine_diagram_with_ai', {
      provider,
      model,
      apiKey,
      currentCode: diagramCode.value,
      instruction: refinementInstruction.value.trim(),
      contentType: isMarkdown.value ? "Markdown" : "Mermaid"
    });

    diagramCode.value = refinedCode;
    refinementInstruction.value = '';
    await saveActiveFile();
    await renderContent();
  } catch (err: any) {
    console.error("AI Refinement Error:", err);
    await dialog.showAlert(err.toString(), 'AI Refinement Failed');
  } finally {
    isRefining.value = false;
  }
};

const fixWithAi = async () => {
  if (!diagramCode.value || !renderingError.value || isFixing.value) return;
  
  isFixing.value = true;
  try {
    const apiKey = await settingsStore.getDecryptedKey();
    if (!apiKey) throw new Error("API Key not found. Please set it in Settings.");

    const provider = settingsStore.selectedAiProvider;
    const model = settingsStore.selectedAiModel;

    const fixedCode = await invoke<string>('fix_diagram_with_ai', {
      provider,
      model,
      apiKey,
      brokenCode: diagramCode.value,
      errorLogs: renderingError.value,
      contentType: isMarkdown.value ? "Markdown" : "Mermaid"
    });

    diagramCode.value = fixedCode;
    renderingError.value = null;
    await saveActiveFile();
    await renderContent();
  } catch (err: any) {
    console.error("AI Fix Error:", err);
    await dialog.showAlert(err.toString(), 'AI Fix Failed');
  } finally {
    isFixing.value = false;
  }
};

// Dynamically retrieve active theme colors and compile them into theme variables for Mermaid
const getMermaidThemeVariables = () => {
  const style = getComputedStyle(document.documentElement);
  const getVal = (varName: string, fallback: string) => style.getPropertyValue(varName).trim() || fallback;

  const bg = getVal('--bg', '#0d1117');
  const surface = getVal('--surface', '#161b22');
  const surfaceSoft = getVal('--surface-soft', '#21262d');
  const line = getVal('--line', '#30363d');
  const ink = getVal('--ink', '#c9d1d9');
  const muted = getVal('--muted', '#8b949e');
  const accent = getVal('--accent', '#58a6ff');

  return {
    theme: 'base' as const,
    themeVariables: {
      background: bg,
      primaryColor: surface,
      primaryTextColor: ink,
      primaryBorderColor: accent,
      lineColor: muted,
      secondaryColor: surfaceSoft,
      tertiaryColor: bg,
      
      // Node, note and actor text/background configurations
      mainBkg: surface,
      nodeBorder: accent,
      actorBorder: accent,
      actorBkg: surface,
      actorTextColor: ink,
      actorLineColor: muted,
      signalColor: ink,
      signalLineColor: muted,
      labelBoxBorderColor: line,
      labelBoxBkgColor: surface,
      labelTextColor: ink,
      loopLimitBorderColor: line,
      loopLimitBkgColor: surface,
      noteBorderColor: line,
      noteBkgColor: surfaceSoft,
      noteTextColor: ink,
      
      // Flowchart settings
      nodeTextColor: ink,
      edgeLabelBackground: surfaceSoft,
      arrowheadColor: muted,
      
      // Entity-Relationship diagram settings
      attributeBackgroundColor: surface,
      attributeBorderColor: line,
      entityBackgroundColor: surface,
      entityBorderColor: accent,
      
      // Typography
      fontFamily: style.getPropertyValue('--font-family').trim() || 'sans-serif',
    }
  };
};

// Rendering Logic
const renderContent = async () => {
  const codeToRender = diagramCode.value.trim();
  if (!codeToRender) {
    diagramSvg.value = '';
    markdownHtml.value = '';
    renderingError.value = null;
    return;
  }
  
  // Increment version to track this specific render request
  const version = ++currentRenderVersion;
  
  isRendering.value = true;
  
  try {
        // 1. Initialize Mermaid dynamic theme variables based on current custom UI theme
    const config = {
      startOnLoad: false,
      securityLevel: 'loose' as const,
      flowchart: { useMaxWidth: false, htmlLabels: false },
      sequence: { useMaxWidth: false, showSequenceNumbers: true },
      er: { useMaxWidth: false },
      ...getMermaidThemeVariables()
    };
    mermaid.initialize(config);

    if (isMarkdown.value) {
      const rawHtml = md.render(codeToRender);
      const sanitized = DOMPurify.sanitize(rawHtml);
      
      if (version !== currentRenderVersion) return;

      // Validate embedded Mermaid blocks beforehand to prevent crashes during actual render
      const tempDiv = document.createElement('div');
      tempDiv.innerHTML = sanitized;
      const mermaidNodes = tempDiv.querySelectorAll('.language-mermaid');
      for (const node of mermaidNodes) {
        const code = node.textContent || '';
        await mermaid.parse(code);
      }
      
      if (version !== currentRenderVersion) return;
      
      markdownHtml.value = sanitized;
      renderingError.value = null;
      
      await nextTick();
      // Render mermaid inside markdown
      const renderedNodes = previewContainer.value?.querySelectorAll('.language-mermaid');
      if (renderedNodes) {
        for (const node of renderedNodes) {
          const code = node.textContent || '';
          const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
          const { svg } = await mermaid.render(id, code);
          
          if (version !== currentRenderVersion) return;

          const wrapper = document.createElement('div');
          wrapper.className = 'mermaid-rendered-wrapper';
          wrapper.innerHTML = svg;
          node.parentElement?.replaceWith(wrapper);
        }
      }
    } else {
      // Validate Mermaid syntax first to prevent DOM pollution from failed renders
      await mermaid.parse(codeToRender);
      
      if (version !== currentRenderVersion) return;

      const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
      const { svg } = await mermaid.render(id, codeToRender);
      
      if (version !== currentRenderVersion) return;

      diagramSvg.value = svg;
      renderingError.value = null;
      
      await nextTick();
      initializePanZoom();
    }
  } catch (err: any) {
    if (version !== currentRenderVersion) return;
    console.error("Render Error:", err);
    renderingError.value = err.message || err.toString();
  } finally {
    if (version === currentRenderVersion) {
      isRendering.value = false;
    }
  }
};

const initializePanZoom = () => {
  if (panZoomInstance.value) {
    panZoomInstance.value.destroy();
    panZoomInstance.value = null;
  }

  if (isMarkdown.value) return;

  const svgElement = previewContainer.value?.querySelector('svg');
  if (svgElement) {
    svgElement.style.width = '100%';
    svgElement.style.height = '100%';
    
    panZoomInstance.value = svgPanZoom(svgElement, {
      zoomEnabled: true,
      controlIconsEnabled: true,
      fit: true,
      center: true,
      minZoom: 0.1,
      maxZoom: 10
    });
  }
};

// Auto-save & Render logic
watch(diagramCode, () => {
  // 1. Skip during initial setup
  if (isInitialLoad.value) return;

  // 2. Skip when the change is programmatic (from selectFile)
  if (isProgrammaticChange.value) {
    isProgrammaticChange.value = false;
    return;
  }

  isDirty.value = true;

  if (settingsStore.isAutoCompileEnabled) {
    if (renderTimeout) clearTimeout(renderTimeout);
    renderTimeout = setTimeout(() => {
      renderContent();
    }, 1000);
  }
});

const handleBlur = async () => {
  if (isDirty.value) {
    await saveActiveFile();
    if (settingsStore.isAutoCompileEnabled) {
      await renderContent();
    }
  }
};

const activeFileName = computed(() => {
  if (!activeFilePath.value) return `unsaved.${standaloneFileType.value}`;
  return activeFilePath.value.split(/[/\\]/).pop() || 'diagram.mmd';
});
</script>

<template>
  <div class="studio-container">
    <header class="studio-header">
      <div class="header-left">
        <button class="toggle-sidebar-btn" @click="toggleSidebar" title="Toggle Sidebar">
          <Layout :size="18" />
        </button>
        <Share2 :size="20" class="header-icon" />
        <h1>Diagram Studio</h1>
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

        <div v-if="!activeFilePath" class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'switch-type'" @mouseleave="activeTooltip = null">
          <button class="action-btn mode-toggle" @click="toggleStandaloneType">
            <ArrowLeftRight :size="16" />
            <span class="mode-label">{{ standaloneFileType.toUpperCase() }}</span>
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'switch-type'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Switch to {{ standaloneFileType === 'mmd' ? 'Markdown' : 'Mermaid' }}
            </Motion>
          </AnimatePresence>
        </div>

        <div class="divider-v"></div>

        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'auto-render'" @mouseleave="activeTooltip = null">
          <label class="auto-compile-toggle">
            <input 
              type="checkbox" 
              :checked="settingsStore.isAutoCompileEnabled"
              @change="settingsStore.setAutoCompile(($event.target as HTMLInputElement).checked)"
            >
            <Info :size="12" class="info-icon" />
          </label>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'auto-render'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Live Render on Stop Typing
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'save'" @mouseleave="activeTooltip = null">
          <button 
            class="action-btn save-btn"
            @click="saveActiveFile"
            :disabled="!isDirty"
          >
            <Save :size="16" />
            <span>Save</span>
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'save'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              {{ isDirty ? 'Save Changes' : 'All Changes Saved' }}
            </Motion>
          </AnimatePresence>
        </div>

        <div class="btn-tooltip-wrapper" v-if="renderingError" @mouseenter="activeTooltip = 'ai-fix'" @mouseleave="activeTooltip = null">
          <button 
            class="action-btn ai-fix-btn" 
            @click="fixWithAi" 
            :disabled="isFixing"
          >
            <Wand2 :size="16" />
            <span>AI Fix</span>
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'ai-fix'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              Repair Syntax Errors
            </Motion>
          </AnimatePresence>
        </div>
        
        <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'render'" @mouseleave="activeTooltip = null">
          <button 
            class="action-btn render-btn" 
            @click="renderContent" 
            :disabled="isRendering || !diagramCode"
          >
            <Workflow v-if="!isRendering" :size="16" />
            <RotateCw v-else :size="16" class="spinner" />
          </button>
          <AnimatePresence>
            <Motion
              v-if="activeTooltip === 'render'"
              :initial="{ opacity: 0, y: 5, scale: 0.9 }"
              :animate="{ opacity: 1, y: 0, scale: 1 }"
              :exit="{ opacity: 0, y: 5, scale: 0.9 }"
              :transition="{ duration: 0.15 }"
              class="floating-message tooltip-bottom-left"
            >
              {{ isMarkdown ? 'Preview Content' : 'Render Diagram' }}
            </Motion>
          </AnimatePresence>
        </div>
      </div>
    </header>

    <main class="studio-main">
      <div class="split-pane">
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
              <button class="header-tool-btn" @click="createNewFile(null, '.mmd')" title="New Diagram"><Plus :size="14" /></button>
              <button class="header-tool-btn" @click="createNewFile(null, '.md')" title="New Markdown"><FileCode :size="14" /></button>
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

          <div v-else class="file-tree">
            <div v-if="isLoadingWorkspace" class="tree-loading">
              <RotateCw :size="16" class="spinner" />
            </div>
            
            <template v-else>
              <FileTreeItem 
                v-for="item in fileTree" 
                :key="item.path"
                :item="item"
                :active-file-path="activeFilePath"
                :is-diagram="true"
                :on-toggle="toggleFolder"
                :on-select="selectFile"
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
            <div class="pane-header-actions">
              <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'preview'" @mouseleave="activeTooltip = null">
                <button class="action-btn-inline" @click="togglePreview" :class="{ active: isPreviewVisible }">
                  <PanelRight :size="14" />
                </button>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'preview'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    class="floating-message tooltip-bottom-left"
                  >
                    Toggle Preview
                  </Motion>
                </AnimatePresence>
              </div>
            </div>
          </div>
          <div class="editor-relative-wrapper" ref="editorContainer">
            <codemirror
              v-model="diagramCode"
              placeholder="Enter Mermaid or Markdown code..."
              :style="{ height: '100%' }"
              :autofocus="true"
              :indent-with-tab="true"
              :tab-size="2"
              :extensions="extensions"
              @blur="handleBlur"
              class="mermaid-editor-cm"
            />

            <AnimatePresence>
              <Motion 
                v-if="diagramCode"
                class="refinement-bar"
                drag
                :drag-constraints="editorContainer || undefined"
                :drag-elastic="0.1"
                :initial="{ opacity: 0, y: -10, x: '-50%' }"
                :animate="{ opacity: 1, y: 0, x: '-50%' }"
                :exit="{ opacity: 0, y: -10, x: '-50%' }"
              >
                <input 
                  v-model="refinementInstruction" 
                  :placeholder="isMarkdown ? 'Refine document...' : 'Modify diagram (e.g. \'Change to LR layout\')...'"
                  @keyup.enter="refineWithAi"
                />
                <button @click="refineWithAi" :disabled="isRefining">
                  <Loader2 v-if="isRefining" :size="14" class="spinner" />
                  <span v-else>→</span>
                </button>
              </Motion>
            </AnimatePresence>
          </div>
        </section>

        <!-- Preview Section -->
        <section class="preview-section" v-if="isPreviewVisible">
          <!-- Loading Overlay (Scoped to Preview) -->
          <AnimatePresence>
            <Motion
              v-if="isFixing"
              :initial="{ opacity: 0 }"
              :animate="{ opacity: 1 }"
              :exit="{ opacity: 0 }"
              class="loading-overlay"
            >
              <div class="loader-content">
                <RotateCw :size="32" class="spinner" />
                <h3>DEBUGGING...</h3>
              </div>
            </Motion>
          </AnimatePresence>

          <div class="pane-header">
            <div class="pane-header-left">
              <Layout :size="14" />
              <span>{{ isMarkdown ? 'DOCUMENT PREVIEW' : 'DIAGRAM PREVIEW' }}</span>
            </div>
            <div class="pane-header-actions" v-if="!isMarkdown && diagramSvg">
              <!-- Export as SVG (Guaranteed success) -->
              <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'export-svg'" @mouseleave="activeTooltip = null">
                <button class="action-btn-inline" @click="downloadAsSvg">
                  <FileCode :size="14" />
                </button>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'export-svg'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    class="floating-message tooltip-bottom-left"
                  >
                    Export as SVG
                  </Motion>
                </AnimatePresence>
              </div>

              <!-- Export as PNG (Resolution picker) -->
              <div class="btn-tooltip-wrapper" @mouseenter="activeTooltip = 'export-png'" @mouseleave="activeTooltip = null">
                <button class="action-btn-inline" @click="openExportModal" :disabled="isExporting">
                  <ImageDown v-if="!isExporting" :size="14" />
                  <RotateCw v-else :size="14" class="spinner" />
                </button>
                <AnimatePresence>
                  <Motion
                    v-if="activeTooltip === 'export-png'"
                    :initial="{ opacity: 0, y: 5, scale: 0.9 }"
                    :animate="{ opacity: 1, y: 0, scale: 1 }"
                    :exit="{ opacity: 0, y: 5, scale: 0.9 }"
                    class="floating-message tooltip-bottom-left"
                  >
                    Export as PNG
                  </Motion>
                </AnimatePresence>
              </div>
            </div>
          </div>
          <div class="preview-wrapper" ref="previewContainer" :class="{ 'markdown-view': isMarkdown }">
             <div v-if="isMarkdown" class="markdown-body" v-html="markdownHtml"></div>
             <div v-else-if="diagramSvg" v-html="diagramSvg" class="svg-container"></div>
             <div v-else class="empty-preview">
                <Workflow :size="48" />
                <h3>No content rendered</h3>
                <p>Enter Mermaid or Markdown code to see the preview.</p>
             </div>
          </div>
        </section>
      </div>

      <!-- Error Console -->
      <AnimatePresence>
        <Motion
          v-if="renderingError"
          :initial="{ y: 100, opacity: 0 }"
          :animate="{ y: 0, opacity: 1 }"
          :exit="{ y: 100, opacity: 0 }"
          class="error-console"
        >
          <div class="console-header">
            <div class="title">
              <X :size="14" class="error-icon" />
              <span>RENDERING ERROR</span>
            </div>
            <div class="console-actions">
              <button class="action-btn-inline" @click="handleCopyError" :title="isCopyingError ? 'Copied!' : 'Copy to Clipboard'">
                <Check v-if="isCopyingError" :size="14" class="success-icon" />
                <Copy v-else :size="14" />
              </button>
              <button class="action-btn-inline close-btn" @click="renderingError = null">
                <X :size="14" />
              </button>
            </div>
          </div>
          <div class="error-logs-container">
            <pre class="error-logs">{{ renderingError }}</pre>
          </div>
        </Motion>
      </AnimatePresence>
    </main>

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
            <h3>Gallery</h3>
            <button @click="isTemplatesVisible = false"><X :size="18" /></button>
          </div>
          <div class="template-grid">
            <div v-for="temp in diagramTemplates" :key="temp.name" class="template-card" @click="useTemplate(temp)">
              <div class="temp-icon">
                <Workflow v-if="temp.type === 'mmd'" :size="32" />
                <FileCode v-else :size="32" />
              </div>
              <h4>{{ temp.name }}</h4>
              <p>{{ temp.description }}</p>
            </div>
          </div>
        </Motion>
      </Motion>
    </AnimatePresence>

    <!-- Export Resolution Modal -->
    <AnimatePresence>
      <Motion
        v-if="isExportModalVisible"
        :initial="{ opacity: 0 }"
        :animate="{ opacity: 1 }"
        :exit="{ opacity: 0 }"
        class="modal-backdrop"
        @click="!isExporting && (isExportModalVisible = false)"
      >
        <Motion
          :initial="{ scale: 0.92, opacity: 0, y: 20 }"
          :animate="{ scale: 1, opacity: 1, y: 0 }"
          :exit="{ scale: 0.92, opacity: 0, y: 20 }"
          :transition="{ duration: 0.2 }"
          class="export-modal"
          @click.stop
        >
          <div class="export-modal-header">
            <div class="export-modal-title">
              <Maximize2 :size="18" />
              <h3>Export Resolution</h3>
            </div>
            <button @click="!isExporting && (isExportModalVisible = false)" :disabled="isExporting">
              <X :size="18" />
            </button>
          </div>

          <div class="export-presets-grid">
            <button
              v-for="preset in exportPresets"
              :key="preset.id"
              class="export-preset-card"
              :class="{ active: selectedExportPreset === preset.id, dangerous: preset.id === selectedExportPreset && isExportDangerous }"
              @click="selectedExportPreset = preset.id"
              :disabled="isExporting"
            >
              <span v-if="preset.badge" class="preset-badge" :class="{ 'badge-ultra': preset.badge === 'Ultra' || preset.badge === 'Max' }">{{ preset.badge }}</span>
              <span class="preset-label">{{ preset.label }}</span>
              <span class="preset-sublabel">{{ preset.sublabel }}</span>
            </button>
          </div>

          <!-- Custom width input -->
          <div v-if="selectedExportPreset === 'custom'" class="custom-width-row">
            <label>Target Width (px)</label>
            <input
              type="number"
              v-model.number="customExportWidth"
              min="100"
              max="65536"
              step="100"
              :disabled="isExporting"
            />
          </div>

          <!-- Dimension preview -->
          <div class="export-dimension-preview">
            <div class="dim-item">
              <span class="dim-label">Output</span>
              <span class="dim-value">{{ estimatedExportDimensions.width.toLocaleString() }} × {{ estimatedExportDimensions.height.toLocaleString() }}</span>
            </div>
            <div class="dim-item">
              <span class="dim-label">Megapixels</span>
              <span class="dim-value">{{ estimatedExportDimensions.megapixels }} MP</span>
            </div>
          </div>

          <!-- Warning for ultra-high-res -->
          <div v-if="isExportDangerous" class="export-warning">
            <span>⚠</span>
            <p>This resolution exceeds typical browser canvas limits (16384px). Export may fail or produce a blank image on some systems. Consider using <strong>SVG export</strong> for lossless, infinitely scalable output.</p>
          </div>

          <!-- Progress -->
          <div v-if="exportProgress" class="export-progress">
            <RotateCw :size="14" class="spinner" />
            <span>{{ exportProgress }}</span>
          </div>

          <!-- Actions -->
          <div class="export-modal-actions">
            <button class="export-cancel-btn" @click="isExportModalVisible = false" :disabled="isExporting">Cancel</button>
            <button class="export-confirm-btn" @click="downloadAsPng" :disabled="isExporting">
              <Download :size="16" />
              <span>{{ isExporting ? 'Exporting...' : 'Export PNG' }}</span>
            </button>
          </div>
        </Motion>
      </Motion>
    </AnimatePresence>
  </div>
</template>

<style scoped>
.studio-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg);
}

.studio-header {
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

.auto-compile-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
}

.auto-compile-toggle input {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--accent);
}

.info-icon {
  color: var(--muted);
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

.mode-toggle {
  width: auto;
  padding: 0 10px;
  gap: 8px;
}

.mode-label {
  font-size: 0.65rem;
  font-weight: 800;
  color: var(--accent);
}

.render-btn {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.save-btn {
  width: auto;
  padding: 0 12px;
  gap: 8px;
  border-color: var(--accent-soft);
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 0.75rem;
  font-weight: 700;
}

.save-btn span {
  display: inline;
}

.save-btn:hover:not(:disabled) {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.ai-fix-btn {
  width: auto;
  padding: 0 12px;
  gap: 8px;
  color: #a371f7;
  border-color: rgba(163, 113, 247, 0.3);
  font-size: 0.75rem;
  font-weight: 700;
}

.ai-fix-btn:hover:not(:disabled) {
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

.studio-main {
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

.workspace-sidebar {
  background: var(--bg-accent);
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  min-width: 150px;
  max-width: 500px;
}

.sidebar-resizer {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  transition: background 0.2s;
  z-index: 10;
  margin-left: -2px;
}

.sidebar-resizer:hover, .sidebar-resizer:active {
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

.file-tree {
  flex: 1;
  overflow-y: auto;
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

.item-icon {
  display: flex;
  align-items: center;
  color: var(--muted);
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

.tree-children {
  padding-left: 12px;
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

.mermaid-editor-cm {
  flex: 1;
  width: 100%;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
}

:deep(.cm-editor) {
  height: 100%;
  outline: none !important;
}

.refinement-bar {
  position: absolute;
  top: 16px;
  left: 50%;
  width: 90%;
  max-width: 440px;
  background: var(--surface-soft);
  border: 1px solid var(--accent-soft);
  border-radius: 20px;
  display: flex;
  padding: 4px 14px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.5);
  z-index: 20;
}

.refinement-bar input {
  flex: 1;
  background: none;
  border: none;
  color: var(--ink);
  font-size: 0.75rem;
  padding: 8px 0;
  outline: none;
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

.preview-section {
  flex: 1.5;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--bg);
}

.preview-wrapper {
  flex: 1;
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-wrapper.markdown-view {
  display: block;
  overflow-y: auto;
  padding: 24px;
  background: var(--bg-accent);
}

:deep(.markdown-body) {
  background: transparent !important;
  color: var(--ink) !important;
  font-family: var(--font-family) !important;
}

:deep(.markdown-body p), :deep(.markdown-body li), :deep(.markdown-body span) {
  color: var(--ink) !important;
}

:deep(.markdown-body h1), :deep(.markdown-body h2), :deep(.markdown-body h3), :deep(.markdown-body h4), :deep(.markdown-body h5), :deep(.markdown-body h6) {
  color: var(--ink) !important;
  border-bottom-color: var(--line) !important;
}

:deep(.markdown-body pre) {
  background-color: var(--surface) !important;
  border: 1px solid var(--line) !important;
}

:deep(.markdown-body code) {
  color: var(--accent) !important;
  background-color: var(--surface) !important;
}

:deep(.markdown-body a) {
  color: var(--accent) !important;
}

.svg-container {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

:deep(svg) {
  max-width: 100%;
  max-height: 100%;
}

/* svg-pan-zoom controls theming */
:deep(#svg-pan-zoom-controls) {
  fill: var(--muted);
  fill-opacity: 0.8;
}

:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-background) {
  fill: var(--surface) !important;
  fill-opacity: 0.9;
  stroke: var(--line);
  stroke-width: 1px;
}

:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-element:hover .svg-pan-zoom-control-background) {
  fill: var(--bg-accent) !important;
  stroke: var(--accent);
}

:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-element path),
:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-element polygon) {
  fill: var(--ink);
}

:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-element:hover path),
:deep(#svg-pan-zoom-controls .svg-pan-zoom-control-element:hover polygon) {
  fill: var(--accent);
}

:deep(.mermaid-rendered-wrapper) {
  margin: 20px 0;
  background: var(--surface);
  border: 1px solid var(--line);
  padding: 16px;
  border-radius: 8px;
  display: flex;
  justify-content: center;
}

.empty-preview {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: var(--muted);
  gap: 16px;
}

.render-overlay {
  position: absolute;
  top: 12px;
  right: 12px;
  background: var(--surface-soft);
  padding: 8px;
  border-radius: 50%;
  box-shadow: 0 4px 12px rgba(0,0,0,0.2);
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
  animation: spin 1s linear infinite;
  color: var(--accent);
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

/* ===== Export Resolution Modal ===== */
.export-modal {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
  width: 92%;
  max-width: 520px;
  padding: 24px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5);
}

.export-modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.export-modal-title {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--accent);
}

.export-modal-title h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
  color: var(--ink);
}

.export-modal-header > button {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
  transition: 0.15s;
}

.export-modal-header > button:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.export-presets-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.export-preset-card {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 14px 8px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: var(--surface-soft);
  color: var(--ink);
  cursor: pointer;
  transition: all 0.2s ease;
  gap: 4px;
}

.export-preset-card:hover:not(:disabled) {
  border-color: var(--muted);
  background: var(--surface);
  transform: translateY(-1px);
}

.export-preset-card.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  box-shadow: 0 0 0 1px var(--accent), 0 4px 16px rgba(88, 166, 255, 0.15);
}

.export-preset-card.dangerous {
  border-color: var(--warning);
  box-shadow: 0 0 0 1px var(--warning), 0 4px 16px rgba(255, 166, 87, 0.15);
}

.preset-badge {
  position: absolute;
  top: -7px;
  right: -4px;
  font-size: 0.55rem;
  font-weight: 800;
  letter-spacing: 0.04em;
  padding: 2px 6px;
  border-radius: 6px;
  background: var(--accent);
  color: white;
  line-height: 1.2;
}

.preset-badge.badge-ultra {
  background: linear-gradient(135deg, #a371f7, #f778ba);
}

.preset-label {
  font-size: 0.8rem;
  font-weight: 700;
}

.preset-sublabel {
  font-size: 0.6rem;
  color: var(--muted);
  text-align: center;
  line-height: 1.3;
}

.custom-width-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 12px 14px;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  border-radius: 10px;
}

.custom-width-row label {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--muted);
  white-space: nowrap;
}

.custom-width-row input {
  flex: 1;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--ink);
  font-size: 0.85rem;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  outline: none;
  transition: border-color 0.15s;
}

.custom-width-row input:focus {
  border-color: var(--accent);
}

.export-dimension-preview {
  display: flex;
  gap: 16px;
  padding: 12px 14px;
  background: var(--bg);
  border: 1px solid var(--line);
  border-radius: 10px;
  margin-bottom: 12px;
}

.dim-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.dim-label {
  font-size: 0.6rem;
  font-weight: 700;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.dim-value {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ink);
  font-family: 'JetBrains Mono', monospace;
}

.export-warning {
  display: flex;
  gap: 10px;
  padding: 10px 14px;
  background: rgba(255, 166, 87, 0.08);
  border: 1px solid rgba(255, 166, 87, 0.25);
  border-radius: 10px;
  margin-bottom: 12px;
  align-items: flex-start;
}

.export-warning span {
  font-size: 1rem;
  flex-shrink: 0;
  line-height: 1.4;
}

.export-warning p {
  margin: 0;
  font-size: 0.7rem;
  color: var(--muted);
  line-height: 1.5;
}

.export-warning strong {
  color: var(--ink);
}

.export-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background: var(--accent-soft);
  border-radius: 10px;
  margin-bottom: 12px;
}

.export-progress span {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--accent);
}

.export-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 4px;
}

.export-cancel-btn {
  padding: 8px 18px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--surface-soft);
  color: var(--muted);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: 0.15s;
}

.export-cancel-btn:hover:not(:disabled) {
  background: var(--surface);
  color: var(--ink);
}

.export-confirm-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 20px;
  border-radius: 8px;
  border: none;
  background: var(--accent);
  color: white;
  font-size: 0.8rem;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s ease;
}

.export-confirm-btn:hover:not(:disabled) {
  filter: brightness(1.15);
  transform: translateY(-1px);
  box-shadow: 0 4px 16px rgba(88, 166, 255, 0.3);
}

.export-confirm-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 1024px) {
  .workspace-sidebar {
    width: 200px;
  }
  
  .export-presets-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (max-width: 768px) {
  .split-pane {
    flex-direction: column;
  }
  
  .workspace-sidebar {
    width: 100%;
    height: 180px;
    border-right: none;
    border-bottom: 1px solid var(--line);
  }
  
  .editor-section {
    border-right: none;
    border-bottom: 1px solid var(--line);
    flex: 1;
  }
  
  .preview-section {
    flex: 1.2;
  }
  
  .export-presets-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
