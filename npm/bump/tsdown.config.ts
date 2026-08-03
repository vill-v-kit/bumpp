import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    cli: 'src/cli.ts',
    changelog: 'src/changelog.ts',
  },
  dts: true,
  target: 'node20',
  clean: true,
  treeshake: true,
  shims: true,
  platform: 'node',
  format: ['esm'],
  publint: true,
  unused: true,
  deps: {
    skipNodeModulesBundle: true,
  },
  exports: true,
  fixedExtension: false,
})
