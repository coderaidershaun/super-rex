// The guard's decision on the other commands that are hard to undo: writes over a disk, a recursive change of the home folder's owner or mode, a stop of the machine, a delete of cloud resources, and a forced push to the shared branch.
// Each rule reads the program and its words as text; the cloud rules are one table, so a new command is one more row.
import { alertOf } from './alert'
import type { Alert } from './alert'
import { gitCommand } from './rm'
import { hasShort, literal, machineReason, namedBy, runsOf, splitArgs, written } from './shell'
import type { RmPlaces, Run, Word } from './shell'

/** A command the guard stops by its program and words. */
type Row = {
  /** The program's names. */
  readonly programs: readonly string[]
  /** Words that must come in this order among its words that are no option; `a|b` takes either. */
  readonly words: readonly string[]
  /** When given, one of these options must be there too, alone or with `=value`. */
  readonly options?: readonly string[]
  readonly reason: string
}

const DESTROYS_INFRA = 'deletes the infrastructure it manages'

/** The commands stopped by their words. Compared without case. */
export const ROWS: readonly Row[] = [
  { programs: ['terraform', 'tofu', 'terragrunt'], words: ['destroy'], reason: DESTROYS_INFRA },
  { programs: ['terraform', 'tofu', 'terragrunt'], words: ['apply'], options: ['-destroy', '--destroy'], reason: DESTROYS_INFRA },
  { programs: ['pulumi'], words: ['destroy'], reason: DESTROYS_INFRA },
  { programs: ['pulumi'], words: ['stack', 'rm'], reason: 'deletes a stack and its state' },
  { programs: ['cdk'], words: ['destroy'], reason: DESTROYS_INFRA },
  { programs: ['kubectl', 'oc'], words: ['delete', 'namespace|namespaces|ns'], reason: 'deletes a namespace and everything in it' },
  { programs: ['kubectl', 'oc'], words: ['delete'], options: ['--all', '--all-namespaces'], reason: 'deletes every resource of a kind' },
  { programs: ['aws'], words: ['s3', 'rb'], reason: 'deletes a bucket' },
  { programs: ['aws'], words: ['s3', 'rm'], options: ['--recursive'], reason: 'deletes every object below a prefix of a bucket' },
  { programs: ['aws'], words: ['s3api', 'delete-bucket'], reason: 'deletes a bucket' },
  { programs: ['aws'], words: ['rds', 'delete-db-instance|delete-db-cluster'], reason: 'deletes a database server' },
  { programs: ['aws'], words: ['ec2', 'terminate-instances'], reason: 'deletes servers and their disks' },
  { programs: ['aws'], words: ['dynamodb', 'delete-table'], reason: 'deletes a table and all its items' },
  { programs: ['gcloud'], words: ['projects', 'delete'], reason: 'deletes a project and everything in it' },
  { programs: ['gcloud'], words: ['sql', 'instances', 'delete'], reason: 'deletes a database server' },
  { programs: ['gcloud'], words: ['compute', 'instances', 'delete'], reason: 'deletes servers' },
  { programs: ['az'], words: ['group', 'delete'], reason: 'deletes a resource group and everything in it' },
  { programs: ['docker', 'podman'], words: ['system', 'prune'], options: ['--volumes'], reason: 'deletes every volume no container uses, with its data' },
  { programs: ['docker', 'podman'], words: ['volume', 'rm|prune'], reason: 'deletes volumes and their data' },
  { programs: ['heroku'], words: ['apps:destroy'], reason: 'deletes an app and its add-ons' },
  { programs: ['heroku'], words: ['pg:reset'], reason: 'deletes all the data of a database' },
  { programs: ['fly', 'flyctl'], words: ['apps', 'destroy'], reason: 'deletes an app' },
  { programs: ['gh'], words: ['repo', 'delete'], reason: 'deletes a GitHub repo' },
  { programs: ['supabase'], words: ['db', 'reset'], reason: 'deletes all the data of a database' },
  { programs: ['prisma'], words: ['migrate', 'reset'], reason: 'deletes all the data of a database' },
  { programs: ['prisma'], words: ['db', 'push'], options: ['--force-reset'], reason: 'deletes all the data of a database' },
  { programs: ['rails', 'rake'], words: ['db:drop|db:drop:all'], reason: 'deletes a database and all its data' },
  { programs: ['dropuser'], words: [], reason: 'deletes a database user' },
  { programs: ['diskutil'], words: ['erasedisk|erasevolume|partitiondisk|secureerase|zerodisk|randomdisk|reformat'], reason: 'erases a disk or a volume' },
  { programs: ['diskutil'], words: ['apfs|ap', 'deletecontainer|deletevolume|erasevolume'], reason: 'erases a disk or a volume' },
  { programs: ['gpt'], words: ['destroy'], reason: 'deletes the partition table of a disk' },
  { programs: ['shutdown', 'poweroff', 'halt'], words: [], reason: 'stops the machine' },
  { programs: ['reboot'], words: [], reason: 'restarts the machine' },
  { programs: ['init', 'telinit'], words: ['0|6'], reason: 'stops or restarts the machine' },
  { programs: ['systemctl'], words: ['poweroff|reboot|halt|kexec'], reason: 'stops or restarts the machine' },
]

