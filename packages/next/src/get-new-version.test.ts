import { expect, test } from 'vitest'
import { semverNewVersion } from './get-new-version'

test('semverNewVersion', async () => {
  expect(semverNewVersion('1.2.2-rc.1')).toEqual({
    major: '2.0.0',
    minor: '1.3.0',
    patch: '1.2.2',
    next: '1.2.2-rc.2',
    premajor: '2.0.0-rc.1',
    preminor: '1.3.0-rc.1',
    prepatch: '1.2.3-rc.1',
    prerelease: '1.2.2-rc.2',
  })
  expect(semverNewVersion('1.2.2')).toEqual({
    major: '2.0.0',
    minor: '1.3.0',
    patch: '1.2.3',
    next: '1.2.3',
    premajor: '2.0.0-beta.1',
    preminor: '1.3.0-beta.1',
    prepatch: '1.2.3-beta.1',
    prerelease: '1.2.3-beta.1',
  })
  expect(semverNewVersion('1.2.2-beta.0')).toEqual({
    major: '2.0.0',
    minor: '1.3.0',
    patch: '1.2.2',
    next: '1.2.2-beta.1',
    premajor: '2.0.0-beta.1',
    preminor: '1.3.0-beta.1',
    prepatch: '1.2.3-beta.1',
    prerelease: '1.2.2-beta.1',
  })

  expect(semverNewVersion('0.0.0')).toEqual({
    major: '1.0.0',
    minor: '0.1.0',
    patch: '0.0.1',
    next: '0.0.1',
    premajor: '1.0.0-beta.1',
    preminor: '0.1.0-beta.1',
    prepatch: '0.0.1-beta.1',
    prerelease: '0.0.1-beta.1',
  })
})
