export * from './bump'
export { defineConfig } from '@vill-v/bumpp'
declare module '@vill-v/bumpp' {
  interface Accesstokens {
    gitcode: string
  }
}
