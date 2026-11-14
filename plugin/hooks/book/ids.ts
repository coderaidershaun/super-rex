// Makes job ids, question ids, ingest streams and their secrets from the time, the session and random bytes, so the random part is an argument and the rules can be checked.

const DIGITS = '0123456789abcdefghijklmnopqrstuvwxyz'
/** 2026-01-01T00:00:00Z: the first six characters of a job id count seconds from here. */
const EPOCH_MS = 1_767_225_600_000
const SECONDS_WIDTH = 6
const RANDOM_WIDTH = 4
const RANDOM_LIMIT = 36 ** RANDOM_WIDTH

export class ClockOutOfRangeError extends Error {
  readonly nowMs: number

  constructor(nowMs: number) {
    super(`the clock reads ${String(nowMs)} ms, which no job id can hold: ids count seconds from 2026 to late 2094`)
    this.name = 'ClockOutOfRangeError'
    this.nowMs = nowMs
  }
}

function base36(value: number, width: number): string {
  let text = ''
  let rest = value
  while (rest > 0) {
    text = (DIGITS[rest % 36] ?? '0') + text
    rest = Math.floor(rest / 36)
  }
  return text.padStart(width, '0')
}

/** The job id rule of the store: 6 characters of seconds since 2026, then 4 random ones. */
export function jobIdOf(nowMs: number, random: number): string {
  const seconds = Math.floor((nowMs - EPOCH_MS) / 1000)
  if (seconds < 0 || seconds >= 36 ** SECONDS_WIDTH) {
    throw new ClockOutOfRangeError(nowMs)
  }
  return base36(seconds, SECONDS_WIDTH) + base36(random % RANDOM_LIMIT, RANDOM_WIDTH)
}

/** Characters of the session id a question id keeps: the end of a session UUID is random in every UUID version. */
const QUESTION_SESSION_WIDTH = 12

/**
 * The question id rule: `q<n>-<last 12 letters and digits of the session id>`.
 * The store keys questions by id alone, so the session part keeps two
 * sessions' first questions apart; the count keeps one session's apart.
 */
export function questionIdOf(session: string, count: number): string {
  const tail = session.toLowerCase().replace(/[^0-9a-z]/g, '').slice(-QUESTION_SESSION_WIDTH)
  return `q${String(count)}-${tail}`
}

export function hexOf(bytes: Uint8Array): string {
  return [...bytes].map(byte => byte.toString(16).padStart(2, '0')).join('')
}

export function randomBytes(count: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(count))
}

export function randomNumber(): number {
  return crypto.getRandomValues(new Uint32Array(1))[0] ?? 0
}

export type StreamKeys = { readonly stream: string; readonly secret: string }

export function newStreamKeys(): StreamKeys {
  return { stream: hexOf(randomBytes(8)), secret: hexOf(randomBytes(32)) }
}
