import { Button } from '../ui/Button';
import { Add, Import } from '../ui/icons';

interface DropTargetProps {
  /** Something is hovering the window right now. */
  isOver: boolean;
  onAddFiles: () => void;
  disabled?: boolean;
}

/**
 * The empty state, which is the drop target itself rather than a message with a
 * drop zone somewhere under it.
 *
 * The extensions line is the list the backend actually accepts
 * (`AUDIO_EXTENSIONS` in audio/decode.rs): a drop of anything else is filtered
 * out silently, and a user who dropped a folder of AIFFs needs to know whether
 * nothing happened because of the app or because of the files.
 */
export function DropTarget({ isOver, onAddFiles, disabled }: DropTargetProps) {
  return (
    <div className="flex min-h-0 flex-1 items-center justify-center p-6">
      <div
        className={`flex w-full max-w-xl flex-col items-center gap-5 rounded-xl border border-dashed px-8 py-14 transition-colors ${
          isOver ? 'border-capstone-sky bg-capstone-sky/5' : 'border-gray-700 bg-gray-900/40'
        }`}
      >
        <Import
          size={28}
          color="currentColor"
          variant="Linear"
          className={isOver ? 'text-capstone-sky' : 'text-gray-600'}
        />
        <div className="text-center">
          <p className="text-[15px] font-semibold text-capstone-white">
            Drag files and folders here
          </p>
          <p className="mono mt-2 text-[10px] uppercase tracking-[0.14em] text-gray-600">
            wav · aiff · flac · mp3 · m4a · ogg
          </p>
        </div>
        <Button
          variant="primary"
          size="md"
          disabled={disabled}
          onClick={onAddFiles}
          icon={<Add size={16} color="currentColor" variant="Linear" />}
        >
          Add Files...
        </Button>
      </div>
    </div>
  );
}

/**
 * Shown over a queue that already has rows, so a second drop has the same
 * visible target as the first. Pointer events stay off it: the window-level
 * drag-drop event is what delivers the paths, and an overlay that swallowed
 * clicks would also swallow the row underneath after the drag ended.
 */
export function DropOverlay() {
  return (
    <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-capstone-black/70">
      <div className="rounded-xl border border-dashed border-capstone-sky bg-capstone-sky/5 px-10 py-8 text-center">
        <p className="text-sm font-semibold text-capstone-white">Drop to add</p>
      </div>
    </div>
  );
}
