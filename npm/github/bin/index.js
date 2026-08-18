#!/usr/bin/env node
// argv 语法唯一归属 Rust——bin 仅透传 argv、注入平台变体身份并回写退出码
import { cliRun } from '@vill-v/bumpp-core'

process.exitCode = await cliRun(process.argv.slice(2), 'github')
