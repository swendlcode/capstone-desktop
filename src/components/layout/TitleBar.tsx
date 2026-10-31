import { getCurrentWindow } from '@tauri-apps/api/window';
import { CapstoneMark, Moon, Sun } from '../ui/icons';
import { useTheme } from '../../hooks/useTheme';
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

  return (
    <div className="relative grid h-12 shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-3 border-b border-gray-700 bg-gray-900 px-4">
      <div
        data-tauri-drag-region
        onMouseDown={startWindowDrag}
        className="absolute inset-0 z-0 cursor-default"
      />

      {/* Left — the mark, clear of the traffic lights, which sit at x:16. */}
      <div className="pointer-events-none z-10 flex items-center gap-2 pl-[68px]">
        <CapstoneMark size={16} className="text-capstone-sky" />
        <span className="text-sm font-bold tracking-tight text-capstone-white">Capstone</span>
      </div>

      <div className="pointer-events-none z-10 flex justify-center">
        <div className="no-drag pointer-events-auto">
          <TabBar />
        </div>
      </div>

      <div className="pointer-events-none z-10 flex items-center justify-end">
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
