import { expect, it } from 'vitest'
import { plus100 } from './index.js'

it('loads the native napi addon and calls into rust', () => {
  expect(plus100(1)).toBe(101)
})
