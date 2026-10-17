import { describe, expect, test } from 'bun:test'
import { decodePayload, routeToken } from '../src/auth'
import type { Env, ImagesLike } from '../src/env'
import { isLinkId, isOrgId, parseKey, sqlString } from '../src/keys'
import { docData, findMember } from '../src/module'
import { sign, signedUrl, verify } from '../src/sign'
import { sniff } from '../src/sniff'
import { decodeOption, decodeResponse, normaliseIdentity } from '../src/sql'
import { makeThumbnail } from '../src/thumbnail'
import { callerVisibility, userPermissions, visibility, type Member } from '../src/visibility'
import {
  DOC_SCHEMA,
  HTML,
  ID,
  JPEG,
  MEMBER_SCHEMA,
  ORG,
  PDF,
  PNG,
  bytesOf,
  docRow,
  group,
  groupMember,
  memberRow,
  stdbToken,
} from './fixtures'

describe('sniff', () => {
  test('recognises each allowed type by its bytes', () => {
    expect(sniff(JPEG)).toBe('image/jpeg')
    expect(sniff(PNG)).toBe('image/png')
    expect(sniff(bytesOf('GIF89a'))).toBe('image/gif')
    expect(sniff(bytesOf('GIF87a'))).toBe('image/gif')
    expect(sniff(bytesOf('RIFF\x10\x00\x00\x00WEBPVP8 '))).toBe('image/webp')
    expect(sniff(bytesOf('\x00\x00\x00\x18ftypheic'))).toBe('image/heic')
    expect(sniff(bytesOf('\x00\x00\x00\x18ftypmif1'))).toBe('image/heif')
    expect(sniff(PDF)).toBe('application/pdf')
  })
  test('rejects everything else', () => {
    expect(sniff(HTML)).toBeNull()
    expect(sniff(bytesOf('PK\x03\x04'))).toBeNull()
    expect(sniff(bytesOf('\x00\x00\x00\x18ftypisom'))).toBeNull()
    expect(sniff(bytesOf('RIFF\x10\x00\x00\x00WAVE'))).toBeNull()
    expect(sniff(new Uint8Array([0xff, 0xd8]))).toBeNull()
    expect(sniff(new Uint8Array())).toBeNull()
  })
})

describe('SQL decoding', () => {
  test('option values in both encodings', () => {
    expect(decodeOption([0, 'x'])).toBe('x')
    expect(decodeOption([1, []])).toBeNull()
    expect(decodeOption({ some: 5 })).toBe(5)
    expect(decodeOption({ none: [] })).toBeNull()
  })
  test('identity shapes normalise to lowercase hex', () => {
    const hex = 'AB'.repeat(32)
    const want = 'ab'.repeat(32)
    expect(normaliseIdentity(`0x${hex}`)).toBe(want)
    expect(normaliseIdentity({ __identity__: `0x${hex}` })).toBe(want)
    expect(normaliseIdentity([`0x${hex}`])).toBe(want)
    expect(normaliseIdentity([{ __identity__: `0x${hex}` }])).toBe(want)
    expect(normaliseIdentity('nope')).toBeNull()
  })
  test('rows are keyed by schema position, options and identities decoded', () => {
    const optStr = {
      Sum: {
        variants: [
          { name: { some: 'some' }, algebraic_type: { String: [] } },
          { name: { some: 'none' }, algebraic_type: { Product: { elements: [] } } },
        ],
      },
    }
    const body = [
      {
        schema: {
          elements: [
            ...MEMBER_SCHEMA.elements.slice(0, 2),
            { name: { some: 'note' }, algebraic_type: optStr },
            { name: { some: 'other' }, algebraic_type: optStr },
          ],
        },
        rows: [['k1', [`0x${ID.alice}`], [0, 'hello'], { none: [] }]],
      },
    ]
    expect(decodeResponse(body)).toEqual([
      { key: 'k1', identity: ID.alice, note: 'hello', other: null },
    ])
  })
  test('member rows and doc rows', () => {
    const rows = decodeResponse([{ schema: MEMBER_SCHEMA, rows: [memberRow(ID.tom, 'tom')] }])
    expect(rows[0]).toMatchObject({ identity: ID.tom, user_id: 'tom', org_id: ORG, can_post: true })
    const docs = decodeResponse([
      { schema: DOC_SCHEMA, rows: [docRow(ORG, { id: 'v1' }), [`${ORG}:bad`, ORG, 'bad', 1, '{nope']] },
    ])
    expect(docData<{ id: string }>(docs)).toEqual([{ id: 'v1' }])
  })
})

