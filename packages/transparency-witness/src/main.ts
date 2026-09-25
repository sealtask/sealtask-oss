import { readFile } from 'node:fs/promises'
import { auditOnce, WitnessAuditError, type AuditorConfig } from './auditor'
import { fetchBoundedWitnessText } from '@sealtask/crypto-web/trust/transparency-witness'

// The systemd unit holds flock for the whole process, including initialization.
const [configPath, operation] = process.argv.slice(2)
try {
  if (!configPath || (operation !== 'audit' && operation !== 'initialize')) {
    throw new Error('Usage: transparency-witness CONFIG.json audit|initialize (under flock)')
  }
  const config: AuditorConfig = JSON.parse(await readFile(configPath, 'utf8'))
  const result = await auditOnce(config, async (url) => JSON.parse(await fetchBoundedWitnessText(url)), {
    initialize: operation === 'initialize',
  })
  console.log(JSON.stringify({ event: 'checkpoint_verified', ...result }))
} catch (error) {
  // Neither network bodies nor private key/parser errors belong in telemetry.
  console.error(JSON.stringify({ event: 'checkpoint_verification_failed', stage: error instanceof WitnessAuditError ? error.stage : 'configuration' }))
  process.exitCode = 1
}
