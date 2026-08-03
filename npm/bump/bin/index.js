#!/usr/bin/env node
// ADR-0016：argv 语法唯一归属 Rust——bin 仅透传 argv 并回写退出码
import { cliRun } from '@vill-v/bumpp-core'

process.exitCode = await cliRun(process.argv.slice(2))