/** The options that take a value, by program, so a value such as the `ns` of `-n ns` is not read as a word of the command. */
const VALUED: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ...['kubectl', 'oc'].map(name => [name, new Set(['-n', '--namespace', '--context', '--cluster', '--user', '--kubeconfig', '-l', '--selector', '-f', '--filename', '-o', '--output', '-c', '--container', '--field-selector', '--grace-period', '--timeout', '-s', '--server'])] as const),
  ['aws', new Set(['--profile', '--region', '--output', '--endpoint-url', '--query', '--include', '--exclude'])],
  ['gcloud', new Set(['--project', '--zone', '--region', '--account', '--format', '--configuration'])],
  ['az', new Set(['-n', '--name', '-g', '--resource-group', '--subscription', '-o', '--output', '--query'])],
  ['gh', new Set(['-R', '--repo'])],
  ['heroku', new Set(['-a', '--app', '-r', '--remote', '--confirm'])],
  ['fly', new Set(['-a', '--app', '-c', '--config'])],
  ['flyctl', new Set(['-a', '--app', '-c', '--config'])],
  ['pulumi', new Set(['-s', '--stack', '-C', '--cwd'])],
])

/** Programs that run the program after them: `npx prisma …`, `bundle exec rake …`. Each with the word it needs first, when it needs one. */
const RUNNERS: ReadonlyMap<string, ReadonlySet<string> | null> = new Map([
  ['npx', null],
  ['pnpx', null],
  ['bunx', null],
  ['uvx', null],
  ['yarn', null],
  ['pnpm', null],
  ['bun', new Set(['x', 'run'])],
  ['npm', new Set(['exec', 'x'])],
  ['bundle', new Set(['exec'])],
  ['poetry', new Set(['run'])],
  ['uv', new Set(['run'])],
  ['pipx', new Set(['run'])],
])
/** The words by which yarn and pnpm run a program, which they also run with no such word. */
const RUN_WORDS = new Set(['exec', 'run', 'dlx'])

/** The program a runner starts and its words, or the run's own when it is no runner. */
function peeled(run: Run): { readonly program: string; readonly args: readonly Word[] } {
  let program = run.program
  let args = run.args
  for (let depth = 0; depth < 3 && RUNNERS.has(program); depth += 1) {
    const needs = RUNNERS.get(program) ?? null
    const next = (from: number): number => args.findIndex((word, at) => at >= from && !(literal(word) ?? '').startsWith('-'))
    let index = next(0)
    const first = literal(args[index]) ?? ''
    if (needs !== null && !needs.has(first)) {
      break
    }
    if (needs !== null || ((program === 'yarn' || program === 'pnpm') && RUN_WORDS.has(first))) {
      index = next(index + 1)
    }
    if (index === -1) {
      break
    }
    const word = literal(args[index]) ?? ''
    program = word.slice(word.lastIndexOf('/') + 1)
    args = args.slice(index + 1)
  }
  return { program, args }
}

function rowAlertsOf(run: Run): readonly Alert[] {
  const { program, args } = peeled(run)
  const { options, plain } = splitArgs(args, VALUED.get(program))
  const words = plain.map(word => (literal(word) ?? '').toLowerCase())
  for (const row of ROWS) {
    if (!row.programs.includes(program)) {
      continue
    }
    const matched: string[] = []
    let at = 0
    for (const wanted of row.words) {
      const choices = wanted.split('|')
      const found = words.findIndex((word, index) => index >= at && choices.includes(word))
      if (found === -1) {
        break
      }
      matched.push(literal(plain[found]) ?? '')
      at = found + 1
    }
    const hasOption = row.options === undefined || row.options.some(wanted => options.some(option => option === wanted || option.startsWith(`${wanted}=`)))
    if (matched.length === row.words.length && hasOption) {
      const option = row.options === undefined ? [] : [options.find(given => row.options?.some(wanted => given === wanted || given.startsWith(`${wanted}=`))) ?? '']
      return [alertOf(`a ${[program, ...matched, ...option].join(' ')}`, row.reason)]
    }
  }
  return []
}

