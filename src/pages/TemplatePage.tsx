import { Button } from '../components/ui/Button';
import { Slider } from '../components/ui/icons';
import { TemplatePicker } from '../components/template/TemplatePicker';
import { OutputVolumeCard } from '../components/template/OutputVolumeCard';
import { RepairClipsCard } from '../components/template/RepairClipsCard';
import { WarmthCard } from '../components/template/WarmthCard';
import { PitchCard } from '../components/template/PitchCard';
import { MultibandEditor } from '../components/template/MultibandEditor';
import { TemplateShare } from '../components/template/TemplateShare';
import { useTemplateEditor } from '../components/template/useTemplateEditor';
import { useTemplateShare } from '../components/template/useTemplateShare';
import { useTemplates } from '../hooks/useTemplates';
import { useUiStore } from '../stores/uiStore';

/**
 * Configure Template: the named template, four setting cards, and the
 * multiband editor behind a button.
 *
 * There is no Save button anywhere on this tab. Every control commits through
 * `save_template` (see useTemplateEditor), and a stock template disables its
 * inputs instead of taking edits it would have to throw away.
 */
export function TemplatePage() {
  const { data: templates, isLoading } = useTemplates();
  const {
    template,
    editable,
    setField,
    setBand,
    setCrossover,
    selectTemplate,
    duplicateActive,
    rename,
    deleteActive,
    isDuplicating,
  } = useTemplateEditor();
  const share = useTemplateShare();
  const multibandOpen = useUiStore((s) => s.multibandOpen);
  const openMultiband = useUiStore((s) => s.openMultiband);
  const closeMultiband = useUiStore((s) => s.closeMultiband);

  if (isLoading || !template || !templates) {
    return (
      <div className="p-6 text-sm text-gray-400">
        {isLoading ? 'Loading templates…' : 'No templates.'}
      </div>
    );
  }

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <header className="flex flex-col gap-3">
        {/* Sharing sits beside the heading rather than in the gear menu: Import
            is not an action on the selected template, so a menu hung off that
            template is the wrong place for it. */}
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-lg font-bold text-capstone-white">Configure Template</h1>
          <TemplateShare
            templateName={template.name}
            busy={share.busy}
            notice={share.notice}
            onExport={() => share.exportActive(template.id)}
            onImport={share.importFile}
          />
        </div>
        <TemplatePicker
          templates={templates}
          active={template}
          editable={editable}
          isDuplicating={isDuplicating}
          onSelect={selectTemplate}
          onDuplicate={duplicateActive}
          onRename={rename}
          onDelete={deleteActive}
        />
      </header>

      {/* Two by two on a wide window, stacked when narrow. */}
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        <OutputVolumeCard template={template} editable={editable} setField={setField} />
        <RepairClipsCard template={template} editable={editable} setField={setField} />
        <WarmthCard template={template} editable={editable} setField={setField} />
        <PitchCard template={template} editable={editable} setField={setField} />
      </div>

      <div className="flex items-center gap-3">
        <Button
          variant="secondary"
          onClick={openMultiband}
          icon={<Slider size={16} color="currentColor" variant="Linear" />}
        >
          Edit multiband dynamics
        </Button>
        <span className="mono text-xs text-gray-400">{template.bands.length} bands</span>
      </div>

      {multibandOpen && (
        <MultibandEditor
          template={template}
          editable={editable}
          onClose={closeMultiband}
          onBandChange={setBand}
          onCrossover={setCrossover}
        />
      )}
    </div>
  );
}
