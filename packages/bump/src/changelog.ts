/**
 * 改写自 unjs/changelogen
 * @link https://github.com/unjs/changelogen
 * @licence https://github.com/unjs/changelogen/blob/main/LICENSE
 */
import {
  ChangelogConfig,
  generateMarkDown,
  getGitDiff,
  getLastGitTag,
  parseCommits,
} from 'changelogen'
import { existsSync } from 'node:fs'
import { readFile, writeFile } from 'node:fs/promises'

export const getTag = async () => {
  try {
    return await getLastGitTag()
  } catch (e) {
    return ''
  }
}
export const changelog = async (rawConfig: ChangelogConfig) => {
  const config = {
    ...rawConfig,
    newVersion: rawConfig.to,
    to: `v${rawConfig.to}`,
    from: `v${rawConfig.from}`,
  } as ChangelogConfig

  const rawCommits = await getGitDiff(config.from)

  const commits = parseCommits(rawCommits, config).filter(
    (c) => config.types[c.type] && !(c.type === 'chore' && c.scope === 'deps' && !c.isBreaking)
  )
  const markdown = await generateMarkDown(commits, config)

  // Update changelog file (only when bumping or releasing or when --output is specified as a file)
  let changelogMD: string
  if (existsSync(config.output as string)) {
    changelogMD = await readFile(config.output as string, 'utf8')
  } else {
    changelogMD = '# Changelog\n\n'
  }

  const lastEntry = changelogMD.match(/^###?\s+.*$/m)

  if (lastEntry) {
    changelogMD =
      changelogMD.slice(0, lastEntry.index) + markdown + '\n\n' + changelogMD.slice(lastEntry.index)
  } else {
    changelogMD += '\n' + markdown + '\n\n'
  }

  await writeFile(config.output as string, changelogMD)
  const { $ } = await import('execa')
  await $`git add ${config.output as string} package.json`
  await $`git commit -m ${[`chore: update ${config.output as string}`]}`
  return {
    markdown,
    changelogMD,
  }
}
