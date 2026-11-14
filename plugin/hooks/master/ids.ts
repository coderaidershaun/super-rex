// Names a master tree: `t`, then the seconds since 2026 and three random characters, so a tree id never reads as a job id.

const DIGITS = '0123456789abcdefghijklmnopqrstuvwxyz'
const EPOCH_MS = 1_767_225_600_000
const SECONDS_WIDTH = 6
const RANDOM_WIDTH = 3

function base36(value: number, width: number): string {
  let text = ''
  let rest = Math.max(0, Math.floor(value))
  while (rest > 0) {
    text = (DIGITS[rest % 36] ?? '0') + text
    rest = Math.floor(rest / 36)
  }
  return text.padStart(width, '0').slice(-width)
}

export function treeIdOf(nowMs: number, random: number = crypto.getRandomValues(new Uint32Array(1))[0] ?? 0): string {
  const seconds = Math.floor((nowMs - EPOCH_MS) / 1000)
  return `t${base36(seconds, SECONDS_WIDTH)}${base36(random % 36 ** RANDOM_WIDTH, RANDOM_WIDTH)}`
}
