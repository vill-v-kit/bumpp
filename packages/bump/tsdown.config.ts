import { defineConfig } from 'tsdown'
import pkg from '../../package.json'

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    cli: 'src/cli.ts',
    changelogens: 'src/changelogen.ts',
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
  define: {
    __version__: `"${pkg.version}"`,
  },
})
