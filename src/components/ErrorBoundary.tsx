import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
  fallback?: (error: Error, reset: () => void) => ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Keeps a throwing tab from taking the whole window down. App gives it
 * `key={activeTab}`, so switching tabs resets it — otherwise a crash in one
 * tab would persist across a navigation that has nothing to do with it.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[ErrorBoundary] Caught error:', error);
    console.error('[ErrorBoundary] Component stack:', info.componentStack);
  }

  reset = () => this.setState({ error: null });

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    if (this.props.fallback) return this.props.fallback(error, this.reset);
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
        <div className="text-base font-semibold text-capstone-white">Something went wrong</div>
        <pre className="mono max-w-lg overflow-auto rounded bg-gray-800 p-3 text-left text-xs text-capstone-sky">
          {error.message}
        </pre>
        <button
          onClick={this.reset}
          className="rounded-md bg-capstone-sky px-4 py-2 text-sm font-medium text-capstone-black hover:bg-[color:var(--color-accent-hover)]"
        >
          Try again
        </button>
      </div>
    );
  }
}
