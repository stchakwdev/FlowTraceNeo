import type { ImagesLike } from './env'

/** Thumbnail width in pixels. */
export const THUMB_WIDTH = 320

/**
 * A 320 px wide WebP of an image, through the Images binding
 * (https://developers.cloudflare.com/images/transform-images/bindings/). Returns null when the
 * binding is absent or the transform fails, so the upload still stores the original.
 */
export async function makeThumbnail(
  images: ImagesLike | undefined,
  bytes: Uint8Array,
): Promise<ArrayBuffer | null> {
  if (!images) return null
  try {
    const stream = new Response(bytes).body as ReadableStream<Uint8Array>
    const result = await images
      .input(stream)
      .transform({ width: THUMB_WIDTH })
      .output({ format: 'image/webp' })
    const res = result.response()
    if (!res.ok) return null
    const out = await res.arrayBuffer()
    return out.byteLength > 0 ? out : null
  } catch {
    return null
  }
}
