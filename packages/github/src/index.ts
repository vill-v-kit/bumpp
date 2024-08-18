export * from './bump'
export * from './createRelease'
export { defineConfig } from '@vill-v/bumpp'
declare module '@vill-v/bumpp' {
  interface Accesstokens {
    github: string
  }
}