describe('visibility', () => {
  const member = (userId: string): Member => ({
    key: `${ORG}:x`,
    identity: ID.alice,
    org_id: ORG,
    user_id: userId,
    name: userId,
    can_post: false,
    can_approve: false,
    can_close: false,
  })
  const groups = [
    group('g-nodes', { can_view_nodes: true }),
    group('g-proofs', { can_view_proofs: true }),
    group('g-full', { can_view_full_proofs: true }),
  ]
  test('levels in order', () => {
    expect(visibility({ can_view_full_proofs: true, can_view_proofs: false })).toBe('full')
    expect(visibility({ can_view_proofs: true })).toBe('thumbnail')
    expect(visibility({ can_view_nodes: true })).toBe('summary')
    expect(visibility({})).toBe('hidden')
  })
  test('flags are the union over every group the user is in', () => {
    const gms = [groupMember('m1', 'g-nodes', 'u'), groupMember('m2', 'g-proofs', 'u')]
    const p = userPermissions(groups, gms, 'u')
    expect(p.can_view_nodes && p.can_view_proofs && !p.can_view_full_proofs).toBe(true)
    expect(callerVisibility(groups, gms, member('u'))).toBe('thumbnail')
    gms.push(groupMember('m3', 'g-full', 'u'))
    expect(callerVisibility(groups, gms, member('u'))).toBe('full')
  })
  test('a non-member, or a member in no group, sees nothing', () => {
    const gms = [groupMember('m1', 'g-full', 'someone-else'), groupMember('m2', 'g-full', null)]
    expect(callerVisibility(groups, gms, null)).toBe('hidden')
    expect(callerVisibility(groups, gms, member('u'))).toBe('hidden')
  })
  test('validity dates are not checked', () => {
    const gms = [{ ...groupMember('m1', 'g-full', 'u'), valid_until: '2000-01-01T00:00:00Z' }]
    expect(callerVisibility(groups, gms, member('u'))).toBe('full')
  })
})

describe('signing', () => {
  const key = `org/${ORG}/${'0'.repeat(64)}`
  test('a fresh signature verifies; tampering or expiry fails', async () => {
    const sig = await sign('secret', key, 2000)
    expect(await verify('secret', key, '2000', sig, 1000)).toBe('ok')
    expect(await verify('secret', key, '2001', sig, 1000)).toBe('bad')
    expect(await verify('secret', `${key}x`, '2000', sig, 1000)).toBe('bad')
    expect(await verify('other', key, '2000', sig, 1000)).toBe('bad')
    const flipped = (sig[0] === '0' ? '1' : '0') + sig.slice(1)
    expect(await verify('secret', key, '2000', flipped, 1000)).toBe('bad')
    expect(await verify('secret', key, '2000', sig, 2001)).toBe('expired')
    expect(await verify('secret', key, null, sig, 1000)).toBe('bad')
    expect(await verify('secret', key, '2000', 'zz', 1000)).toBe('bad')
  })
  test('signed URLs last ten minutes', async () => {
    const { url, exp } = await signedUrl('secret', 'https://p.test', key, 1000)
    expect(exp).toBe(1600)
    expect(url).toStartWith(`https://p.test/signed/${key}?exp=1600&sig=`)
  })
})

