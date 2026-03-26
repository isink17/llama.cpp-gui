import { tauriBridge } from './lib/tauri';

function App() {
  return (
    <main className="app-shell">
      <section className="hero">
        <p className="eyebrow">Migration item #n1</p>
        <h1>LlamaCppDesk frontend scaffold</h1>
        <p className="lede">
          This is the baseline Tauri UI shell. It is intentionally small and is
          ready for future command, event, and state wiring.
        </p>

        <div className="notice">
          <strong>Migration notice</strong>
          <p>
            The app shell is live. The next migration steps will connect real
            Tauri commands and event listeners through the bridge module.
          </p>
        </div>

        <dl className="status">
          <div>
            <dt>Frontend stack</dt>
            <dd>Vite + React + TypeScript</dd>
          </div>
          <div>
            <dt>Tauri bridge</dt>
            <dd>{tauriBridge.status}</dd>
          </div>
        </dl>
      </section>
    </main>
  );
}

export default App;
