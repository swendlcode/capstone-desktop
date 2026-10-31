import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App';
import { applyTheme, readPersistedAccent, readPersistedTheme } from './hooks/useTheme';
import './index.css';

// Before React mounts, so the first paint is the user's theme rather than the
// default flashing and then swapping. ES imports are evaluated first, so
// index.css is already applied by the time this line runs.
//
// It cannot cover the paint before this module runs at all. The stylesheet's
// `:root` block is the dark set, so a dark user never sees a wrong frame; a
// light user gets one dark frame between the CSS landing and this line.
// Closing that needs data-theme set from index.html's <head>, which is not
// this file's to write.
applyTheme(readPersistedTheme(), readPersistedAccent());

window.addEventListener('error', (e) => {
  console.error('[Global] Unhandled error:', e.error ?? e.message);
});
window.addEventListener('unhandledrejection', (e) => {
  console.error('[Global] Unhandled promise rejection:', e.reason);
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

// An `invoke` rejection arrives as a plain string from Rust (see the backend's
// Serialize impl), which react-query will otherwise swallow into a query state
// nobody reads.
queryClient.getQueryCache().subscribe((event) => {
  if (event.type === 'updated' && event.query.state.status === 'error') {
    console.error(
      '[QueryClient] Query error:',
      event.query.queryKey,
      event.query.state.error,
    );
  }
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
