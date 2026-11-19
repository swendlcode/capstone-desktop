import buildMeta from '../data/build.json';

/**
 * Which build this is.
 *
 * The version says what changed; the build number says which copy is running.
 * That second question could not be answered from anything on screen while
 * several builds of one version went on and off the machine in an afternoon:
 * both called themselves 0.2.3, so "are you on the one with the fix?" had no
 * answer.
 *
 * `scripts/install-local.sh` increments this before every build, so the number
 * counts builds that actually exist rather than something anyone has to
 * remember to bump.
 */
export const BUILD_NUMBER: number = buildMeta.build;

/** `0.2.3 (47)` — the version with the build it came from. */
export function versionLabel(version: string): string {
  if (!version) return '';
  return `${version} (${BUILD_NUMBER})`;
}
