/**
 * The Worker's bindings. Vars come from `wrangler.toml` `[vars]`, secrets from
 * `wrangler secret put` (or `.dev.vars` locally).
 * https://developers.cloudflare.com/workers/configuration/environment-variables/
 */
export interface Env {
  /** The R2 bucket `flowtrace-proofs`. */
  PROOFS: R2Bucket
  /** Optional Cloudflare Images binding, used only for thumbnails. */
  IMAGES?: ImagesLike

  /** SpacetimeDB host, e.g. `https://maincloud.spacetimedb.com`. */
  SPACETIMEDB_URL: string
  /** The ledger module's database name, e.g. `flowtrace-ledger-dev`. */
  SPACETIMEDB_DATABASE: string
  /** Comma-separated origins allowed by CORS, exact match. */
  ALLOWED_ORIGINS?: string
  /** WorkOS issuer; empty turns the WorkOS path off. */
  WORKOS_ISSUER?: string
  /** WorkOS JWKS URL, e.g. `https://api.workos.com/sso/jwks/<client_id>`. */
  WORKOS_JWKS_URL?: string
  /** Expected `aud`; empty skips the audience check. */
  WORKOS_AUDIENCE?: string
  /** Comma-separated auth paths that are on: `spacetimedb`, `workos`. */
  AUTH_MODES?: string

  /** Secret: a SpacetimeDB token that can read the module's private tables over SQL. */
  MODULE_TOKEN: string
  /** Secret: HMAC key for signed file URLs. */
  URL_SIGNING_KEY: string
}

/**
 * The slice of the Images binding the Worker uses (`ImagesBinding` in
 * `@cloudflare/workers-types`; https://developers.cloudflare.com/images/transform-images/bindings/).
 * Kept structural so tests can pass a small stub.
 */
export interface ImagesLike {
  input(stream: ReadableStream<Uint8Array>): {
    transform(t: { width: number }): {
      output(o: { format: 'image/webp' }): Promise<{ response(): Response }>
    }
  }
}

/** True when `name` is listed in `AUTH_MODES` (default `spacetimedb,workos`). */
export function authModeOn(env: Env, name: 'spacetimedb' | 'workos'): boolean {
  return splitList(env.AUTH_MODES ?? 'spacetimedb,workos').includes(name)
}

/** Split a comma list, trimming and dropping blanks. */
export function splitList(s: string | undefined): string[] {
  return (s ?? '')
    .split(',')
    .map((x) => x.trim())
    .filter((x) => x.length > 0)
}
