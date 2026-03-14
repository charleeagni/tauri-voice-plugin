# Svelte → React Migration: `examples/tauri-app`

Convert the example Tauri app from Svelte 5 to React 18, keeping all existing STT plugin logic and UI intact.

## Proposed Changes

### `examples/tauri-app`

---

#### [MODIFY] [package.json](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/package.json)

- Remove `svelte` and `@sveltejs/vite-plugin-svelte` from `devDependencies`.
- Add `react` and `react-dom` to `dependencies`.
- Add `@vitejs/plugin-react` to `devDependencies`.

---

#### [MODIFY] [vite.config.js](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/vite.config.js)

- Replace `import { svelte } from "@sveltejs/vite-plugin-svelte"` with `import react from "@vitejs/plugin-react"`.
- Replace `plugins: [svelte()]` with `plugins: [react()]`.
- All Tauri-specific config (`clearScreen`, `server`, `strictPort`, `hmr`) remains unchanged.

---

#### [MODIFY] [jsconfig.json](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/jsconfig.json)

- Remove Svelte-specific options (`verbatimModuleSyntax`, `isolatedModules`, `checkJs`).
- Add `"jsx": "react-jsx"` to `compilerOptions`.
- Update `include` to cover `src/**/*.jsx` instead of `src/**/*.svelte`.

---

#### [MODIFY] [index.html](file:///Users/karthik/merge_conflicts/voice_plugin/tauri-plugin-stt/examples/tauri-app/index.html)

- Update `<title>` from `Tauri + Svelte` to `Tauri + React`.
- Update the `<script>` `src` from `/src/main.js` to `/src/main.jsx`.

---

#### [DELETE] `src/main.js` + `src/App.svelte`

These files are removed and replaced below.

---

#### [NEW] `src/main.jsx`

- Import `React` and `ReactDOM` from `react` and `react-dom/client`.
- Import `./style.css` and `App`.
- Mount with `ReactDOM.createRoot(document.getElementById('app')).render(<App />)`.

---

#### [NEW] `src/App.jsx`

Port `App.svelte` logic 1-for-1 into a React function component:

| Svelte concept | React equivalent |
|---|---|
| `let response = $state('')` | `const [response, setResponse] = useState('')` |
| `updateResponse(val)` helper | Same function, calls `setResponse(prev => ...)` |
| `onclick={fn}` on buttons | `onClick={fn}` |
| `<style>` block | Moved into `style.css` (already exists) or kept as a `<style>` tag via a CSS module — simplest: move styles to `style.css` |

All four actions (`checkHealth`, `bootstrap`, `checkState`, `clearLogs`) and the debug console `<pre>` are preserved verbatim.

> [!NOTE]
> The component CSS currently lives inside `App.svelte`'s `<style>` block (scoped). Moving to React, these styles will be added to `style.css` as global rules — the selectors (`.container`, `.actions`, `button`, `.debug-console`, etc.) are specific enough that there is no real risk of collision in this single-component app.

---

## Verification Plan

### Manual Verification

After making the changes, run the example app and confirm it behaves identically to the Svelte version:

```bash
cd examples/tauri-app
pnpm install        # installs new deps, removes old ones
pnpm tauri dev      # starts the Tauri + Vite dev server
```

Check:
1. The window opens with the title **STT Plugin Debugger**.
2. Clicking **Check Health** appends a timestamped response to the debug console.
3. Clicking **Bootstrap STT** appends a timestamped response.
4. Clicking **Get Runtime State** appends a timestamped response.
5. Clicking **Clear Logs** empties the console.
6. No Svelte-related errors appear in the browser dev tools console or terminal.
