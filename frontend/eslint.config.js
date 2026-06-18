import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  { ignores: ['dist', 'target', 'node_modules', 'public'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  // Browser source code.
  {
    files: ['src/**/*.{ts,tsx}'],
    extends: [reactHooks.configs.flat['recommended-latest']],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      'react-refresh': reactRefresh,
    },
    rules: {
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
      // New in react-hooks v7 (React Compiler era). Fires on idiomatic
      // patterns like `setLoading(true)` at the top of a fetch effect.
      // Surface it as advice without blocking `ops verify`.
      'react-hooks/set-state-in-effect': 'warn',
    },
  },
  // Node-context config files (vite.config.ts, etc.).
  {
    files: ['*.{ts,js,mts,mjs}'],
    languageOptions: {
      globals: globals.node,
    },
  },
)
