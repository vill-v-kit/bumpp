import { createCipheriv, createDecipheriv, randomBytes, scryptSync } from 'node:crypto'
import { existsSync } from 'node:fs'
import { chmod, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'

/**
 * 派生密钥使用的内嵌盐
 * 注意：本方案防护级别为「防明文落盘」，密钥由机器信息派生，
 * 持有存储文件与本源码的人可以解密，请勿将其视作高安全保险柜
 */
const SALT = 'vbumpp/token-store'

/**
 * 二进制存储文件的 magic 头
 */
const STORE_MAGIC = Buffer.from('VBTK')

/**
 * 二进制存储格式版本
 */
const STORE_VERSION = 1

/**
 * token 独立二进制存储文件路径（可用 VBUMPP_TOKEN_STORE 覆盖）
 */
const getStorePath = (): string =>
  process.env.VBUMPP_TOKEN_STORE || path.join(os.homedir(), '.vbumpp', 'tokens.bin')

/**
 * 基于机器信息派生加密密钥
 */
const deriveKey = (): Buffer =>
  scryptSync(`${os.hostname()}:${os.userInfo().username}:${os.homedir()}`, SALT, 32)

/**
 * 加密为二进制存储格式
 * 布局: magic "VBTK"(4B) | version(1B) | iv(12B) | authTag(16B) | ciphertext
 * @param plain 明文
 */
const encryptBuffer = (plain: Buffer): Buffer => {
  const iv = randomBytes(12)
  const cipher = createCipheriv('aes-256-gcm', deriveKey(), iv)
  const data = Buffer.concat([cipher.update(plain), cipher.final()])
  return Buffer.concat([STORE_MAGIC, Buffer.from([STORE_VERSION]), iv, cipher.getAuthTag(), data])
}

/**
 * 解密二进制存储格式
 * @param blob 存储文件内容
 */
const decryptToBuffer = (blob: Buffer): Buffer => {
  const headerLength = STORE_MAGIC.length + 1 + 12 + 16
  if (blob.length < headerLength || !blob.subarray(0, STORE_MAGIC.length).equals(STORE_MAGIC)) {
    throw new Error('token 存储文件格式不正确')
  }
  const version = blob[STORE_MAGIC.length]
  if (version !== STORE_VERSION) {
    throw new Error(`不支持的 token 存储版本: ${version}`)
  }
  const ivStart = STORE_MAGIC.length + 1
  const iv = blob.subarray(ivStart, ivStart + 12)
  const tag = blob.subarray(ivStart + 12, ivStart + 12 + 16)
  const data = blob.subarray(headerLength)
  const decipher = createDecipheriv('aes-256-gcm', deriveKey(), iv)
  decipher.setAuthTag(tag)
  return Buffer.concat([decipher.update(data), decipher.final()])
}

/**
 * 读取二进制存储中的全部 token（文件不存在时返回空对象）
 */
export const readTokenStore = async (): Promise<Record<string, string>> => {
  const storePath = getStorePath()
  if (!existsSync(storePath)) {
    return {}
  }
  const blob = await readFile(storePath)
  return JSON.parse(decryptToBuffer(blob).toString('utf8'))
}

/**
 * 将全部 token 整体加密写入二进制存储文件
 * @param tokens
 */
const writeTokenStore = async (tokens: Record<string, string>): Promise<void> => {
  const storePath = getStorePath()
  await mkdir(path.dirname(storePath), { recursive: true, mode: 0o700 })
  await writeFile(storePath, encryptBuffer(Buffer.from(JSON.stringify(tokens), 'utf8')))
  await chmod(storePath, 0o600)
}

/**
 * 保存 token 到独立二进制存储文件
 * @param name 平台标识（如 gitee / gitcode）
 * @param token 明文 token
 */
export const saveToken = async (name: string, token: string): Promise<void> => {
  // 存储文件损坏时从空配置重写，保证 token set 始终可用
  const tokens = await readTokenStore().catch(() => ({}))
  tokens[name] = token
  await writeTokenStore(tokens)
}

/**
 * 从二进制存储文件删除 token（清空后删除文件）
 * @param name 平台标识
 * @returns 是否实际删除
 */
export const removeToken = async (name: string): Promise<boolean> => {
  const tokens = await readTokenStore()
  if (!(name in tokens)) {
    return false
  }
  delete tokens[name]
  if (Object.keys(tokens).length) {
    await writeTokenStore(tokens)
  } else {
    await rm(getStorePath(), { force: true })
  }
  return true
}

