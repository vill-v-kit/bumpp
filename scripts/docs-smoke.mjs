/**
 * 文档站冒烟验证（COL-77）：ADR-0020 子路径部署约束的防回归手段。
 * 站点以 basePath 子路径部署到 GitHub Pages，运行时拼接的 URL 必须显式
 * 带 basePath——这条已两次漏掉（llms 生成物链接、静态搜索客户端 fetch），
 * 本脚本把它变成 CI 可执行的断言。
 *
 * 用法：
 *   node scripts/docs-smoke.mjs assert-artifacts <outDir> <siteBaseUrl>
 *     构建后、部署前对导出产物做静态断言：
 *       1. <outDir>/api/search 存在且非空（静态搜索索引已导出）
 *       2. 产物 JS bundle 中出现 "<basePath>/api/search" 或模板字面量形态
 *          `${…basePath…}/api/search`（搜索客户端显式携带 basePath 抓取
 *          索引；fumadocs staticClient 默认回退 '/api/search'，子路径部署
 *          下客户端会 404。注意产物里同时存在库代码内嵌的默认形参
 *          '/api/search'，无法靠负向 grep 区分，所以这里只做正向断言）
 *       3. llms.txt / llms-full.txt 中所有同源绝对链接都以 siteBaseUrl
 *          为前缀（llms 生成物链接必须带 basePath）
 *
 *   node scripts/docs-smoke.mjs check-live <siteBaseUrl>
 *     部署后轮询线上关键资源直到全部 200（Pages 有传播延迟，需重试）：
 *       <siteBaseUrl>/、<siteBaseUrl>/api/search、<siteBaseUrl>/llms.txt
 *     轮询预算可用环境变量覆盖（测试 stub 用）：
 *       DOCS_SMOKE_ROUNDS（默认 18）、DOCS_SMOKE_INTERVAL_MS（默认 10000）
 *
 * 退出码契约：0 通过；1 断言失败 / 轮询超时；2 用法错误。
 */
import { readdir, readFile, stat } from 'node:fs/promises'
import { join } from 'node:path'

const SMOKE_UA = 'vill-v-kit/bumpp docs-smoke (https://github.com/vill-v-kit/bumpp)'
const LIVE_PATHS = ['/', '/api/search', '/llms.txt']

const [command, ...args] = process.argv.slice(2)

function usage() {
  console.error('usage:')
  console.error('  node scripts/docs-smoke.mjs assert-artifacts <outDir> <siteBaseUrl>')
  console.error('  node scripts/docs-smoke.mjs check-live <siteBaseUrl>')
  process.exit(2)
}

/** 解析站点 base URL，返回 { origin, basePath, base }（base 无尾斜杠） */
function parseSiteBase(raw) {
  let url
  try {
    url = new URL(raw)
  } catch {
    console.error(`invalid siteBaseUrl: ${raw}`)
    process.exit(2)
  }
  const basePath = url.pathname.replace(/\/+$/, '')
  return { origin: url.origin, basePath, base: `${url.origin}${basePath}` }
}

async function assertArtifacts(outDir, siteBaseUrl) {
  const { origin, base, basePath } = parseSiteBase(siteBaseUrl)
  const failures = []

  // 1. 搜索索引已导出且非空
  const indexPath = join(outDir, 'api', 'search')
  try {
    const info = await stat(indexPath)
    if (info.size === 0) failures.push(`${indexPath} 是空文件（搜索索引导出失败）`)
  } catch {
    failures.push(`${indexPath} 不存在（静态搜索索引未导出）`)
  }

  // 2. bundle 正向断言：搜索客户端携带 basePath 抓取索引
  //    两种打包形态都算数——内联字面量 "<basePath>/api/search"（旧），或经
  //    运行时 basePath 导出拼接的模板字面量 `${…basePath…}/api/search`（新）
  const expectedLiteral = `"${basePath}/api/search"`
  const expectedTemplate = 'basePath}/api/search`'
  const jsFiles = await collectFiles(outDir, (name) => name.endsWith('.js'))
  let found = false
  for (const file of jsFiles) {
    const content = await readFile(file, 'utf8')
    if (
      content.includes(expectedLiteral) ||
      content.includes(expectedTemplate)
    ) {
      found = true
      break
    }
  }
  if (!found) {
    failures.push(
      `产物 bundle 中未出现 ${expectedLiteral} 或 ${expectedTemplate}——搜索客户端可能丢了 basePath` +
        '（fumadocs staticClient 默认回退 /api/search，子路径部署下会 404）',
    )
  }

  // 3. llms 生成物的同源绝对链接必须带 basePath
  for (const name of ['llms.txt', 'llms-full.txt']) {
    const file = join(outDir, name)
    let content
    try {
      content = await readFile(file, 'utf8')
    } catch {
      continue // 文件不存在不视为失败（生成物集合可能调整）
    }
    const links = content.match(/https?:\/\/[^\s)\]>"']+/g) ?? []
    for (const link of links) {
      if (link.startsWith(origin) && link !== base && !link.startsWith(`${base}/`)) {
        failures.push(`${name} 中同源链接未带 basePath: ${link}`)
      }
    }
  }

  return failures
}

async function collectFiles(dir, predicate) {
  const out = []
  let entries
  try {
    entries = await readdir(dir, { withFileTypes: true })
  } catch {
    return out
  }
  for (const entry of entries) {
    const path = join(dir, entry.name)
    if (entry.isDirectory()) out.push(...(await collectFiles(path, predicate)))
    else if (predicate(entry.name)) out.push(path)
  }
  return out
}

async function checkLive(siteBaseUrl) {
  const { base } = parseSiteBase(siteBaseUrl)
  const rounds = Number(process.env.DOCS_SMOKE_ROUNDS ?? 18)
  const intervalMs = Number(process.env.DOCS_SMOKE_INTERVAL_MS ?? 10_000)

  const pending = new Map(LIVE_PATHS.map((path) => [`${base}${path}`, null]))
  for (let round = 1; round <= rounds && pending.size > 0; round++) {
    for (const [url] of pending) {
      try {
        const res = await fetch(url, { headers: { 'user-agent': SMOKE_UA } })
        if (res.status === 200) {
          console.log(`OK ${url}`)
          pending.delete(url)
        } else {
          pending.set(url, `HTTP ${res.status}`)
        }
      } catch (err) {
        pending.set(url, err.cause?.code ?? err.message)
      }
    }
    if (pending.size > 0 && round < rounds) {
      await new Promise((resolve) => setTimeout(resolve, intervalMs))
    }
  }

  if (pending.size === 0) return []
  return [...pending].map(([url, reason]) => `${url} 轮询 ${rounds} 轮后仍未 200（最后状态: ${reason}）`)
}

let failures
if (command === 'assert-artifacts' && args.length === 2) {
  failures = await assertArtifacts(args[0], args[1])
} else if (command === 'check-live' && args.length === 1) {
  failures = await checkLive(args[0])
} else {
  usage()
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`FAIL ${failure}`)
  process.exit(1)
}
console.log(`docs-smoke ${command}: all checks passed`)
