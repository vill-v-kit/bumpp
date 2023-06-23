import { ChangelogConfig } from 'changelogen'
import { VersionBumpOptions, versionBumpInfo } from 'bumpp'
import { changelog } from './changelog'
export interface ChangelogOptions
  extends Omit<Partial<ChangelogConfig>, 'cwd' | 'github' | 'newVersion' | 'to' | 'from'> {}

export interface Config {
  changelog?: ChangelogOptions
  bumpp?: Omit<VersionBumpOptions, 'progress' | 'cwd'>
}

export interface ResolveConfig {
  changelog: ChangelogConfig
  bumpp: Omit<VersionBumpOptions, 'progress'>
}

export type BumppResult = Awaited<ReturnType<typeof versionBumpInfo>>
export type ChangelogResult = Awaited<ReturnType<typeof changelog>>
export interface BumpVersion {
  bumpp: BumppResult['state']
  changelog: ChangelogResult
}
