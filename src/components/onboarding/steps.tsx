import type { ReactNode } from 'react';
import { FolderOpen, Import, ShieldTick, Slider } from '../ui/icons';

export interface OnboardingStep {
  /** Two or three words. The dialog header is one line and does not wrap. */
  title: string;
  /**
   * One line, and it stays one line. A step that needs a paragraph is a sign
   * the screen behind it is unclear, and the fix for that is the screen.
   */
  body: string;
  icon: ReactNode;
}

const ICON = { size: 20, color: 'currentColor', variant: 'Linear' } as const;

/**
 * Four steps, in the order a first run actually happens, answering the three
 * questions a new user has: what this does to my files, where they go, and
 * whether it touches the originals.
 */
export const ONBOARDING_STEPS: OnboardingStep[] = [
  {
    title: 'What this does',
    body: 'Fixes loudness, clipping and pitch across a batch. Never touches your originals, it writes new files.',
    icon: <ShieldTick {...ICON} />,
  },
  {
    title: 'Add files',
    body: 'Drop files or folders anywhere on the Process tab.',
    icon: <Import {...ICON} />,
  },
  {
    title: 'Templates',
    body: 'A template is the settings. Duplicate Festival to change anything.',
    icon: <Slider {...ICON} />,
  },
  {
    title: 'Output',
    body: 'New files land in your output folder. Personalize sets it and the format.',
    icon: <FolderOpen {...ICON} />,
  },
];
