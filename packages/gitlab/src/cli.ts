import { createBaseCli } from '@vill-v/bumpp/cli'
import { bumpVersion } from './bump'

export const createCli = () => createBaseCli(bumpVersion)
