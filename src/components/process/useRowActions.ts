import { useMutation } from '@tanstack/react-query';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';

/**
 * The shell actions behind a queue row's right-click menu: hand a path to the
 * OS, show it in the file manager, or put it on the clipboard.
 *
 * Reached from a hook rather than from the row itself, for the same reason
 * `invoke` lives behind the service layer: a component that talks to the shell
 * directly is a component that cannot be rendered anywhere else.
 *
 * Mutations rather than bare calls, so a failure — a file the user has moved
 * since dropping it is the everyday one — is a rejected mutation react-query
 * owns, not an unhandled rejection in the console.
 *
 * `openPath` needs more than the opener plugin's default permission set, which
 * covers urls and reveal but not opening a path. src-tauri/capabilities/
 * default.json carries an `opener:allow-open-path` entry with a `**` scope for
 * it — without both the permission and the scope, Play file and Open output
 * reject at runtime with nothing failing at build time.
 */
export function useRowActions() {
  const open = useMutation({ mutationFn: (path: string) => openPath(path) });
  const reveal = useMutation({ mutationFn: (path: string) => revealItemInDir(path) });
  const copyPath = useMutation({ mutationFn: (path: string) => copyText(path) });

  return { open, reveal, copyPath };
}

async function copyText(text: string): Promise<void> {
  // `navigator.clipboard` is exposed only in a secure context, which the
  // webview's custom origin is not on every platform, so the textarea path is
  // a working fallback here rather than legacy-browser decoration.
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const field = document.createElement('textarea');
  field.value = text;
  // Off-screen but focusable: `display: none` cannot be selected, and a
  // visible field steals focus from the table for a frame.
  field.setAttribute('aria-hidden', 'true');
  field.style.position = 'fixed';
  field.style.opacity = '0';
  document.body.appendChild(field);
  field.select();
  try {
    document.execCommand('copy');
  } finally {
    field.remove();
  }
}
