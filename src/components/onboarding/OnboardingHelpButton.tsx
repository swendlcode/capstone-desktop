import { Teacher } from '../ui/icons';
import { useOnboarding } from '../../hooks/useOnboarding';

interface OnboardingHelpButtonProps {
  className?: string;
}

/**
 * Reopens the tour. One icon button, not a help menu: the tour is the only
 * thing there is to reopen, and a menu holding a single item is a menu that
 * exists to be clicked twice.
 *
 * Sized and coloured like the theme toggle it sits beside in the title bar,
 * which is also why the drag-region exemptions are on it — the title bar has
 * no native chrome, so every control on it has to opt out of dragging.
 */
export function OnboardingHelpButton({ className = '' }: OnboardingHelpButtonProps) {
  const restart = useOnboarding((s) => s.restart);

  return (
    <button
      type="button"
      onClick={restart}
      aria-label="Show the tutorial"
      title="Show the tutorial"
      className={`no-drag pointer-events-auto flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition-colors hover:bg-gray-800 hover:text-capstone-white ${className}`}
    >
      <Teacher size={18} color="currentColor" variant="Linear" />
    </button>
  );
}
