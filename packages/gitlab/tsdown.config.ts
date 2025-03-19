import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    cli: 'src/cli.ts',
  },
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
