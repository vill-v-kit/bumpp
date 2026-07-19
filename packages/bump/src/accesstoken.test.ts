import { existsSync } from 'node:fs'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { readTokenStore, removeToken, saveToken } from './accesstoken'

let dir: string
let storePath: string

beforeAll(async () => {
  dir = await mkdtemp(path.join(tmpdir(), 'vbumpp-token-'))
  storePath = path.join(dir, 'tokens.bin')
  process.env.VBUMPP_TOKEN_STORE = storePath
})

afterAll(async () => {
  delete process.env.VBUMPP_TOKEN_STORE
  await rm(dir, { recursive: true, force: true })
})

describe('token 二进制存储', () => {
  it('保存后可正确读取（round-trip）', async () => {
    await saveToken('gitcode', 'test-token-123')
    await saveToken('gitee', 'another-token-456')
    const tokens = await readTokenStore()
    expect(tokens.gitcode).toBe('test-token-123')
    expect(tokens.gitee).toBe('another-token-456')
  })

  it('存储文件为二进制格式且不包含明文', async () => {
    await saveToken('gitcode', 'test-token-123')
    const blob = await readFile(storePath)
    expect(blob.subarray(0, 4).toString('utf8')).toBe('VBTK')
    expect(blob.includes('test-token-123')).toBe(false)
    expect(blob.includes('gitcode')).toBe(false)
  })

  it('magic 不正确时读取抛错', async () => {
    await writeFile(storePath, Buffer.from('not-a-valid-store-file'))
    await expect(readTokenStore()).rejects.toThrow('token 存储文件格式不正确')
  })

  it('密文被篡改后读取抛错', async () => {
    await saveToken('gitcode', 'test-token-123')
    const blob = await readFile(storePath)
    blob[blob.length - 1] = blob[blob.length - 1] ^ 0xff
    await writeFile(storePath, blob)
    await expect(readTokenStore()).rejects.toThrow()
  })

  it('删除 token，清空后删除存储文件', async () => {
    await saveToken('gitcode', 'test-token-123')
    expect(await removeToken('gitcode')).toBe(true)
    expect(existsSync(storePath)).toBe(false)
    expect(await removeToken('gitcode')).toBe(false)
  })
})
