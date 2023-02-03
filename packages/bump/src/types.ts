import { ChangelogConfig } from 'changelogen'
import { VersionBumpOptions } from 'bumpp'

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
