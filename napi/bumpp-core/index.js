// @vill-v/bumpp-core 原生绑定加载器（自研，替代 napi-rs 生成物——
// 生成的 loader 报错不含已支持平台列表，不满足本仓库的报错可读性要求）。
//
// 解析顺序：NAPI_RS_NATIVE_LIBRARY_PATH → 平台包（optionalDependencies）→ 本包根目录
// 的本地构建产物（开发内循环 `pnpm build` 只编本机 target）。

import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const pkg = require('./package.json')

/**
 * 预编译平台包清单：从 optionalDependencies 声明派生（@vill-v/bumpp-core-<triple>），
 * 与 GitHub Actions 构建矩阵一一对应；triple 形如 `linux-x64-gnu` → (platform, arch, abi)
 */
const SUPPORTED_TARGETS = Object.keys(pkg.optionalDependencies ?? {})
  .map((name) => name.match(/^@vill-v\/bumpp-core-(.+)$/)?.[1])
  .filter(Boolean)
  .map((triple) => {
    const [platform, arch, abi] = triple.split('-')
    return { platform, arch, abi, triple }
  })

const isMusl = () => {
  if (process.platform !== 'linux') return false
  try {
    return readFileSync('/usr/bin/ldd', 'utf8').includes('musl')
  } catch {
    return false
  }
}

function requireNative() {
  if (process.env.NAPI_RS_NATIVE_LIBRARY_PATH) {
    return require(process.env.NAPI_RS_NATIVE_LIBRARY_PATH)
  }

  const errors = []
  for (const target of SUPPORTED_TARGETS) {
    if (target.platform !== process.platform || target.arch !== process.arch) continue
    if (target.abi === 'gnu' && isMusl()) continue // musl 不在支持矩阵内

    const packageName = `@vill-v/bumpp-core-${target.triple}`
    let binding
    try {
      binding = require(packageName)
    } catch (err) {
      errors.push(err)
    }
    if (binding) {
      // 版本不匹配是配置错误：直接抛可读错误，不再回退（陈旧 optionalDep 的常见症状）
      const bindingVersion = require(`${packageName}/package.json`).version
      if (bindingVersion !== pkg.version) {
        throw new Error(
          `platform package ${packageName} version mismatch: expected ${pkg.version}, got ${bindingVersion} — reinstall dependencies`,
        )
      }
      return binding
    }

    try {
      return require(`./bumpp-core.${target.triple}.node`)
    } catch (err) {
      errors.push(err)
    }
  }

  const current = `${process.platform}-${process.arch}${isMusl() ? '-musl' : ''}`
  const supported = SUPPORTED_TARGETS.map((t) => t.triple).join(', ')
  const error = new Error(
    `@vill-v/bumpp-core cannot load the native binding on the current platform (${current}).\n` +
      `Supported platforms: ${supported}.\n` +
      'If the current platform is listed, this is most likely the optionalDependencies ' +
      'install flaw (https://github.com/npm/cli/issues/4828) — ' +
      'delete pnpm-lock.yaml and node_modules, then reinstall.',
  )
  error.cause = errors[errors.length - 1]
  throw error
}

const nativeBinding = requireNative()

// ADR-0016：平台 Release 四导出已删（独立 release 由 CLI `vbumpp release` 承接）
const { bumpVersion, cliRun } = nativeBinding
export { bumpVersion, cliRun }
