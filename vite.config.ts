import { defineConfig, lazyPlugins } from "vite-plus";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// The whole web toolchain (Vite+): `vp dev` / `vp build` for Tauri, `vp test`, and `vp check` (format, lint, types).
// Tauri serves the dev build from a fixed port and reads the production build from dist-web/.
export default defineConfig({
  // Loaded only when Vite runs (dev, build, test), not when vp reads the lint/fmt settings.
  plugins: lazyPlugins(() => [react(), tailwindcss()]),
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  clearScreen: false,
  // Cargo writes thousands of files under src-tauri/target while `tauri dev` runs: none of them is a module.
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { outDir: "dist-web", target: "es2022" },
  test: { environment: "node" },
  lint: {
    // Oxlint's default plugins, plus React's rules.
    plugins: ["eslint", "typescript", "unicorn", "oxc", "react"],
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error", "react/rules-of-hooks": "error" },
    // Type checking (tsgolint, on TypeScript 7) replaces `tsc --noEmit`.
    options: { typeAware: true, typeCheck: true },
  },
  // Markdown keeps its hand-aligned tables and lists.
  fmt: { printWidth: 120, ignorePatterns: ["*.md"] },
});
