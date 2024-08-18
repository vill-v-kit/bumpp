import { defineConfig } from 'tsup'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    cli: 'src/cli.ts',
  },
  clean: true,
  dts: true,
  target: 'node18',
  splitting: true,
  treeshake: true,
  shims: true,
  platform: 'node',
  format: ['esm', 'cjs'],
})
