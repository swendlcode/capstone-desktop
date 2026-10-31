import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  // Tauri's own CLI output is the useful half of the dev log; clearing the
  // screen on every Vite restart wipes the Rust compile errors above it.
  clearScreen: false,
  resolve: {
    // Exact-match patterns so the shims' own `@tauri-apps/api` import does not
    // resolve back into themselves. Plugin packages import from these same
    // specifiers, which is why the shims must re-export everything a plugin
    // reaches for (see src/lib/tauri-core.ts).
    alias: [
      {
        find: /^@tauri-apps\/api\/core$/,
        replacement: fileURLToPath(new URL('./src/lib/tauri-core.ts', import.meta.url)),
      },
      {
        find: /^@tauri-apps\/api\/event$/,
        replacement: fileURLToPath(new URL('./src/lib/tauri-event.ts', import.meta.url)),
      },
    ],
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    // Rust sources are rebuilt by the Tauri CLI, not Vite. Without this the
    // watcher restarts the dev server on every `cargo` artifact write.
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules/react/') || id.includes('node_modules/react-dom/')) {
            return 'react-vendor';
          }
          if (id.includes('@tanstack')) {
            return 'tanstack';
          }
          if (id.includes('zustand')) {
            return 'state';
          }
          if (id.includes('@tauri-apps')) {
            return 'tauri';
          }
        },
      },
    },
  },
}));
