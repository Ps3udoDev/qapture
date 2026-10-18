import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  {
    ignores: [
      'dist',
      'src-tauri/target',
      'node_modules',
      'src/infrastructure/tauri/__fixtures__',
      'videos',
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
    plugins: { 'react-hooks': reactHooks },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'error',
    },
  },
  {
    files: ['scripts/**/*.{js,mjs,ts}', 'vite.config.ts', 'eslint.config.js'],
    languageOptions: { globals: globals.node },
  },
)
