// The few node built-ins that the gate A scripts use, typed by hand so the type check needs no new dependency.
// Each signature is narrowed to the way these scripts call it; it is not a general description of node.

declare const process: {
  readonly argv: readonly string[]
  readonly stdout: { write(text: string): boolean }
  readonly stderr: { write(text: string): boolean }
  exitCode: number | undefined
}

declare const performance: { now(): number }

declare const Buffer: { byteLength(text: string): number }

interface ImportMeta {
  readonly dirname: string
}

declare module 'node:fs' {
  export function existsSync(path: string): boolean
  export function mkdirSync(path: string, options: { readonly recursive: true }): void
  export function readdirSync(path: string): string[]
  export function readFileSync(path: string, encoding: 'utf8'): string
  export function writeFileSync(path: string, data: string): void
}

declare module 'node:path' {
  export function basename(path: string): string
  export function join(...parts: readonly string[]): string
  export function resolve(...parts: readonly string[]): string
}

declare module 'node:crypto' {
  export function createHash(algorithm: 'sha256'): {
    update(text: string): { digest(encoding: 'hex'): string }
  }
}

declare module 'node:child_process' {
  export type SpawnSyncOptions = {
    readonly input?: string
    readonly cwd?: string
    readonly encoding: 'utf8'
    readonly maxBuffer: number
  }
  export type SpawnSyncResult = {
    readonly status: number | null
    readonly signal: string | null
    readonly stdout: string
    readonly stderr: string
    readonly error?: Error
  }
  export function spawnSync(command: string, args: readonly string[], options: SpawnSyncOptions): SpawnSyncResult
}
