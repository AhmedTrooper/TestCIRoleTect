import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface InboxJob {
  id: string;
  url: string | null;
  raw_description: string;
  status: 'Pending' | 'Processed';
  created_at: string;
}

export interface ExtensionConfig {
  secret: string;
  port: string;
}

export const useInboxStore = defineStore('inbox', () => {
  const jobs = ref<InboxJob[]>([]);
  const isLoading = ref(false);
  const extensionConfig = ref<ExtensionConfig | null>(null);

  const loadJobs = async () => {
    isLoading.value = true;
    try {
      jobs.value = await invoke<InboxJob[]>('get_all_inbox_jobs');
    } catch (error) {
      console.error('Failed to load inbox jobs:', error);
    } finally {
      isLoading.value = false;
    }
  };

  const deleteJob = async (id: string) => {
    try {
      await invoke('delete_inbox_job', { id });
      await loadJobs();
    } catch (error) {
      console.error('Failed to delete inbox job:', error);
      throw error;
    }
  };

  const deleteAllJobs = async () => {
    try {
      await invoke('delete_all_inbox_jobs');
      await loadJobs();
    } catch (error) {
      console.error('Failed to delete all inbox jobs:', error);
      throw error;
    }
  };

  const markProcessed = async (id: string) => {
    try {
      await invoke('mark_inbox_job_processed', { id });
      await loadJobs();
    } catch (error) {
      console.error('Failed to mark job as processed:', error);
    }
  };

  const loadExtensionConfig = async () => {
    try {
      extensionConfig.value = await invoke<ExtensionConfig>('get_extension_config');
    } catch (error) {
      console.error('Failed to load extension config:', error);
    }
  };

  const resetSecret = async () => {
    try {
      const newSecret = await invoke<string>('reset_extension_secret');
      if (extensionConfig.value) {
        extensionConfig.value.secret = newSecret;
      }
      return newSecret;
    } catch (error) {
      console.error('Failed to reset extension secret:', error);
      throw error;
    }
  };

  return {
    jobs,
    isLoading,
    extensionConfig,
    loadJobs,
    deleteJob,
    deleteAllJobs,
    markProcessed,
    loadExtensionConfig,
    resetSecret,
  };
});