/** The device files that are no disk, so a write to them loses nothing. */
const HARMLESS_DEVICE = /^\/dev\/(null|zero|full|random|urandom|tty\w*|stdout|stderr|stdin|fd\/\d+|pts\/\d+|shm\/.*)$/
/** The device files of a disk or a part of one. */
const DISK_DEVICE = /^\/dev\/(r?disk\d|sd[a-z]|hd[a-z]|vd[a-z]|xvd[a-z]|nvme\d|mmcblk\d|md\d|dm-\d|mapper\/|loop\d|sr\d)/

function isDevice(text: string | null): boolean {
  return text !== null && text.startsWith('/dev/') && !HARMLESS_DEVICE.test(text)
}

const WRITES_DISK = 'writes over a disk device, and what was on it is lost'

/** Writes over a disk: `dd of=/dev/…`, a redirect or `tee` to a disk, `shred` of a device, a new file system or partition table on a device. */
function diskAlertsOf(run: Run): readonly Alert[] {
  const alerts: Alert[] = []
  for (const redirect of run.simple.redirects) {
    const target = literal(redirect.target)
    if (redirect.op.includes('>') && target !== null && DISK_DEVICE.test(target)) {
      alerts.push(alertOf(`a redirect to ${target}`, WRITES_DISK))
    }
  }
  const texts = run.args.map(word => literal(word))
  const devices = texts.filter(text => isDevice(text))
  if (run.program === 'dd') {
    const target = texts.find(text => text?.startsWith('of=') === true)?.slice('of='.length) ?? null
    if (isDevice(target)) {
      alerts.push(alertOf(`a dd to ${target ?? ''}`, WRITES_DISK))
    }
  }
  const disks = texts.filter(text => text !== null && DISK_DEVICE.test(text))
  if ((run.program === 'tee' || run.program === 'shred') && disks.length > 0) {
    alerts.push(alertOf(`a ${run.program} of ${disks[0] ?? ''}`, WRITES_DISK))
  }
  // A file system made in an image file is no disaster; one made on a device, or on a path the guard cannot read, is.
  const isFileSystem = /^(mkfs(\..+)?|newfs(_\w+)?|wipefs|mke2fs|mkswap)$/.test(run.program)
  if (isFileSystem && (devices.length > 0 || texts.some(text => text === null))) {
    alerts.push(alertOf(`a ${run.program} of ${devices[0] ?? written(run.args[run.args.length - 1] ?? [])}`, 'erases a file system, and what was on it is lost'))
  }
  if (run.program === 'fdisk' || run.program === 'sfdisk' || run.program === 'parted' || run.program === 'sgdisk' || run.program === 'gdisk') {
    const reads = new Set(['-l', '--list', '-d', '--dump', '-s', '--show-size', '-J', '--json', '-V', '--verify', '--list-free', '-p', '--print'])
    const PARTED_WRITES = new Set(['mklabel', 'mktable', 'mkpart', 'mkpartfs', 'rm', 'resizepart', 'resize', 'move', 'rescue'])
    const { options, plain } = splitArgs(run.args)
    // parted's `-s` runs a script; its reads are `-l` and the words that change nothing.
    const isListing = run.program === 'parted' ? options.some(option => option === '-l' || option === '--list') : options.some(option => reads.has(option))
    const isPartedRead = run.program === 'parted' && !plain.some(word => PARTED_WRITES.has((literal(word) ?? '').toLowerCase()))
    if (devices.length > 0 && !isListing && !isPartedRead) {
      alerts.push(alertOf(`a ${run.program} of ${devices[0] ?? ''}`, 'changes the partitions of a disk, and what was on it may be lost'))
    }
  }
  return alerts
}

/** A recursive chmod, chown or chgrp of the root, the home folder or a folder that holds it. */
function modeAlertsOf(run: Run): readonly Alert[] {
  if (run.program !== 'chmod' && run.program !== 'chown' && run.program !== 'chgrp') {
    return []
  }
  // A mode such as `-w` looks like an option; it is the mode.
  const isMode = (text: string): boolean => run.program === 'chmod' && /^([ugoa]*[-+=][rwxXstugo]*,?)+$|^\d{3,4}$/.test(text)
  const options: string[] = []
  const plain: Word[] = []
  for (const word of run.args) {
    const text = literal(word)
    if (text !== null && text.startsWith('-') && text.length > 1 && !isMode(text)) {
      options.push(text)
    } else {
      plain.push(word)
    }
  }
  if (!options.includes('--recursive') && !hasShort(options, 'R')) {
    return []
  }
  const hasReference = options.some(option => option.startsWith('--reference'))
  const targets = hasReference ? plain : plain.slice(1)
  return targets.flatMap(target => {
    const named = namedBy(target, run.places, run.vars)
    const reason = machineReason(named, run.places)
    return reason === null || named.path === null ? [] : [alertOf(`a recursive ${run.program} of ${named.path}`, `changes every file below ${reason}`)]
  })
}

