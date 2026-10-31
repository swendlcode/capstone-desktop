import { useUiStore } from '../../stores/uiStore';
import type { ActiveTab } from '../../stores/uiStore';

const TABS: Array<{ id: ActiveTab; label: string }> = [
  { id: 'process', label: 'Process' },
  { id: 'template', label: 'Template' },
  { id: 'personalize', label: 'Personalize' },
];

/** The three top-level tabs, rendered as a segmented control in the title bar. */
export function TabBar() {
  const activeTab = useUiStore((s) => s.activeTab);
  const setActiveTab = useUiStore((s) => s.setActiveTab);

  return (
    <div
      role="tablist"
      aria-label="Sections"
      className="flex items-center gap-1 rounded-lg bg-gray-800 p-1"
    >
      {TABS.map((tab) => {
        const active = tab.id === activeTab;
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active}
            onClick={() => setActiveTab(tab.id)}
            className={`h-7 rounded-md px-3 text-xs font-semibold uppercase tracking-wide transition-colors ${
              active
                ? 'bg-capstone-sky/10 text-capstone-sky'
                : 'text-gray-400 hover:bg-gray-700 hover:text-capstone-white'
            }`}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
