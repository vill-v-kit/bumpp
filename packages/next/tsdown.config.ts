import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts'],
  dts: { transformer: 'oxc' },
  target: 'node18',
  clean: true,
  treeshake: true,
  shims: true,
  platform: 'node',
  format: ['esm'],
  publint: true,
  skipNodeModulesBundle: true,
})
