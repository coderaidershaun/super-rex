// The few node built-ins that the WP0 runner and stub server use, typed by hand so the check needs no new dependency.
// Each signature is narrowed to the way these two files call it; it is not a general description of node.

declare const process: {
  readonly env: Readonly<Record<string, string | undefined>>
  readonly argv: readonly string[]
  readonly pid: number
  readonly stdin: unknown
  readonly stdout: { write(text: string): boolean }
  exitCode: number | undefined
  cwd(): string
}

interface ImportMeta {
  readonly dirname: string
}

declare module 'node:fs' {
  export function appendFileSync(path: string, data: string): void
  export function existsSync(path: string): boolean
  export function mkdtempSync(prefix: string): string
  export function readFileSync(path: string, encoding: 'utf8'): string
  export function writeFileSync(path: string, data: string): void
}

declare module 'node:path' {
  export function join(...parts: readonly string[]): string
  export function resolve(...parts: readonly string[]): string
}

declare module 'node:readline' {
  export function createInterface(options: { readonly input: unknown }): { on(event: 'line', listener: (line: string) => void): unknown }
}

declare module 'node:os' {
  export function tmpdir(): string
}

declare module 'node:crypto' {
  export function randomUUID(): string
}

declare module 'node:child_process' {
  export type SpawnSyncOptions = {
    readonly cwd: string
    readonly env: Readonly<Record<string, string | undefined>>
    readonly encoding: 'utf8'
    readonly timeout: number
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
