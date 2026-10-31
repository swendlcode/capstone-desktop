import { Cpu, Folder2, MusicSquare } from '../ui/icons';
import { useSettingsStore } from '../../stores/settingsStore';
import type { PersonalizeSection } from '../../stores/settingsStore';

const SECTIONS: Array<{ id: PersonalizeSection; label: string; icon: typeof Cpu }> = [
  { id: 'location', label: 'Output Location', icon: Folder2 },
  { id: 'format', label: 'Output File Format', icon: MusicSquare },
  { id: 'cores', label: 'Processor Load', icon: Cpu },
];

/** The left rail. Which pane is showing is the only UI state on this tab. */
export function PersonalizeRail() {
  const section = useSettingsStore((s) => s.section);
  const setSection = useSettingsStore((s) => s.setSection);

  return (
    <nav aria-label="Personalize sections" className="flex w-52 shrink-0 flex-col gap-1">
      {SECTIONS.map(({ id, label, icon: Icon }) => {
        const active = id === section;
        return (
          <button
            key={id}
            onClick={() => setSection(id)}
            aria-current={active ? 'page' : undefined}
            className={`flex items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
              active
                ? 'bg-capstone-sky/10 text-capstone-sky'
                : 'text-gray-400 hover:bg-gray-800 hover:text-capstone-white'
            }`}
          >
            <Icon size={16} color="currentColor" variant="Linear" />
            {label}
          </button>
        );
      })}
    </nav>
  );
}
