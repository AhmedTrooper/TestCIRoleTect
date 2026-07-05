<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Motion, AnimatePresence } from "motion-v";
import { openUrl } from "@tauri-apps/plugin-opener";
import Titlebar from "./components/Titlebar.vue";
import SplashLoader from "./components/SplashLoader.vue";
import CustomDialog from "./components/CustomDialog.vue";
import CloudUploadOverlay from "./components/CloudUploadOverlay.vue";
import { useSettingsStore } from "./store/settings";
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { exit } from '@tauri-apps/plugin-process';
import {
    Home,
    Briefcase,
    FileText,
    Mail,
    Settings,
    Code,
    Video,
    Cpu,
    Info,
    Share2,
    Inbox,
} from "@lucide/vue";

const tabs = [
    { path: "/", label: "Home", icon: Home },
    { path: "/jobs", label: "Jobs", icon: Briefcase },
    { path: "/inbox", label: "Inbox", icon: Inbox },
    { path: "/resumes", label: "Resume Templates", icon: FileText },
    { path: "/cover-letters", label: "CL Templates", icon: Mail },
    { path: "/compiler", label: "Compiler", icon: Cpu },
    { path: "/diagrams", label: "Diagrams", icon: Share2 },
    { path: "/settings", label: "Settings", icon: Settings },
    { path: "/about", label: "About", icon: Info },
];

const externalLinks = [
    {
        url: "https://github.com/AhmedTrooper/Roletect",
        label: "Source",
        icon: Code,
    },
    {
        url: "https://www.youtube.com/@AhmedTrooper",
        label: "YouTube",
        icon: Video,
    },
];

const settingsStore = useSettingsStore();
const activeTooltip = ref<string | null>(null);
const isAppLoading = ref(true);
const isUploadingToCloud = ref(false);

onMounted(async () => {
    try {
        // Load settings (database, stronghold, etc.)
        await settingsStore.loadSettings();
    } catch (error) {
        console.error("Initialization error:", error);
    } finally {
        isAppLoading.value = false;
    }
    
    // Intercept window close for cloud backup
    const appWindow = getCurrentWindow();
    appWindow.onCloseRequested(async (event) => {
        // Prevent immediate close
        event.preventDefault();
        
        try {
            // Check if dirty
            const isDirty = await invoke<boolean>('check_data_dirty');
            if (!isDirty) {
                await exit(0);
                return;
            }
            
            // Check if user wants auto local backup
            const autoLocal = await invoke<string>('get_setting', { key: 'auto_local_backup', default_value: 'true' });
            if (autoLocal === 'true') {
                try {
                    await invoke('auto_local_backup');
                } catch (e) {
                    console.error("Local backup failed:", e);
                }
            }
            
            // Check if S3 is setup correctly AND auto cloud backup is enabled
            const isSetupOk = await invoke<string>('get_setting', { key: 's3_setup_ok', default_value: 'false' });
            const autoCloud = await invoke<string>('get_setting', { key: 'auto_cloud_backup', default_value: 'true' });
            if (isSetupOk !== 'true' || autoCloud !== 'true') {
                await exit(0);
                return;
            }
            
            // Show overlay
            isUploadingToCloud.value = true;
            
            // Get credentials from Stronghold
            const ak = (await settingsStore.getSecret('s3_access_key')) || '';
            const sk = (await settingsStore.getSecret('s3_secret_key')) || '';
            
            // Call upload command
            await invoke('upload_backup_to_s3', {
                accessKeyId: ak,
                secretAccessKey: sk
            });
            
            // Brief pause so the user sees it actually completed if it was too fast
            setTimeout(async () => {
                await exit(0);
            }, 600);
            
        } catch (e) {
            console.error("Cloud backup on close failed:", e);
            await exit(0);
        }
    });
});

onMounted(() => {
    document.addEventListener("contextmenu", (e: MouseEvent) => {
        const target = e.target;

        // 1. Ensure target is not null and is an HTML element
        if (target instanceof HTMLElement) {
            // 2. TypeScript now safely recognizes .tagName and .isContentEditable
            if (
                target.tagName === "INPUT" ||
                target.tagName === "TEXTAREA" ||
                target.isContentEditable
            ) {
                return;
            }
        }

        // Block the browser context menu everywhere else
        e.preventDefault();
    });
});

const handleExternalClick = (url: string) => {
    openUrl(url).catch((err: any) => console.error("Failed to open URL:", err));
};
</script>

