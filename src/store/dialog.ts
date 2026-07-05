import { defineStore } from 'pinia';
import { ref } from 'vue';

export type DialogType = 'alert' | 'confirm' | 'prompt' | 'datepicker';

interface DialogOptions {
  title?: string;
  message: string;
  type: DialogType;
  defaultValue?: any;
  confirmText?: string;
  cancelText?: string;
  onConfirm: (value?: any) => void;
  onCancel: () => void;
}

export const useDialogStore = defineStore('dialog', () => {
  const isOpen = ref(false);
  const options = ref<DialogOptions | null>(null);
  const inputValue = ref<any>('');

  const showAlert = (message: string, title = 'Notification') => {
    return new Promise<void>((resolve) => {
      options.value = {
        title,
        message,
        type: 'alert',
        onConfirm: () => {
          isOpen.value = false;
          resolve();
        },
        onCancel: () => {
          isOpen.value = false;
          resolve();
        }
      };
      isOpen.value = true;
    });
  };

  const showConfirm = (message: string, title = 'Confirm Action') => {
    return new Promise<boolean>((resolve) => {
      options.value = {
        title,
        message,
        type: 'confirm',
        onConfirm: () => {
          isOpen.value = false;
          resolve(true);
        },
        onCancel: () => {
          isOpen.value = false;
          resolve(false);
        }
      };
      isOpen.value = true;
    });
  };

  const showPrompt = (message: string, defaultValue = '', title = 'Input Required') => {
    inputValue.value = defaultValue;
    return new Promise<string | null>((resolve) => {
      options.value = {
        title,
        message,
        type: 'prompt',
        defaultValue,
        onConfirm: (value) => {
          isOpen.value = false;
          resolve(value || '');
        },
        onCancel: () => {
          isOpen.value = false;
          resolve(null);
        }
      };
      isOpen.value = true;
    });
  };

  const showDatePicker = (message: string, defaultValue: string | Date = new Date(), title = 'Select Date') => {
    inputValue.value = defaultValue;
    return new Promise<string | null>((resolve) => {
      options.value = {
        title,
        message,
        type: 'datepicker',
        defaultValue,
        onConfirm: (value) => {
          isOpen.value = false;
          // Format date to YYYY-MM-DD
          if (value instanceof Date) {
            resolve(value.toISOString().split('T')[0]);
          } else if (typeof value === 'string') {
            resolve(value);
          } else {
            resolve(null);
          }
        },
        onCancel: () => {
          isOpen.value = false;
          resolve(null);
        }
      };
      isOpen.value = true;
    });
  };

  return { isOpen, options, inputValue, showAlert, showConfirm, showPrompt, showDatePicker };
});