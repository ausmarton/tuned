import js from "@eslint/js";
import tseslint from "typescript-eslint";
import globals from "globals";

export default tseslint.config(
  { ignores: ["dist", "pkg", "node_modules"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: { ...globals.browser, ...globals.worker },
    },
  },
  {
    files: ["src/audio/worklet/*.js"],
    languageOptions: {
      globals: { ...globals.worker, AudioWorkletProcessor: "readonly", registerProcessor: "readonly", sampleRate: "readonly" },
    },
  },
);
