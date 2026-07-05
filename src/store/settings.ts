import { defineStore } from 'pinia';
import { ref, shallowRef } from 'vue';
import { Stronghold, Store } from '@tauri-apps/plugin-stronghold';
import { appDataDir, join } from '@tauri-apps/api/path';
import { readTextFile, writeTextFile, remove, exists } from '@tauri-apps/plugin-fs';
import { invoke } from '@tauri-apps/api/core';

export interface Theme {
  id: string;
  name: string;
  config: string;
  is_builtin: boolean;
}

export const useSettingsStore = defineStore('settings', () => {
  const hasSecureKey = ref(false);
  const selectedAiProvider = ref('openai'); 
  const selectedAiModel = ref('gpt-4o');
  
  const availableThemes = ref<Theme[]>([]);
  const activeThemeId = ref('github-dark');

  // Font Settings
  const fontFamily = ref('Inter');
  const fontSize = ref(14);
  const fontWeight = ref('400');
  const fontStyle = ref('normal');

  const isAutoCompileEnabled = ref(true);

  // Cache Stronghold and Store instances
  const strongholdInstance = shallowRef<Stronghold | null>(null);
  const storeInstance = shallowRef<Store | null>(null);

  const applyTheme = (themeConfig: string) => {
    try {
      const colors = JSON.parse(themeConfig);
      const root = document.documentElement;
      Object.entries(colors).forEach(([key, value]) => {
        root.style.setProperty(key, value as string);
      });
    } catch (e) {
      console.error("Failed to apply theme:", e);
    }
  };

  const applyFonts = () => {
    const root = document.documentElement;
    const family = fontFamily.value.includes(' ') ? `"${fontFamily.value}"` : fontFamily.value;
    root.style.setProperty('--font-family', `${family}, sans-serif`);
    root.style.setProperty('--font-size', `${fontSize.value}px`);
    root.style.setProperty('--font-weight', fontWeight.value);
    root.style.setProperty('--font-style', fontStyle.value);
  };

  const loadThemes = async () => {
    try {
      availableThemes.value = await invoke<Theme[]>('get_all_themes');
      const active: Theme = await invoke('get_active_theme');
      activeThemeId.value = active.id;
      applyTheme(active.config);
    } catch (e) {
      console.error("Error loading themes:", e);
    }
  };

  const setTheme = async (themeId: string) => {
    try {
      await invoke('save_active_theme', { themeId });
      activeThemeId.value = themeId;
      const theme = availableThemes.value.find(t => t.id === themeId);
      if (theme) {
        applyTheme(theme.config);
      }
    } catch (e) {
      console.error("Error setting theme:", e);
    }
  };

  const setFontFamily = async (family: string) => {
    fontFamily.value = family;
    applyFonts();
    await invoke('save_setting', { key: 'font_family', value: family });
  };

  const setFontSize = async (size: number) => {
    fontSize.value = size;
    applyFonts();
    await invoke('save_setting', { key: 'font_size', value: size.toString() });
  };

  const setFontWeight = async (weight: string) => {
    fontWeight.value = weight;
    applyFonts();
    await invoke('save_setting', { key: 'font_weight', value: weight });
  };

  const setFontStyle = async (style: string) => {
    fontStyle.value = style;
    applyFonts();
    await invoke('save_setting', { key: 'font_style', value: style });
  };

  const setAutoCompile = async (enabled: boolean) => {
    isAutoCompileEnabled.value = enabled;
    await invoke('save_setting', { key: 'auto_compile', value: enabled.toString() });
  };

  const resetTypography = async () => {
    await setFontFamily('Inter');
    await setFontSize(14);
    await setFontWeight('400');
    await setFontStyle('normal');
  };

  const importCustomTheme = async (themeJson: string) => {
    try {
      const parsed = JSON.parse(themeJson);
      if (!parsed.name || !parsed.colors) throw new Error("Invalid theme format. Missing 'name' or 'colors'.");
      
      // Check if name already exists (Uniqueness Requirement)
      if (availableThemes.value.some(t => t.name.toLowerCase() === parsed.name.toLowerCase())) {
        throw new Error(`A theme with the name "${parsed.name}" already exists.`);
      }

      // Create a unique slug: name-lowercase + random suffix
      const slugBase = parsed.name.toLowerCase().replace(/\s+/g, '-');
      const randomSuffix = Math.random().toString(36).substring(2, 6);
      const id = `${slugBase}-${randomSuffix}`;
      
      const config = JSON.stringify(parsed.colors);
      
      await invoke('save_custom_theme', { id, name: parsed.name, config });
      await loadThemes();
    } catch (e) {
      console.error("Error importing theme:", e);
      throw e;
    }
  };

  const deleteCustomTheme = async (themeId: string) => {
    try {
      await invoke('delete_theme', { id: themeId });
      await loadThemes();
    } catch (e) {
      console.error("Error deleting theme:", e);
      throw e;
    }
  };

  const generateVaultPassword = () => {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  };

  const getVaultPassword = async () => {
    const dir = await appDataDir();
    const passwordPath = await join(dir, 'stronghold.pass');
    try {
      return (await readTextFile(passwordPath)).trim();
    } catch {
      const password = generateVaultPassword();
      await writeTextFile(passwordPath, password);
      return password;
    }
  };

  let vaultInitPromise: Promise<{ stronghold: Stronghold, store: Store }> | null = null;

  const getVault = async () => {
    if (strongholdInstance.value && storeInstance.value) {
      return { stronghold: strongholdInstance.value, store: storeInstance.value };
    }

    if (vaultInitPromise) {
      return vaultInitPromise;
    }

    vaultInitPromise = (async () => {
      const dir = await appDataDir();
      const vaultPath = await join(dir, 'secrets.stronghold');
      let password = await getVaultPassword();
      let stronghold: Stronghold;

      try {
        stronghold = await Stronghold.load(vaultPath, password);
      } catch (error) {
        console.warn("Failed to load Stronghold, attempting reset:", error);
        // Only reset if it's likely a password issue or corruption
        password = generateVaultPassword();
        const passwordPath = await join(dir, 'stronghold.pass');
        await writeTextFile(passwordPath, password);
        
        if (await exists(vaultPath)) {
          await remove(vaultPath);
        }
        stronghold = await Stronghold.load(vaultPath, password);
      }

      let client;
      try {
        client = await stronghold.loadClient('api_client');
      } catch {
        client = await stronghold.createClient('api_client');
        await stronghold.save();
      }
      
      const store = client.getStore();
      
      strongholdInstance.value = stronghold;
      storeInstance.value = store;

      return { stronghold, store };
    })();

    try {
      return await vaultInitPromise;
    } catch (e) {
      vaultInitPromise = null;
      throw e;
    }
  };

  const saveApiKey = async (provider: string, key: string) => {
    try {
      const { stronghold, store } = await getVault();
      const storageKey = `ai_api_key_${provider}`;
      await store.insert(storageKey, Array.from(new TextEncoder().encode(key)));
      await stronghold.save(); 
      hasSecureKey.value = true;
    } catch (error) {
      console.error("Stronghold save error:", error);
      throw error;
    }
  };

  const getDecryptedKey = async (provider?: string): Promise<string | null> => {
    try {
      const targetProvider = provider || selectedAiProvider.value;
      const { store } = await getVault();
      const storageKey = `ai_api_key_${targetProvider}`;
      const keyBytes = await store.get(storageKey);
      if (keyBytes && keyBytes.length > 0) {
        return new TextDecoder().decode(keyBytes);
      }
      if (targetProvider === 'ollama') {
        return 'ollama';
      }
      return null;
    } catch (error) {
      const targetProvider = provider || selectedAiProvider.value;
      console.error("Stronghold get error:", error);
      if (targetProvider === 'ollama') {
        return 'ollama';
      }
      return null;
    }
  };

  const saveSecret = async (key: string, value: string) => {
    try {
      const { stronghold, store } = await getVault();
      await store.insert(key, Array.from(new TextEncoder().encode(value)));
      await stronghold.save(); 
    } catch (error) {
      console.error("Stronghold save error:", error);
      throw error;
    }
  };

  const getSecret = async (key: string): Promise<string | null> => {
    try {
      const { store } = await getVault();
      const keyBytes = await store.get(key);
      if (keyBytes && keyBytes.length > 0) {
        return new TextDecoder().decode(keyBytes);
      }
      return null;
    } catch (error) {
      console.error("Stronghold get error:", error);
      return null;
    }
  };

  const loadProviderKeyStatus = async (provider: string) => {
    try {
      const { store } = await getVault();
      const storageKey = `ai_api_key_${provider}`;
      const keyBytes = await store.get(storageKey);
      hasSecureKey.value = keyBytes !== null && keyBytes.length > 0;
    } catch (error) {
      console.error("Error loading key status:", error);
      hasSecureKey.value = false;
    }
  };

  const saveModelConfig = async (provider: string, model: string, customBaseUrlVal?: string, customModelVal?: string) => {
    try {
      await invoke('save_model_pref', { provider, model });
      await invoke('save_setting', { key: `${provider}_custom_base_url`, value: customBaseUrlVal || '' });
      await invoke('save_setting', { key: `${provider}_custom_model`, value: customModelVal || '' });
      
      const config: { provider: string, model: string } = await invoke('get_model_pref');
      selectedAiProvider.value = config.provider;
      selectedAiModel.value = config.model;
    } catch (error) {
      console.error("SQLite save error:", error);
      throw error;
    }
  };

  const loadSettings = async () => {
    try {
      const config: { provider: string, model: string } = await invoke('get_model_pref');
      selectedAiProvider.value = config.provider;
      selectedAiModel.value = config.model;

      await loadProviderKeyStatus(selectedAiProvider.value);
      await loadThemes();

      // Load Fonts
      fontFamily.value = await invoke('get_setting', { key: 'font_family', default_value: 'Inter' });
      const savedFontSize = await invoke('get_setting', { key: 'font_size', default_value: '14' });
      fontSize.value = parseInt(savedFontSize as string);
      fontWeight.value = await invoke('get_setting', { key: 'font_weight', default_value: '400' });
      fontStyle.value = await invoke('get_setting', { key: 'font_style', default_value: 'normal' });
      
      const autoCompile = await invoke('get_setting', { key: 'auto_compile', default_value: 'true' });
      isAutoCompileEnabled.value = autoCompile === 'true';

      applyFonts();

    } catch (e) {
      console.error("Error loading settings:", e);
      hasSecureKey.value = false;
    }
  };

  return { 
    hasSecureKey, 
    selectedAiProvider,
    selectedAiModel, 
    availableThemes,
    activeThemeId,
    fontFamily,
    fontSize,
    fontWeight,
    fontStyle,
    isAutoCompileEnabled,
    setTheme,
    setFontFamily,
    setFontSize,
    setFontWeight,
    setFontStyle,
    setAutoCompile,
    resetTypography,
    importCustomTheme,
    deleteCustomTheme,
    saveApiKey, 
    getDecryptedKey,
    saveSecret,
    getSecret,
    loadProviderKeyStatus,
    saveModelConfig,
    loadSettings 
  };
});
