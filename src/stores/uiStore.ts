import { create } from 'zustand';

/** The three top-level tabs, mirroring the product's own structure. */
export type ActiveTab = 'process' | 'template' | 'personalize';

const ACTIVE_TAB_KEY = 'capstone:activeTab';

function loadActiveTab(): ActiveTab {
  try {
    const v = localStorage.getItem(ACTIVE_TAB_KEY);
    if (v === 'process' || v === 'template' || v === 'personalize') return v;
  } catch {
    /* private mode or blocked site data — fall through to the default */
  }
  return 'process';
}

interface UiStore {
  activeTab: ActiveTab;
  /**
   * The multiband compressor editor, opened from the Configure Template tab.
   * Deliberately not persisted: reopening a modal editor on launch hides the
   * tab the user actually asked for.
   */
  multibandOpen: boolean;
  setActiveTab: (tab: ActiveTab) => void;
  openMultiband: () => void;
  closeMultiband: () => void;
}

export const useUiStore = create<UiStore>((set) => ({
  activeTab: loadActiveTab(),
  multibandOpen: false,
  setActiveTab: (activeTab) => {
    try {
      localStorage.setItem(ACTIVE_TAB_KEY, activeTab);
    } catch {
      /* best effort; the tab still switches for this session */
    }
    set({ activeTab });
  },
  openMultiband: () => set({ multibandOpen: true }),
  closeMultiband: () => set({ multibandOpen: false }),
}));
