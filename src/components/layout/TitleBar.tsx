import { getCurrentWindow } from '@tauri-apps/api/window';
import { Moon, Sun } from '../ui/icons';
import { useTheme } from '../../hooks/useTheme';
import { UpdateBadge } from './UpdateBadge';
import { OnboardingHelpButton } from '../onboarding';
import { TabBar } from './TabBar';

export function TitleBar() {
  const { theme, toggleTheme } = useTheme();

  // The window uses the macOS Overlay title bar, so there is no native bar to
  // drag. A full-bleed layer underneath the controls provides it, Chrome-style,
  // and everything above it opts back out with `no-drag`.
  const startWindowDrag = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    getCurrentWindow().startDragging().catch(() => {});
  };

  // The mark lives in the StatusBar instead of here. With the left side free,
  // the tabs take it rather than sitting wedged between a logo and the theme
  // toggle, which is what made the bar read as three competing groups.
  return (
    <div className="relative flex h-12 shrink-0 items-center justify-between gap-3 border-b border-gray-700 bg-gray-900 px-4">
      <div
        data-tauri-drag-region
        onMouseDown={startWindowDrag}
        className="absolute inset-0 z-0 cursor-default"
      />

      {/* pl-[68px] keeps the tabs clear of the traffic lights, which sit at x:16.
          The wrappers stay pointer-events-none so the bar's empty width still
          reaches the drag layer; only the controls take the clicks back. */}
      <div className="pointer-events-none z-10 flex items-center pl-[68px]">
        <div className="no-drag pointer-events-auto">
          <TabBar />
        </div>
      </div>

      <div className="pointer-events-none z-10 flex items-center gap-2">
        <div className="no-drag pointer-events-auto">
          <UpdateBadge />
        </div>
        <OnboardingHelpButton />
        <button
          onClick={toggleTheme}
          className="no-drag pointer-events-auto flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition-colors hover:bg-gray-800 hover:text-capstone-white"
          aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        >
          {theme === 'dark' ? (
            <Sun size={18} color="currentColor" variant="Linear" />
          ) : (
            <Moon size={18} color="currentColor" variant="Linear" />
          )}
        </button>
      </div>
    </div>
  );
}
