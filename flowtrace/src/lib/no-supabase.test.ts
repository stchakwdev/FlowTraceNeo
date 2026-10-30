import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { describe, expect, it } from 'vitest'

// RW3 removed the Supabase client: no source file may bring it back, and the app does not
// depend on it.
const ROOT = join(__dirname, '..', '..')

function sources(dir: string): Array<string> {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) return sources(path)
    return /\.(ts|tsx|js|jsx)$/.test(name) ? [path] : []
  })
}

describe('no Supabase client', () => {
  it('no source file imports @supabase/supabase-js or the old supabase module', () => {
    const offenders = sources(join(ROOT, 'src')).filter((f) => {
      const text = readFileSync(f, 'utf8')
      return (
        /from\s+['"]@supabase\//.test(text) ||
        /from\s+['"](@\/lib|\.{1,2}(\/\.\.)*)\/supabase['"]/.test(text) ||
        /import\(\s*['"]@supabase\//.test(text)
      )
    })
    expect(offenders.map((f) => relative(ROOT, f))).toEqual([])
  })

  it('the app does not depend on @supabase/supabase-js', () => {
    const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')) as {
      dependencies?: Record<string, string>
      devDependencies?: Record<string, string>
    }
    const all = { ...pkg.dependencies, ...pkg.devDependencies }
    expect(Object.keys(all).filter((d) => d.startsWith('@supabase/'))).toEqual([])
  })
})
