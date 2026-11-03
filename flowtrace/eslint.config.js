//  @ts-check

import { tanstackConfig } from '@tanstack/eslint-config'

export default [
  {
    ignores: ['.output/**', '.vercel/**', 'dist/**', 'node_modules/**'],
  },
  ...tanstackConfig,
  {
    // Plain JavaScript outside the TypeScript project (tool configs and the
    // service worker): parse without type information and switch off the
    // rules that need it.
    files: ['*.config.js', 'public/**/*.js'],
    languageOptions: {
      parserOptions: { project: false },
    },
    rules: {
      '@typescript-eslint/no-for-in-array': 'off',
      '@typescript-eslint/no-unnecessary-condition': 'off',
      '@typescript-eslint/no-unnecessary-type-assertion': 'off',
      '@typescript-eslint/require-await': 'off',
    },
  },
]
