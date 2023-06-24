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
import { $ } from 'execa'

/**
 * 获取远程仓库的最新tag
 * @returns
 */
export const getTag = async () => {
  try {
    return await getLastGitTag()
  } catch (e) {
    return ''
  }
}

/**
 * 生成changelog
 * @param rawConfig
 */
export const changelog = async (rawConfig: ChangelogConfig) => {
  const config = {
    ...rawConfig,
    newVersion: rawConfig.to,
    to: `v${rawConfig.to}`,
    from: `v${rawConfig.from}`,
  } as ChangelogConfig

  // 获取当前最新提交与上个版本的所有提交信息
  const rawCommits = await getGitDiff(config.from)

  // 格式化所有提交信息
  const commits = parseCommits(rawCommits, config).filter(
    (c) => config.types[c.type] && !(c.type === 'chore' && c.scope === 'deps' && !c.isBreaking)
  )

  // 提交信息生成 markdown
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

  // 更新 changelog
  await writeFile(config.output as string, changelogMD)
  // 将changelog 文件和 package.json 提供给 git暂存
  await $`git add ${config.output as string} package.json`
  // 添加 changelog 提交信息
  await $`git commit -m ${[`chore: update ${config.output as string}`]}`
  return {
    // 当前发版的markdown信息
    markdown,
    // changelog 所有的信息
    changelogMD,
  }
}
