// token 存储 golden fixture 生成器——npm/bump accesstoken.ts
// 二进制格式的 verbatim 复刻（格式冻结证据：magic "VBTK" | version | iv | authTag | ct）。
// 密钥与 iv 取固定值使产物可复现；真实运行时为随机生成。
// 用法: node tests/fixtures/token-gen.ts
import { createCipheriv } from 'node:crypto'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const STORE_MAGIC = Buffer.from('VBTK')
const STORE_VERSION = 1
const outDir = join(dirname(fileURLToPath(import.meta.url)), 'token')

const key = Buffer.from([...Array(32).keys()])
const iv = Buffer.from([...Array(12).keys()].map((i) => i + 100))
const tokens = {
  gitee: 'test-token-gitee-123',
  github: 'test-token-github-456',
}

const cipher = createCipheriv('aes-256-gcm', key, iv)
const data = Buffer.concat([cipher.update(JSON.stringify(tokens), 'utf8'), cipher.final()])
const blob = Buffer.concat([STORE_MAGIC, Buffer.from([STORE_VERSION]), iv, cipher.getAuthTag(), data])

mkdirSync(outDir, { recursive: true })
writeFileSync(join(outDir, 'key.bin'), key)
writeFileSync(join(outDir, 'tokens.bin'), blob)
writeFileSync(join(outDir, 'expected.json'), JSON.stringify(tokens, null, 2) + '\n')
console.log('fixture written to', outDir)
