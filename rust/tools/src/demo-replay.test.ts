// The demo fixtures under another organisation id, as a run against an existing database opens
// them (src/demo-replay.ts#demoFixtures).
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { DEMO_ORG, demoFixtures } from './demo-replay'

describe('demoFixtures', () => {
  test('keeps the fixture organisation by default', () => {
    expect(demoFixtures().org).toBe(DEMO_ORG)
  })

  test('moves the demo books and the command log to another organisation', () => {
    const fx = demoFixtures('org-g3-test')
    expect(fx.org).toBe('org-g3-test')
    expect(((fx.commands.final_state as Record<string, unknown>).settings as Record<string, unknown>).organization_id).toBe('org-g3-test')
    expect(JSON.stringify([fx.commands, fx.demoState])).not.toContain(DEMO_ORG)
  })
})
