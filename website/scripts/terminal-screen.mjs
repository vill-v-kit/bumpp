#!/usr/bin/env node
// 终端屏幕模拟：把 pty 原始字节流（含 dialoguer FuzzySelect 的 ANSI 重绘帧）
// 塌缩成「最终可见屏幕」纯文本。capture-home-demo.sh 的捕获后处理步骤。
//
// 只实现捕获所需的转义子集：
//   CSI {n}A / {n}B  光标上/下移（菜单重绘）
//   CSI {n}G         光标到列
//   CSI [0]K / 2K    清到行尾 / 清整行
//   CR / LF / BS     回车、换行、退格
//   SGR 及其余 CSI、OSC、私有序列（?25l 等）一律忽略；其他控制字符忽略。
// 用法：node terminal-screen.mjs <raw-input> <screen-output>

import { readFileSync, writeFileSync } from 'node:fs';

const [, , inputPath, outputPath] = process.argv;
if (!inputPath || !outputPath) {
  console.error('usage: node terminal-screen.mjs <raw-input> <screen-output>');
  process.exit(2);
}

const text = new TextDecoder('utf-8').decode(readFileSync(inputPath));

/** @type {string[]} 屏幕行（懒增长） */
const rows = [];
let row = 0;
let col = 0;

const ensureRow = () => {
  while (rows.length <= row) rows.push('');
};

const putChar = (ch) => {
  ensureRow();
  const line = rows[row];
  rows[row] =
    line.length < col
      ? line.padEnd(col) + ch
      : line.slice(0, col) + ch + line.slice(col + 1);
  col += 1;
};

let i = 0;
while (i < text.length) {
  const ch = text[i];
  const code = text.charCodeAt(i);

  if (ch === '\x1b') {
    const next = text[i + 1];
    if (next === '[') {
      // CSI：参数直到最终字节（@–~）
      let j = i + 2;
      while (j < text.length && !/[@-~]/.test(text[j])) j += 1;
      const params = text.slice(i + 2, j);
      const final = text[j];
      const n = Number.parseInt(params, 10) || 1;
      if (final === 'A') row = Math.max(0, row - n);
      else if (final === 'B') row += n;
      else if (final === 'G') col = Math.max(0, n - 1);
      else if (final === 'K') {
        ensureRow();
        rows[row] = params === '2' ? '' : rows[row].slice(0, col);
      }
      i = j + 1;
      continue;
    }
    if (next === ']') {
      // OSC：跳过至 BEL 或 ESC\
      let j = i + 2;
      while (
        j < text.length &&
        text[j] !== '\x07' &&
        !(text[j] === '\x1b' && text[j + 1] === '\\')
      ) {
        j += 1;
      }
      i = text[j] === '\x07' ? j + 1 : j + 2;
      continue;
    }
    i += 2; // 其余两字节 ESC 序列忽略
    continue;
  }

  if (ch === '\r') {
    col = 0;
  } else if (ch === '\n') {
    row += 1;
  } else if (ch === '\b') {
    col = Math.max(0, col - 1);
  } else if (code >= 0x20 && code !== 0x7f) {
    putChar(ch);
  }
  // 其余控制字符（pty 回显的 ^D 等）忽略
  i += 1;
}

// 尾空行丢弃、逐行右 trim，单换行结尾
while (rows.length > 0 && rows[rows.length - 1].trimEnd() === '') rows.pop();
writeFileSync(outputPath, rows.map((line) => line.trimEnd()).join('\n') + '\n');
