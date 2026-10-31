import { create } from 'zustand';

/** The left rail of the Personalize tab. */
export type PersonalizeSection = 'location' | 'format' | 'cores';

const SECTION_KEY = 'capstone:personalizeSection';

function loadSection(): PersonalizeSection {
  try {
    const v = localStorage.getItem(SECTION_KEY);
    if (v === 'location' || v === 'format' || v === 'cores') return v;
  } catch {
    /* private mode or blocked site data */
  }
  return 'location';
}

/**
 * Which pane of Personalize is showing — and nothing else. The settings
 * themselves are server state (hooks/useSettings.ts): every control writes
 * straight through `update_settings`, so there is no draft to hold here and no
 * second copy that can disagree with what is on disk.
 */
interface SettingsStore {
  section: PersonalizeSection;
  setSection: (section: PersonalizeSection) => void;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  section: loadSection(),
  setSection: (section) => {
    try {
      localStorage.setItem(SECTION_KEY, section);
    } catch {
      /* best effort */
    }
    set({ section });
  },
}));