/** `kill -9 -1`, which stops every process of the user, and `crontab -r`, which deletes every cron job. */
function processAlertsOf(run: Run): readonly Alert[] {
  const texts = run.args.map(word => literal(word) ?? '')
  if (run.program === 'kill') {
    // The first word may name the signal: `-9`, `-KILL`, `-s KILL`, `-n 9`; a `-1` after it is every process.
    const start = texts[0] === '-s' || texts[0] === '-n' ? 2 : texts[0]?.startsWith('-') === true && texts[0] !== '--' ? 1 : 0
    const pids = texts.slice(start).filter(text => text !== '--')
    return pids.includes('-1') ? [alertOf(`a ${['kill', ...texts].join(' ')}`, 'stops every process of your user')] : []
  }
  if (run.program === 'crontab' && texts.some(text => /^-[a-z]*r/.test(text))) {
    return [alertOf('a crontab -r', 'deletes every cron job of your user')]
  }
  return []
}

/** The branches that every agent and the person share. */
const SHARED = new Set(['main', 'master', 'refs/heads/main', 'refs/heads/master'])

/** A forced push or a remote delete of `main` or `master`. A push that names no branch is left to the merge tripwire. */
function pushAlertsOf(run: Run): readonly Alert[] {
  const git = gitCommand(run)
  if (git?.sub !== 'push') {
    return []
  }
  const { options, plain } = splitArgs(git.args, new Set(['-o', '--push-option', '--receive-pack', '--exec', '--repo']))
  const isForced = options.some(option => option === '--force' || option.startsWith('--force-with-lease')) || hasShort(options, 'f')
  const isDelete = options.includes('--delete') || hasShort(options, 'd')
  const hasRepo = options.some(option => option.startsWith('--repo'))
  const refspecs = (hasRepo ? plain : plain.slice(1)).map(word => literal(word) ?? '')
  for (const refspec of refspecs) {
    const spec = refspec.startsWith('+') ? refspec.slice(1) : refspec
    const colon = spec.lastIndexOf(':')
    const target = colon === -1 ? spec : spec.slice(colon + 1)
    if (!SHARED.has(target)) {
      continue
    }
    const branch = target.replace(/^refs\/heads\//, '')
    if (isDelete || colon === 0) {
      return [alertOf(`a delete of the remote branch ${branch}`, 'deletes the branch that every agent and you share')]
    }
    if (isForced || refspec.startsWith('+')) {
      return [alertOf(`a forced push to ${branch}`, 'rewrites the history of the branch that every agent and you share, and commits on it may be lost')]
    }
  }
  return []
}

/** A fork bomb such as `:(){ :|:& };:`: a function that starts itself twice in the background, with any name. */
function forkBomb(command: string): boolean {
  for (const found of command.matchAll(/([^\s(){};|&]+)\s*\(\)\s*\{([^}]*)\}/g)) {
    const name = (found[1] ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    if (new RegExp(`(^|[\\s;{])${name}\\s*\\|\\s*${name}\\s*&`).test(found[2] ?? '')) {
      return true
    }
  }
  return false
}

/** Every other disaster of the command the guard stops. */
export function disasterAlerts(command: string, places: RmPlaces): readonly Alert[] {
  const bomb = forkBomb(command) ? [alertOf('a fork bomb', 'starts processes until the machine stops answering')] : []
  return [...bomb, ...runsOf(command, places).flatMap(run => [...diskAlertsOf(run), ...modeAlertsOf(run), ...processAlertsOf(run), ...pushAlertsOf(run), ...rowAlertsOf(run)])]
}

/** The words of the table, each of which a row needs, so a command with none of them costs the guard nothing more. */
const ROW_WORDS = ROWS.flatMap(row => {
  const last = (row.words[row.words.length - 1] ?? '').split('|')
  // A number such as the `0` of `init 0` is in too many commands to look for; the program is not.
  return row.options ?? (row.words.length > 0 && !last.every(word => /^\d+$/.test(word)) ? last : row.programs)
})
const OWN_WORDS = ['dd', 'tee', 'shred', 'mkfs', 'newfs', 'wipefs', 'mke2fs', 'mkswap', 'fdisk', 'sfdisk', 'parted', 'sgdisk', 'gdisk', 'chmod', 'chown', 'chgrp', 'kill', 'crontab']
const MAY_BE_DISASTER = new RegExp(`(?:^|[^\\w:-])(${[...new Set([...ROW_WORDS, ...OWN_WORDS])].map(word => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})|/dev/|\\(\\)\\s*\\{`, 'i')

/** A cheap first look: a command with none of the words these rules need costs the guard nothing more. */
export function mayBeDisaster(command: string): boolean {
  // A push is looked at only when it names a shared branch: most pushes are of a job's own branch.
  return MAY_BE_DISASTER.test(command) || /\bpush\b.*\b(main|master)\b/s.test(command)
}
