/**
 * 上架幂等守卫（COL-51，ADR-0021 决策⑤）：查询目标 registry 上「包名 + 版本」
 * 是否已存在，供 publish-npm / publish-crates 两个 CI job 以同一语义消费——
 * 部分上架失败后「Re-run failed jobs」时，已上架的包自动跳过、未上架的补发。
 *
 * 用法：node scripts/publish-guard.mjs <npm|crates> <包名> <版本>
 *
 * 退出码契约：
 *   0 + stdout GO …   —— 未上架，放行 publish
 *   1 + stdout SKIP … —— 已上架，跳过
 *   2 + stderr ERROR …—— 查询本身失败（网络错误 / 5xx / 非预期响应），
 *                        消费方必须视为失败（防止误判放行导致重复上架报错）
 *
 * 两路查询均为无凭证只读：npm 走 registry 的 per-version 文档，
 * crates.io 走公开 API（必须带 User-Agent，否则 403）。
 * registry base URL 可经环境变量覆盖（自建 registry / 测试 stub 通用）：
 *   PUBLISH_GUARD_NPM_URL（默认 https://registry.npmjs.org）
 *   PUBLISH_GUARD_CRATES_URL（默认 https://crates.io）
 */
const [registry, name, version] = process.argv.slice(2)

const GUARD_UA = 'vill-v-kit/bumpp publish-guard (https://github.com/vill-v-kit/bumpp)'

function targetUrl() {
  if (registry === 'npm') {
    const base = (process.env.PUBLISH_GUARD_NPM_URL ?? 'https://registry.npmjs.org').replace(/\/$/, '')
    // per-version 文档；scoped 名整体 percent-encode（@vill-v/x → %40vill-v%2Fx）
    return `${base}/${encodeURIComponent(name)}/${encodeURIComponent(version)}`
  }
  if (registry === 'crates') {
    const base = (process.env.PUBLISH_GUARD_CRATES_URL ?? 'https://crates.io').replace(/\/$/, '')
    return `${base}/api/v1/crates/${encodeURIComponent(name)}/${encodeURIComponent(version)}`
  }
  console.error(`unknown registry: ${registry} (expected npm | crates)`)
  process.exit(2)
}

if (!registry || !name || !version) {
  console.error('usage: node scripts/publish-guard.mjs <npm|crates> <name> <version>')
  process.exit(2)
}

let res
try {
  res = await fetch(targetUrl(), { headers: { 'user-agent': GUARD_UA } })
} catch (err) {
  console.error(`ERROR ${registry} query failed: ${err.cause?.code ?? err.message} for ${name}@${version}`)
  process.exit(2)
}
if (res.status === 200) {
  console.log(`SKIP ${name}@${version} (already on ${registry})`)
  process.exit(1)
}
if (res.status === 404) {
  console.log(`GO ${name}@${version} (not on ${registry})`)
  process.exit(0)
}
console.error(`ERROR ${registry} query failed: HTTP ${res.status} for ${name}@${version}`)
process.exit(2)