describe('keys and ids', () => {
  const sha = 'f'.repeat(64)
  test('org ids', () => {
    expect(isOrgId('org-a')).toBe(true)
    expect(isOrgId('A.b_c-9')).toBe(true)
    expect(isOrgId('a'.repeat(128))).toBe(true)
    expect(isOrgId('a'.repeat(129))).toBe(false)
    for (const bad of ['', '-a', "a'b", 'a b', 'a/b', 'a;--', '.a']) expect(isOrgId(bad)).toBe(false)
    expect(sqlString("o'k")).toBe("'o''k'")
  })
  test('key shapes', () => {
    expect(parseKey(`org/${ORG}/${sha}`)).toEqual({ key: `org/${ORG}/${sha}`, org: ORG, sha256: sha, thumbnail: false })
    expect(parseKey(`org/${ORG}/thumb/${sha}.webp`)?.thumbnail).toBe(true)
    expect(parseKey(`org/${ORG}/${sha.toUpperCase()}`)).toBeNull()
    expect(parseKey(`org/${ORG}/thumb/${sha}.png`)).toBeNull()
    expect(parseKey(`org/../${sha}`)).toBeNull()
    expect(parseKey(`org/${ORG}/${sha}/x`)).toBeNull()
    expect(parseKey(`x/${ORG}/${sha}`)).toBeNull()
  })
  test('link ids', () => {
    expect(isLinkId('claim-1')).toBe(true)
    expect(isLinkId('')).toBe(false)
    expect(isLinkId('a'.repeat(129))).toBe(false)
    expect(isLinkId('a\nb')).toBe(false)
  })
})

describe('JWT routing', () => {
  const env = (over: Partial<Env> = {}) =>
    ({ WORKOS_ISSUER: 'https://auth.test', WORKOS_JWKS_URL: 'https://auth.test/jwks', ...over }) as Env
  test('SpacetimeDB payloads route by hex_identity', () => {
    const p = decodePayload(stdbToken(ID.alice))!
    expect(p.iss).toBe('localhost')
    expect(routeToken(env(), p)).toEqual({ path: 'spacetimedb', identity: ID.alice })
  })
  test('WorkOS payloads route by issuer, only when the path is on', () => {
    expect(routeToken(env(), { iss: 'https://auth.test', sub: 'u' })).toEqual({ path: 'workos' })
    expect(routeToken(env({ WORKOS_ISSUER: '' }), { iss: 'https://auth.test' })).toEqual({ path: 'none' })
    expect(routeToken(env({ AUTH_MODES: 'spacetimedb' }), { iss: 'https://auth.test' })).toEqual({ path: 'none' })
    expect(routeToken(env({ AUTH_MODES: 'workos' }), { hex_identity: ID.alice })).toEqual({ path: 'none' })
  })
  test('garbage is not a JWT', () => {
    expect(decodePayload('abc')).toBeNull()
    expect(decodePayload('a.!!!.c')).toBeNull()
    expect(routeToken(env(), { hex_identity: 'short' })).toEqual({ path: 'none' })
  })
  test('members match by identity or by WorkOS user id', () => {
    const list = [
      { key: 'k', identity: ID.alice, org_id: ORG, user_id: 'user_01', name: '', can_post: false, can_approve: false, can_close: false },
    ]
    expect(findMember(list, { kind: 'spacetimedb', identity: ID.alice })).not.toBeNull()
    expect(findMember(list, { kind: 'spacetimedb', identity: ID.tom })).toBeNull()
    expect(findMember(list, { kind: 'workos', userId: 'user_01' })).not.toBeNull()
    expect(findMember(list, { kind: 'workos', userId: ID.alice })).toBeNull()
  })
})

describe('thumbnail', () => {
  const stub = (fail = false): ImagesLike & { calls: unknown[] } => {
    const calls: unknown[] = []
    return {
      calls,
      input: (stream) => ({
        transform: (t) => {
          calls.push(t)
          return {
            output: async (o) => {
              calls.push(o)
              if (fail) throw new Error('unsupported image')
              await new Response(stream).arrayBuffer()
              return { response: () => new Response(bytesOf('RIFF\x00\x00\x00\x00WEBP', 64)) }
            },
          }
        },
      }),
    }
  }
  test('makes a 320 px WebP through the binding', async () => {
    const images = stub()
    const out = await makeThumbnail(images, JPEG)
    expect(out?.byteLength).toBe(64)
    expect(images.calls).toEqual([{ width: 320 }, { format: 'image/webp' }])
  })
  test('null without a binding or when the transform throws', async () => {
    expect(await makeThumbnail(undefined, JPEG)).toBeNull()
    expect(await makeThumbnail(stub(true), JPEG)).toBeNull()
  })
})
