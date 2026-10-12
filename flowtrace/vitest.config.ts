import { defineConfig } from 'vitest/config'
import viteReact from '@vitejs/plugin-react'
import viteTsConfigPaths from 'vite-tsconfig-paths'

// Separate from vite.config.ts so tests don't load the TanStack Start and
// Nitro plugins, which only make sense for the app build.
export default defineConfig({
  plugins: [viteTsConfigPaths({ projects: ['./tsconfig.json'] }), viteReact()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    reporters: process.env.CI ? ['default', 'junit'] : ['default'],
    outputFile: { junit: 'test-results/junit.xml' },
  },
})
