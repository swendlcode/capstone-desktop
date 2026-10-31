import { PersonalizeRail } from '../components/personalize/PersonalizeRail';
import { OutputLocationPane } from '../components/personalize/OutputLocationPane';
import { OutputFormatPane } from '../components/personalize/OutputFormatPane';
import { ProcessorLoadPane } from '../components/personalize/ProcessorLoadPane';
import { useSettings } from '../hooks/useSettings';
import { useSettingsStore } from '../stores/settingsStore';

/**
 * Personalize: a rail of three sections and the matching pane.
 *
 * Like the Configure tab, there is no Save button — every control writes
 * through `update_settings` on change, and the pane renders from what came
 * back rather than from a local draft, so a rejected write shows the stored
 * value instead of the attempted one.
 */
export function PersonalizePage() {
  const { data: settings, isLoading } = useSettings();
  const section = useSettingsStore((s) => s.section);

  return (
    <div className="mx-auto flex max-w-4xl gap-8 p-6">
      <PersonalizeRail />
      {!settings ? (
        <div className="flex-1 text-sm text-gray-400">
          {isLoading ? 'Loading settings…' : 'Settings unavailable.'}
        </div>
      ) : (
        <>
          {section === 'location' && <OutputLocationPane settings={settings} />}
          {section === 'format' && <OutputFormatPane settings={settings} />}
          {section === 'cores' && <ProcessorLoadPane settings={settings} />}
        </>
      )}
    </div>
  );
}
