import type { Template } from '../../types';

/**
 * One line per shipped template: when to reach for it.
 *
 * Keyed by stock id and held in the frontend because `Template` has no
 * description field. The IPC field names are frozen, so adding one is a change
 * to both halves of the contract at once (see the report note). When it gains
 * one, this map goes and the picker reads `template.description`.
 */
const STOCK: Record<string, string> = {
  'stock-festival': 'Loud and controlled, for a big rig.',
  'stock-club': 'Hotter and glued, for a club system.',
  'stock-stream': 'Streaming and radio, at -14 LUFS.',
  'stock-vinyl': 'Vinyl rips. Corrects a deck off speed.',
  'stock-archive': 'Repair only. No colour, no compression.',
};

export function templateDescription(template: Template): string {
  return STOCK[template.id] ?? '';
}
