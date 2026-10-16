// Minimal ESLint policy for the few first-party JS files. Two contexts:
// verify-web/static runs in the BROWSER (ESM), package/ scripts run in
// NODE (CommonJS). eslint.config.mjs because the repo root has no
// package.json (Node would otherwise treat .js config as CommonJS).
const rules = {
  "no-undef": "error",
  "no-unused-vars": ["warn", { args: "none", caughtErrors: "none" }],
  "no-cond-assign": "error",
  "no-var": "error",
  "no-constant-condition": ["error", { checkLoops: false }],
  "no-fallthrough": "error",
};

export default [
  {
    files: ["verify-web/static/*.js"],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: "module",
      globals: {
        TextDecoder: "readonly",
        TextEncoder: "readonly",
        atob: "readonly",
        crypto: "readonly",
        fetch: "readonly",
        console: "readonly",
        document: "readonly",
        window: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        File: "readonly",
        Uint8Array: "readonly",
      },
    },
    rules,
  },
  {
    files: ["package/*.js"],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: "commonjs",
      globals: {
        require: "readonly",
        module: "readonly",
        process: "readonly",
        __dirname: "readonly",
        console: "readonly",
        Buffer: "readonly",
      },
    },
    rules,
  },
];
