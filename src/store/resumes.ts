import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface BaseResume {
  id: string;
  name: string;
  category: string;
  created_at: string;
  updated_at: string;
}

export interface ResumeDetail extends BaseResume {
  latex_content: string;
}

export const useResumesStore = defineStore('resumes', () => {
  const resumes = ref<BaseResume[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  const loadAllResumes = async () => {
    isLoading.value = true;
    error.value = null;
    
    try {
      const data = await invoke<BaseResume[]>('get_all_resumes');
      resumes.value = data;
    } catch (err: any) {
      error.value = err.toString();
    } finally {
      isLoading.value = false;
    }
  };

  const getResumeById = async (resumeId: string): Promise<ResumeDetail> => {
    try {
      return await invoke<ResumeDetail>('get_resume_by_id', { resumeId });
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    }
  };

  const createNewResume = async (name: string, category: string, latex_content: string): Promise<string> => {
    isLoading.value = true;
    error.value = null;
    
    try {
      const resumeId = await invoke<string>('create_new_resume', {
        args: { name, category, latexContent: latex_content }
      });
      await loadAllResumes(); // Refresh list
      return resumeId;
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const updateResume = async (resumeId: string, name: string, category: string, latex_content: string): Promise<void> => {
    isLoading.value = true;
    error.value = null;
    
    try {
      await invoke('update_resume', {
        args: { resumeId, name, category, latexContent: latex_content }
      });
      await loadAllResumes(); // Refresh list
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  const deleteResume = async (resumeId: string): Promise<void> => {
    isLoading.value = true;
    error.value = null;

    try {
      await invoke('delete_resume', { args: { resumeId } });
      await loadAllResumes();
    } catch (err: any) {
      error.value = err.toString();
      throw err;
    } finally {
      isLoading.value = false;
    }
  };

  return {
    resumes,
    isLoading,
    error,
    loadAllResumes,
    getResumeById,
    createNewResume,
    updateResume,
    deleteResume
  };
});