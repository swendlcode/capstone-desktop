import { create } from 'zustand';

const DONE_KEY = 'capstone:onboardingDone';

/**
 * Read synchronously, at module load, so the very first render already knows
 * whether the tour is wanted. Deciding it in an effect instead would mount the
 * dialog for one frame on every relaunch, which is exactly the flash an
 * existing user must not see.
 */
function readDone(): boolean {
  try {
    return localStorage.getItem(DONE_KEY) === '1';
  } catch {
    // Private mode or blocked site data. Treat it as already seen: a tour that
    // cannot remember being dismissed would return on every launch, which is
    // worse than never showing it.
    return true;
  }
}

function markDone(): void {
  try {
    localStorage.setItem(DONE_KEY, '1');
  } catch {
    /* best effort; it still stays closed for this session */
  }
}

interface OnboardingStore {
  open: boolean;
  /** Zero-based index into ONBOARDING_STEPS. */
  step: number;
  goTo: (step: number) => void;
  /**
   * Skip, Escape, the backdrop, the close button and the last step all land
   * here: every way out of the tour counts as having done it, so none of them
   * leaves the user to be interrupted again tomorrow.
   */
  finish: () => void;
  /** Re-opened from the help action, always from the first step. */
  restart: () => void;
}

/**
 * A store rather than local state because two separate components read it: the
 * dialog and the help action that reopens it. It lives here instead of in
 * `stores/` because nothing outside the onboarding feature touches it.
 */
export const useOnboarding = create<OnboardingStore>((set) => ({
  open: !readDone(),
  step: 0,
  goTo: (step) => set({ step: Math.max(0, step) }),
  finish: () => {
    markDone();
    set({ open: false });
  },
  restart: () => set({ open: true, step: 0 }),
}));
