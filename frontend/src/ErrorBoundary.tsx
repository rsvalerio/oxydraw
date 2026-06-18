// Top-level error boundary. The app embeds the third-party `@excalidraw/excalidraw` editor and
// feeds it data parsed from untrusted/remote sources (share fragments, collab snapshots, REST
// responses), any of which can produce a shape the editor rejects at render time. Without a
// boundary, one such throw unwinds to the root and React 19 unmounts the whole tree, leaving a
// blank page with no recovery. This catches the throw, logs it for diagnosis, and renders a
// recoverable fallback instead (ASYNC-6).
//
// Error boundaries have no hook equivalent, so this is a class component by necessity.

import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("unhandled render error", error, info.componentStack);
  }

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div className="oxydraw-error-boundary" role="alert">
          <h1>Something went wrong</h1>
          <p>The editor hit an unexpected error. Reloading usually recovers your workspace.</p>
          <button type="button" onClick={() => window.location.reload()}>
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
