import { Button } from '../ui/Button';
import { Checkbox } from '../ui/Checkbox';
import { Pane } from './Pane';
import { useSettingsPatch, usePickArtworkDir } from '../../hooks/useSettings';
import type { Settings } from '../../types';

/**
 * What the output file carries besides the audio.
 *
 * Both switches default on, and the reason is the same one: a master that
 * comes back without its artist, title and cover has to be identified by ear.
 * They are switches at all because a few people master deliberately
 * anonymous stems.
 *
 * The artwork folder is the case worth having a control for. Downloaded
 * tracks often carry no embedded picture, and the covers were collected
 * separately — one folder, named after the tracks. Leaving it unset still
 * finds an image sitting beside the track or inside it, which is all a
 * normal library needs.
 */
export function TagsPane({ settings }: { settings: Settings }) {
  const { patch } = useSettingsPatch();
  const pickDir = usePickArtworkDir();

  return (
    <Pane title="Tags & Artwork">
      <Checkbox
        checked={settings.keepTags}
        onChange={(e) => patch({ keepTags: e.target.checked })}
        label="Keep the artist, title and other tags"
      />
      <p className="-mt-2 ml-6 text-xs leading-relaxed text-gray-500">
        Apple Lossless cannot hold tags, whatever this says.
      </p>

      <Checkbox
        checked={settings.embedArtwork}
        onChange={(e) => patch({ embedArtwork: e.target.checked })}
        label="Embed cover art"
      />

      <div className="ml-6 flex flex-col gap-3">
        <p className="text-[10px] uppercase tracking-widest text-gray-500">
          Artwork folder
        </p>
        <div className="flex items-center gap-3">
          <span
            className={`mono min-w-0 flex-1 truncate rounded-md border border-gray-600 bg-gray-700 px-3 py-2 text-xs ${
              settings.embedArtwork ? 'text-gray-300' : 'text-gray-500'
            }`}
          >
            {settings.artworkDir ?? 'Beside the track, or the track’s own cover'}
          </span>
          <Button
            size="sm"
            variant="secondary"
            disabled={!settings.embedArtwork}
            onClick={() => pickDir.mutate()}
          >
            Choose…
          </Button>
          {settings.artworkDir && (
            <Button
              size="sm"
              variant="ghost"
              disabled={!settings.embedArtwork}
              onClick={() => patch({ artworkDir: null })}
            >
              Clear
            </Button>
          )}
        </div>
        <p className="text-xs leading-relaxed text-gray-500">
          An image in here whose name matches the track — “Dom Dolla -
          Strangers.jpg” — is used as the cover. JPEG and PNG.
        </p>
      </div>
    </Pane>
  );
}
