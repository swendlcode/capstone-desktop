import { lazy, Suspense } from 'react';
import { TitleBar } from './components/layout/TitleBar';
import { ErrorBoundary } from './components/ErrorBoundary';
import { useUiStore } from './stores/uiStore';
import { useQueueSync } from './hooks/useQueue';

// Each tab is split out so the first paint only carries the one that is open.
const ProcessPage = lazy(() =>
  import('./pages/ProcessPage').then((m) => ({ default: m.ProcessPage })),
);
const TemplatePage = lazy(() =>
  import('./pages/TemplatePage').then((m) => ({ default: m.TemplatePage })),
);
const PersonalizePage = lazy(() =>
  import('./pages/PersonalizePage').then((m) => ({ default: m.PersonalizePage })),
);

export default function App() {
  const activeTab = useUiStore((s) => s.activeTab);

  // Mounted once, here, rather than per page: the queue keeps running while the
  // user reads the other two tabs, and a listener that unmounts with the table
  // would miss every event in between.
  useQueueSync();

  return (
    <div className="h-app flex w-full flex-col overflow-hidden bg-capstone-black">
      <TitleBar />
      <main className="min-h-0 flex-1 overflow-y-auto">
        <ErrorBoundary key={activeTab}>
          <Suspense fallback={<div className="h-full w-full" />}>
            {activeTab === 'process' && <ProcessPage />}
            {activeTab === 'template' && <TemplatePage />}
            {activeTab === 'personalize' && <PersonalizePage />}
          </Suspense>
        </ErrorBoundary>
      </main>
    </div>
  );
}
