// The web's commands and their results, read from the WebSocket frames Playwright sees, so IT1
// can replay exactly what the browser sent through the ledger-core oracle.
//
// The web calls `run_command(org_id, request_id, command_json)` (flowtrace/src/lib/ledger/
// spacetime-backend.ts, `run`) and waits for its `my_command_result` row. The SDK speaks BSATN
// (https://spacetimedb.com/docs/bsatn): a string is a u32 little-endian byte length and UTF-8.
// A client frame holds the reducer name and the arguments in order; a server frame starts with
// a compression tag (0 none, 1 brotli, 2 gzip; the web asks for gzip) and holds each changed row
// with the view's name before it. Rather than decode every message type, this finds the
// length-prefixed strings in order and reads the two sequences IT1 needs:
//
//   sent:     run_command, <org>, <request id>, <command json>
//   received: my_command_result, <org>, <request id>, <result json>
import { brotliDecompressSync, gunzipSync } from 'node:zlib'

export type Command = { type: string } & Record<string, unknown>

/** Printable UTF-8 (tab and newline allowed), no other control characters. */
const TEXT = /^[^\u0000-\u0008\u000b-\u001f\u007f]+$/u

/** Every non-empty string in `bytes`, in order: a u32 LE length followed by that much text. */
export function bsatnStrings(bytes: Uint8Array): Array<string> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const decoder = new TextDecoder('utf-8', { fatal: true })
  const out: Array<string> = []
  let i = 0
  while (i + 4 < bytes.length) {
    const len = view.getUint32(i, true)
    if (len > 0 && i + 4 + len <= bytes.length) {
      try {
        const s = decoder.decode(bytes.subarray(i + 4, i + 4 + len))
        if (TEXT.test(s)) {
          out.push(s)
          i += 4 + len
          continue
        }
      } catch {
        // Not UTF-8: not a string here.
      }
    }
    i++
  }
  return out
}

/** A server frame's message: the compression tag stripped and the body inflated. */
export function serverPayload(frame: Uint8Array): Uint8Array {
  const body = frame.subarray(1)
  if (frame[0] === 2) return new Uint8Array(gunzipSync(body))
  if (frame[0] === 1) return new Uint8Array(brotliDecompressSync(body))
  return body
}

const parseObject = (s: string | undefined): Record<string, unknown> | undefined => {
  if (!s?.startsWith('{')) return undefined
  try {
    const v = JSON.parse(s) as unknown
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : undefined
  } catch {
    return undefined
  }
}

/** The `run_command` calls in a frame the browser sent. */
export function sentCommands(frame: Uint8Array): Array<{ orgId: string; requestId: string; command: Command }> {
  const s = bsatnStrings(frame)
  const out: Array<{ orgId: string; requestId: string; command: Command }> = []
  s.forEach((x, i) => {
    const command = parseObject(s[i + 3])
    if (x === 'run_command' && typeof command?.type === 'string') out.push({ orgId: s[i + 1], requestId: s[i + 2], command: command as Command })
  })
  return out
}

/** The `my_command_result` rows in a frame the browser received. */
export function receivedResults(frame: Uint8Array): Array<{ requestId: string; result: Record<string, unknown> }> {
  if (frame.length < 2) return []
  let s: Array<string>
  try {
    s = bsatnStrings(serverPayload(frame))
  } catch {
    return []
  }
  const out: Array<{ requestId: string; result: Record<string, unknown> }> = []
  s.forEach((x, i) => {
    const result = parseObject(s[i + 3])
    if (x === 'my_command_result' && typeof result?.type === 'string') out.push({ requestId: s[i + 2], result })
  })
  return out
}
