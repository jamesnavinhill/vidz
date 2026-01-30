import solid from 'eslint-plugin-solid';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';

export default [
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: {
      solid,
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...solid.configs['typescript'].rules,
      ...tseslint.configs.recommended.rules,
      'solid/reactivity': 'error',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
    },
  },
  {
    files: ['src/store.ts'],
    rules: {
      'solid/reactivity': 'off',
    },
  },
];