// The few node built-ins that the WP5 live runner uses, typed by hand so the check needs no new dependency.
// Each signature is narrowed to the way the runner calls it; it is not a general description of node.

declare const process: {
  readonly env: Readonly<Record<string, string | undefined>>
  readonly argv: readonly string[]
  readonly stdout: { write(text: string): boolean }
  exitCode: number | undefined
}

declare function setInterval(callback: () => void, ms: number): unknown
declare function clearInterval(handle: unknown): void
declare function setTimeout(callback: () => void, ms: number): unknown
declare function clearTimeout(handle: unknown): void

interface ImportMeta {
  readonly dirname: string
}

declare module 'node:fs' {
  export function existsSync(path: string): boolean
  export function mkdtempSync(prefix: string): string
  export function readFileSync(path: string, encoding: 'utf8'): string
  export function realpathSync(path: string): string
  export function writeFileSync(path: string, data: string): void
}

declare module 'node:path' {
  export function join(...parts: readonly string[]): string
  export function resolve(...parts: readonly string[]): string
}

declare module 'node:os' {
  export function tmpdir(): string
}

declare module 'node:crypto' {
  export function randomUUID(): string
}

declare module 'node:child_process' {
  export type SpawnSyncOptions = {
    readonly encoding: 'utf8'
    readonly timeout: number
    readonly input?: string
    readonly env?: Readonly<Record<string, string | undefined>>
  }
  export type SpawnSyncResult = {
    readonly status: number | null
    readonly signal: string | null
    readonly stdout: string
    readonly stderr: string
    readonly error?: Error
  }
  export function spawnSync(command: string, args: readonly string[], options: SpawnSyncOptions): SpawnSyncResult

  export type SpawnOptions = {
    readonly cwd: string
    readonly env: Readonly<Record<string, string | undefined>>
    readonly stdio: readonly ['ignore', 'pipe', 'pipe']
  }
  export type TextStream = {
    setEncoding(encoding: 'utf8'): void
    on(event: 'data', listener: (chunk: string) => void): unknown
  }
  export type ChildProcess = {
    readonly pid: number | undefined
    readonly stdout: TextStream
    readonly stderr: TextStream
    kill(signal: 'SIGTERM' | 'SIGKILL'): boolean
    on(event: 'close', listener: (code: number | null, signal: string | null) => void): unknown
    on(event: 'error', listener: (error: Error) => void): unknown
  }
  export function spawn(command: string, args: readonly string[], options: SpawnOptions): ChildProcess
}
