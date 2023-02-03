import { defineConfig } from 'tsup'
import pkg from '../../package.json'
export default defineConfig({
  entry: {
    index: 'src/index.ts',
    cli: 'src/cli.ts',
  },
  sourcemap: false,
  clean: true,
  dts: true,
  target: 'node16',
  splitting: true,
  treeshake: true,
  shims: true,
  platform: 'node',
  format: ['esm', 'cjs'],
  define: {
    __version__: `"${pkg.version}"`,
  },
})
