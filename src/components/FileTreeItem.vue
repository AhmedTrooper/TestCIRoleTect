<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { 
  ChevronRight, 
  ChevronDown, 
  File, 
  Plus, 
  FolderPlus, 
  Trash2, 
  Star,
  FileCode,
  Pencil,
  ExternalLink
} from '@lucide/vue';
import { Motion, AnimatePresence } from 'motion-v';

interface FileItem {
  name: string;
  path: string;
  isDir: boolean;
  children?: FileItem[];
  isOpen?: boolean;
}

defineProps<{
  item: FileItem;
  activeFilePath: string | null;
  mainFilePath?: string | null;
  isDiagram?: boolean;
  level?: number;
  onToggle: (item: FileItem) => void;
  onSelect: (item: FileItem) => void;
  onSetMain?: (path: string) => void;
  onCreateFile: (parent: FileItem, ext?: string) => void;
  onCreateFolder: (parent: FileItem) => void;
  onDelete: (item: FileItem) => void;
  onRename?: (item: FileItem) => void;
}>();

const showContextMenu = ref(false);
const contextMenuPos = ref({ x: 0, y: 0 });

const handleContextMenu = (e: MouseEvent) => {
  e.preventDefault();
  e.stopPropagation();
  contextMenuPos.value = { x: e.clientX, y: e.clientY };
  showContextMenu.value = true;
};

const closeContextMenu = () => {
  showContextMenu.value = false;
};

const handleWindowClick = () => {
  if (showContextMenu.value) {
    closeContextMenu();
  }
};

onMounted(() => {
  window.addEventListener('click', handleWindowClick);
  window.addEventListener('contextmenu', handleWindowClick);
});

onUnmounted(() => {
  window.removeEventListener('click', handleWindowClick);
  window.removeEventListener('contextmenu', handleWindowClick);
});

const executeAction = (action: () => void) => {
  closeContextMenu();
  action();
};
</script>

<template>
  <div class="tree-item-wrapper">
    <div 
      class="tree-item" 
      :class="{ 
        active: activeFilePath === item.path, 
        'main-file': mainFilePath === item.path 
      }"
      :style="{ paddingLeft: `${((level || 0) * 12) + 12}px` }"
      @click="item.isDir ? onToggle(item) : onSelect(item)"
      @contextmenu="handleContextMenu"
    >
      <div class="item-icon">
        <template v-if="item.isDir">
          <ChevronRight v-if="!item.isOpen" :size="14" />
          <ChevronDown v-else :size="14" />
        </template>
        <File v-else :size="14" />
      </div>
      
      <span class="item-name">{{ item.name }}</span>
      
      <!-- Main File indicator star -->
      <div v-if="!item.isDir && mainFilePath === item.path" class="main-file-indicator">
        <Star :size="12" fill="var(--accent)" color="var(--accent)" />
      </div>
    </div>

    <!-- Right Click Context Menu -->
    <Teleport to="body">
      <AnimatePresence>
        <Motion
          v-if="showContextMenu"
          :initial="{ opacity: 0, scale: 0.95 }"
          :animate="{ opacity: 1, scale: 1 }"
          :exit="{ opacity: 0, scale: 0.95 }"
          :transition="{ duration: 0.1 }"
          class="context-menu"
          :style="{ top: `${contextMenuPos.y}px`, left: `${contextMenuPos.x}px` }"
          @click.stop
        >
          <!-- Set as Main File (Files only) -->
          <button 
            v-if="!item.isDir && onSetMain" 
            class="context-menu-item"
            @click="executeAction(() => onSetMain!(item.path))"
          >
            <Star :size="14" :fill="mainFilePath === item.path ? 'var(--accent)' : 'none'" />
            <span>{{ mainFilePath === item.path ? 'Unset Main File' : 'Set as Main File' }}</span>
          </button>

          <!-- Create File/Folder (Folders only) -->
          <template v-if="item.isDir">
            <button 
              v-if="!isDiagram" 
              class="context-menu-item" 
              @click="executeAction(() => onCreateFile(item))"
            >
              <Plus :size="14" />
              <span>New File</span>
            </button>
            <template v-else>
              <button 
                class="context-menu-item" 
                @click="executeAction(() => onCreateFile(item, '.mmd'))"
              >
                <Plus :size="14" />
                <span>New Diagram</span>
              </button>
              <button 
                class="context-menu-item" 
                @click="executeAction(() => onCreateFile(item, '.md'))"
              >
                <FileCode :size="14" />
                <span>New Markdown</span>
              </button>
            </template>

            <button 
              class="context-menu-item" 
              @click="executeAction(() => onCreateFolder(item))"
            >
              <FolderPlus :size="14" />
              <span>New Folder</span>
            </button>
            <div class="context-menu-divider"></div>
          </template>

          <!-- Rename -->
          <button 
            v-if="onRename" 
            class="context-menu-item" 
            @click="executeAction(() => onRename!(item))"
          >
            <Pencil :size="14" />
            <span>Rename</span>
          </button>

          <!-- Reveal in Explorer -->
          <button 
            class="context-menu-item" 
            @click="executeAction(() => revealItemInDir(item.path))"
          >
            <ExternalLink :size="14" />
            <span>Reveal in System Explorer</span>
          </button>

          <!-- Delete -->
          <button 
            class="context-menu-item item-delete" 
            @click="executeAction(() => onDelete(item))"
          >
            <Trash2 :size="14" />
            <span>Delete</span>
          </button>
        </Motion>
      </AnimatePresence>
    </Teleport>

    <AnimatePresence>
      <Motion
        v-if="item.isDir && item.isOpen"
        :initial="{ height: 0, opacity: 0 }"
        :animate="{ height: 'auto', opacity: 1 }"
        :exit="{ height: 0, opacity: 0 }"
        class="tree-children"
      >
        <!-- Recursion -->
        <FileTreeItem 
          v-for="child in item.children" 
          :key="child.path"
          :item="child"
          :level="(level || 0) + 1"
          :active-file-path="activeFilePath"
          :main-file-path="mainFilePath"
          :is-diagram="isDiagram"
          :on-toggle="onToggle"
          :on-select="onSelect"
          :on-set-main="onSetMain"
          :on-create-file="onCreateFile"
          :on-create-folder="onCreateFolder"
          :on-delete="onDelete"
          :on-rename="onRename"
        />
      </Motion>
    </AnimatePresence>
  </div>
</template>

<style scoped>
.tree-item-wrapper {
  display: flex;
  flex-direction: column;
  min-width: 100%;
  width: max-content;
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
  min-width: 100%;
  width: max-content;
  box-sizing: border-box;
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
  flex-shrink: 0;
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

.main-file-indicator {
  display: flex;
  align-items: center;
  margin-left: auto;
  flex-shrink: 0;
}

.tree-children {
  padding-left: 0;
  overflow: hidden;
}
</style>

<style>
/* Global styling for teleported context menu matching global theme system */
.context-menu {
  position: fixed;
  z-index: 99999;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius-md);
  padding: 4px;
  min-width: 160px;
  box-shadow: var(--shadow);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 10px;
  background: none;
  border: none;
  color: var(--ink);
  font-size: 0.8rem;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  text-align: left;
}

.context-menu-item:hover {
  background: var(--surface-soft);
  color: var(--ink);
}

.context-menu-item.item-delete:hover {
  background: rgba(248, 81, 73, 0.15);
  color: var(--warning);
}

.context-menu-divider {
  height: 1px;
  background: var(--line);
  margin: 2px 0;
}
</style>