<template>
    <AnimatePresence>
        <SplashLoader v-if="isAppLoading" key="loader" />
    </AnimatePresence>

    <Titlebar />
    <div class="app-container select-none" @dblclick.prevent>
        <aside class="sidebar">
            <nav class="nav-menu">
                <router-link
                    v-for="tab in tabs"
                    :key="tab.path"
                    :to="tab.path"
                    class="nav-item"
                    active-class="active"
                    @mouseenter="activeTooltip = tab.label"
                    @mouseleave="activeTooltip = null"
                >
                    <div class="icon-wrapper">
                        <component :is="tab.icon" :size="20" stroke-width="2" />
                        <AnimatePresence>
                            <Motion
                                v-if="activeTooltip === tab.label"
                                :initial="{ opacity: 0, x: 5, scale: 0.9 }"
                                :animate="{ opacity: 1, x: 12, scale: 1 }"
                                :exit="{ opacity: 0, x: 5, scale: 0.9 }"
                                :transition="{ duration: 0.15 }"
                                class="flying-message sidebar-tooltip"
                            >
                                {{ tab.label }}
                            </Motion>
                        </AnimatePresence>
                    </div>
                </router-link>

                <div class="nav-divider"></div>

                <button
                    v-for="link in externalLinks"
                    :key="link.url"
                    @click="handleExternalClick(link.url)"
                    class="nav-item external"
                    @mouseenter="activeTooltip = link.label"
                    @mouseleave="activeTooltip = null"
                >
                    <div class="icon-wrapper">
                        <component
                            :is="link.icon"
                            :size="20"
                            stroke-width="2"
                        />
                        <AnimatePresence>
                            <Motion
                                v-if="activeTooltip === link.label"
                                :initial="{ opacity: 0, x: 5, scale: 0.9 }"
                                :animate="{ opacity: 1, x: 12, scale: 1 }"
                                :exit="{ opacity: 0, x: 5, scale: 0.9 }"
                                :transition="{ duration: 0.15 }"
                                class="flying-message sidebar-tooltip"
                            >
                                {{ link.label }}
                            </Motion>
                        </AnimatePresence>
                    </div>
                </button>
            </nav>
        </aside>

        <main class="content-area">
            <router-view v-slot="{ Component, route }">
                <transition mode="out-in">
                    <Motion
                        :key="route.path"
                        :initial="{ opacity: 0, y: 5 }"
                        :animate="{ opacity: 1, y: 0 }"
                        :transition="{ duration: 0.15, ease: 'easeOut' }"
                        class="route-wrapper"
                    >
                        <component :is="Component" />
                    </Motion>
                </transition>
            </router-view>
        </main>
    </div>

    <!-- Global Bespoke Dialog System -->
    <CustomDialog />
    
    <!-- Cloud Upload Overlay -->
    <CloudUploadOverlay :is-visible="isUploadingToCloud" />
</template>

<style scoped>
.app-container {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 36px);
    margin-top: 36px;
    width: 100%;
    background: var(--bg);
    color: var(--ink);
    overflow: hidden;
}

.desktop-only {
    display: flex !important;
}

.mobile-only {
    display: none !important;
}

.sidebar {
    order: 2;
    background: var(--bg-accent);
    border-top: 1px solid var(--line);
    z-index: 100;
    display: flex;
    align-items: center;
    padding: 0 8px;
}

.logo-section {
    padding-right: 12px;
    border-right: 1px solid var(--line);
    margin-right: 4px;
}

.logo-dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    box-shadow: 0 0 8px var(--accent);
}

.nav-menu {
    display: flex;
    width: 100%;
    padding: 2px 0;
    overflow-x: auto;
    scrollbar-width: none; /* Firefox */
    -ms-overflow-style: none; /* IE and Edge */
    gap: 2px;
}

.nav-menu::-webkit-scrollbar {
    display: none; /* Chrome, Safari, Opera */
}

.nav-item {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 8px 12px;
    color: var(--muted);
    text-decoration: none;
    transition: 0.15s;
    background: none;
    border: none;
    cursor: pointer;
    flex-shrink: 0;
}

.nav-divider {
    width: 1px;
    height: 20px;
    background: var(--line);
    margin: 0 8px;
    flex-shrink: 0;
}

.nav-item.external {
    opacity: 0.8;
}

.nav-item.external:hover {
    opacity: 1;
    color: var(--accent);
}

.nav-item:hover {
    color: var(--ink);
}

.nav-item.active {
    color: var(--accent);
}

.icon-wrapper {
    font-size: 1.2rem;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    display: none; /* Hidden by default, shown on desktop */
}

.flying-message::after {
    content: "";
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border: 4px solid transparent;
    border-top-color: var(--accent);
}

.sidebar-tooltip {
    left: 100%;
    top: 50%;
    bottom: auto;
    transform: translateY(-50%);
    margin-left: 12px;
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

.content-area {
    flex: 1;
    overflow: hidden;
    position: relative;
}

.route-wrapper {
    height: 100%;
    width: 100%;
    overflow-y: auto;
    overflow-x: hidden;
}

@media (max-width: 959px) {
    .nav-divider {
        display: none;
    }
    .nav-item {
        padding: 8px 10px;
    }
    .icon-wrapper {
        font-size: 1rem;
    }
}

@media (min-width: 960px) {
    .flying-message {
        display: block;
    }
    .app-container {
        flex-direction: row;
    }

    .sidebar {
        order: 0;
        width: 48px;
        height: calc(100vh - 36px);
        flex-direction: column;
        border-top: none;
        border-right: 1px solid var(--line);
        padding: 12px 0;
        align-items: center;
    }

    .nav-menu {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 0;
        overflow-x: visible;
    }

    .nav-divider {
        display: block;
        width: auto;
        height: 1px;
        background: var(--line);
        margin: 8px 12px;
    }

    .nav-item {
        width: 100%;
        padding: 8px 0;
        position: relative;
    }

    .nav-label {
        display: none;
    }

    .nav-item.active::before {
        content: "";
        position: absolute;
        left: 0;
        top: 8px;
        bottom: 8px;
        width: 2px;
        background: var(--accent);
    }

    .icon-wrapper {
        font-size: 1.1rem;
    }
}
</style>
