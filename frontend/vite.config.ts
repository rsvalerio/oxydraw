import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteStaticCopy } from "vite-plugin-static-copy";
import { createRequire } from "node:module";
import path from "node:path";

// The editor loads its hand-drawn fonts from `window.EXCALIDRAW_ASSET_PATH` (set to "/" in
// index.html), so the package's bundled fonts must be served from our own origin. Copy them
// into the build output instead of pulling them from a CDN — this keeps OxyDraw fully
// self-hosted and offline-capable.
// The package blocks `./package.json` via its `exports` map, so locate the bundled fonts
// relative to the resolved main entry (dist/prod/index.js → dist/prod/fonts) instead.
const require = createRequire(import.meta.url);
const excalidrawProd = path.dirname(require.resolve("@excalidraw/excalidraw"));

// rust-embed picks the build output up from backend/crates/server/assets/ at `cargo build`
// time (same mechanism as the old vendored-app build; only the source changes).
export default defineConfig({
  base: "/",
  plugins: [
    react(),
    viteStaticCopy({
      targets: [
        {
          src: path.join(excalidrawProd, "fonts").replace(/\\/g, "/"),
          dest: ".",
        },
      ],
    }),
  ],
  build: {
    outDir: "../backend/crates/server/assets",
    emptyOutDir: true,
  },
});
