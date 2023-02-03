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
import consola from 'consola'

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
    consola.info(`Updating ${config.output}`)
    changelogMD = await readFile(config.output as string, 'utf8')
  } else {
    consola.info(`Creating  ${config.output}`)
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
  const { execa } = await import('execa')
  await execa('git', ['add', config.output as string, 'package.json'])
  await execa('git', ['commit', '-m', `chore: update ${config.output}`])
  consola.success('Update', config.output)
  return {
    markdown,
    changelogMD,
  }
}
