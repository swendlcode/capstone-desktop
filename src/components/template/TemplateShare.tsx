import { Button } from '../ui/Button';
import { Export, Import, TickCircle, Warning2 } from '../ui/icons';

export interface ShareNotice {
  kind: 'ok' | 'error';
  /** One sentence. Refusals come from the backend already worded for a human. */
  text: string;
}

interface Props {
  /** The template Export writes out; used for the buttons' labels only. */
  templateName: string;
  /** Which of the two is waiting on a file picker, if either. */
  busy?: 'export' | 'import' | null;
  notice?: ShareNotice | null;
  onExport: () => void;
  onImport: () => void;
}

/**
 * Export and Import, which is the whole of sharing a template.
 *
 * A template is a file. Export writes one, Import reads one, and anywhere a
 * file can go — a forum post, a DM, a repo — a template can go. There is no
 * server and no account to add later: the file is the feature.
 *
 * Imported templates arrive as the user's own, so they are editable and
 * deletable and show up under "Yours" in the picker. A name already in use
 * gets a suffix rather than overwriting, and every value is clamped to the
 * range the cards below enforce — a shared file is a stranger's file.
 */
export function TemplateShare({
  templateName,
  busy = null,
  notice = null,
  onExport,
  onImport,
}: Props) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        variant="ghost"
        size="sm"
        disabled={busy !== null}
        onClick={onExport}
        aria-label={`Export ${templateName}`}
        icon={<Export size={14} color="currentColor" variant="Linear" />}
      >
        {busy === 'export' ? 'Exporting…' : 'Export'}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        disabled={busy !== null}
        onClick={onImport}
        aria-label="Import a template file"
        icon={<Import size={14} color="currentColor" variant="Linear" />}
      >
        {busy === 'import' ? 'Importing…' : 'Import'}
      </Button>

      {notice && (
        <span
          role="status"
          // A refusal that reads grey like a hint is a refusal the user does not
          // notice, so it takes the palette's danger token — the same one the
          // queue's error rows wear.
          className={`flex min-w-0 items-center gap-1.5 text-xs ${
            notice.kind === 'error' ? 'text-capstone-red' : 'text-gray-400'
          }`}
        >
          {notice.kind === 'error' ? (
            <Warning2 size={14} color="currentColor" variant="Linear" />
          ) : (
            <TickCircle size={14} color="currentColor" variant="Linear" />
          )}
          <span className="truncate">{notice.text}</span>
        </span>
      )}
    </div>
  );
}
