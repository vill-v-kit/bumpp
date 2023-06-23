import { resolveRepoConfig as _resolveRepoConfig } from '@vill-v/bumpp'
import { read, readUser } from 'rc9'
export const resolveRepoConfig = async () => {
  const { repo: _repo } = await _resolveRepoConfig(process.cwd())
  if (_repo) {
    const [owner, repo] = _repo.split('/')
    return {
      owner,
      repo,
    }
  }
}

export interface GiteeConfig {
  access_token: string
}
export const loadGiteeConfig = () => {
  const config = read<GiteeConfig>('.giteerc')
  const userConfig = readUser<GiteeConfig>('.giteerc')
  if (Object.keys(config).length) {
    return config
  }
  return userConfig
}

export const isPreRelease = (v: string) => {
  return /(beta|alpha)/.test(v)
}
