// IT1's command log from the web: the commands the browser sends and the results it gets back,
// read from the WebSocket frames Playwright sees (it1-frames.ts). Real frames, no network.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { gzipSync } from 'node:zlib'
import fixture from './it1-frames.fixture.json'
import { bsatnStrings, receivedResults, sentCommands, serverPayload } from './it1-frames'

const hex = (h: string) => new Uint8Array(Buffer.from(h, 'hex'))

/** A BSATN string: u32 little-endian byte length, then UTF-8. */
const str = (s: string) => {
  const b = Buffer.from(s, 'utf8')
  const len = Buffer.alloc(4)
  len.writeUInt32LE(b.length)
  return Buffer.concat([len, b])
}

describe('bsatnStrings', () => {
  test('finds length-prefixed strings between other bytes', () => {
    const bytes = Buffer.concat([Buffer.from([3, 3, 0, 0, 0]), str('run_command'), Buffer.from([0xa0, 1, 0, 0]), str('org-1'), str('')])
    expect(bsatnStrings(new Uint8Array(bytes))).toEqual(['run_command', 'org-1'])
  })

  test('skips byte runs that are not text', () => {
    const bytes = Buffer.concat([Buffer.from([5, 0, 0, 0, 0, 1, 2, 3, 4]), str('ok')])
    expect(bsatnStrings(new Uint8Array(bytes))).toEqual(['ok'])
  })
})

describe('serverPayload', () => {
  test('strips the compression tag and inflates gzip', () => {
    const body = Buffer.concat([str('my_command_result')])
    expect(Buffer.from(serverPayload(new Uint8Array(Buffer.concat([Buffer.from([0]), body]))))).toEqual(body)
    expect(Buffer.from(serverPayload(new Uint8Array(Buffer.concat([Buffer.from([2]), gzipSync(body)]))))).toEqual(body)
  })
})

describe('a real claim submission', () => {
  test('the sent frame is one run_command with the org, request id and command', () => {
    const [c] = sentCommands(hex(fixture.sent))
    expect(c.orgId).toBe('it1-explore')
    expect(c.requestId).toBe('e1fb3029-e457-46e2-a978-cde684673333')
    expect(c.command.type).toBe('claim.create')
    expect((c.command.input as { lines: Array<{ amount: number }> }).lines[0].amount).toBe(345000)
  })

  test('the received frame carries its result row', () => {
    expect(receivedResults(hex(fixture.received))).toEqual([
      { requestId: 'e1fb3029-e457-46e2-a978-cde684673333', result: { type: 'id', id: '2098e627-54a7-4613-b4c4-41f1c1124d09' } },
    ])
  })

  test('a frame of another kind yields nothing', () => {
    expect(sentCommands(hex(fixture.received))).toEqual([])
    expect(receivedResults(new Uint8Array([0]))).toEqual([])
  })
})
