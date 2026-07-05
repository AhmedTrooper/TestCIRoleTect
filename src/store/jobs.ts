import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from './settings';

export interface Job {
  id: string;
  company_name: string;
  job_title: string;
  work_model: string;
  employment_type: string;
  status: string;
  raw_jd: string;
  requirements?: string;
  core_responsibilities?: string;
  custom_instruction?: string;
  reference_name?: string;
  reference_email?: string;
  social_link?: string;
  job_url?: string;
  base_resume_id?: string;
  base_cl_id?: string;
  salary?: string;
  applied_date?: string;
  interview_date?: string;
  offer_date?: string;
  rejected_date?: string;
  joining_date?: string;
  created_at?: string;
  updated_at?: string;
}

export const useJobsStore = defineStore('jobs', () => {
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  const settingsStore = useSettingsStore();

  const generateId = () => {
    return Math.random().toString(36).substring(2, 12);
  };

  const parseNewJob = async (rawJd: string, jobUrl: string = ''): Promise<string> => {
    isLoading.value = true;
    error.value = null;

    try {
      await settingsStore.loadSettings();
      const apiKey = await settingsStore.getDecryptedKey();
      
      if (!apiKey) {
        const msg = "API Key not found. Please go to Settings and enter your AI API key first.";
        error.value = msg;
        throw new Error(msg);
      }

      const provider = settingsStore.selectedAiProvider;
      const model = settingsStore.selectedAiModel;

      // 1. Parse Job via AI
      const result: any = await invoke('parse_job', { 
        provider,
        model,
        apiKey, 
        rawJd,
        jobUrl: jobUrl.trim() || null
      });

      const details = result.details;
      const finalRawJd = result.raw_description || rawJd;

      // 2. Augment Data on Frontend
      const jobPayload: Job = {
        id: generateId(),
        company_name: details.company_name,
        job_title: details.job_title,
        work_model: details.work_model,
        employment_type: details.employment_type,
        status: 'Drafting',
        raw_jd: finalRawJd.trim(),
        requirements: JSON.stringify(details.requirements || []),
        core_responsibilities: JSON.stringify(details.core_responsibilities || []),
        custom_instruction: '',
        reference_name: '',
        reference_email: '',
        social_link: '',
        job_url: jobUrl.trim()
      };

      // 3. Save to Rust backend
      const savedId: string = await invoke('save_job', { 
        payload: jobPayload 
      });

      return savedId; 
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const loadAllJobs = async (): Promise<Job[]> => {
    isLoading.value = true;
    error.value = null;
    try {
      const jobs: Job[] = await invoke('get_all_jobs');
      return jobs;
    } catch (err: any) {
      error.value = err.toString();
      return [];
    } finally {
      isLoading.value = false;
    }
  };

  const getJobById = async (id: string): Promise<Job> => {
    isLoading.value = true;
    error.value = null;
    try {
      const job: Job = await invoke('get_job_by_id', { id });
      return job;
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const deleteJob = async (id: string): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('delete_job', { id });
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const deleteJobsBatch = async (ids: string[]): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('delete_jobs_batch', { ids });
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const deleteAllJobs = async (): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('delete_all_jobs');
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const updateJobStatus = async (id: string, status: string, metadata?: Record<string, string>): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('update_job_status', { id, status, metadata });
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const updateJobMetadata = async (id: string, field: string, value: string): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    try {
      await invoke('update_job_metadata', { id, field, value });
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  return { isLoading, error, parseNewJob, loadAllJobs, getJobById, deleteJob, deleteJobsBatch, deleteAllJobs, updateJobStatus, updateJobMetadata };
  });