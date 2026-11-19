// Proves the guard's delete rules and its hooks: a recursive rm, a find that deletes or a git clean of ignored files in the wrong folder is stopped with an alert, and every delete below the folder that holds the repo, a job worktree or a temp folder runs.
import { expect, test } from 'claude-code/testing'

import { CONFIG_OFF, ROOT, world } from '../book/kit'
import type { ProcessCall } from '../book/kit'
import { alertText, clearable, commandsOf, GUARD_CHECKS, guardAlert, rmAlert, simpleCommands } from './index'
import type { RmPlaces } from './index'

const START = { cwd: ROOT, surface: 'terminal', isInteractive: true } as const

/** A repo in a folder of projects, as the owner keeps them. */
const PLACES: RmPlaces = {
  cwd: '/Users/sam/Code/app',
  home: '/Users/sam',
  repo: '/Users/sam/Code/app',
  parent: '/Users/sam/Code',
  temp: ['/tmp', '/private/tmp', '/var/folders/x1/T'],
}

/** The same repo with the agent in a job worktree. */
const IN_WORKTREE: RmPlaces = { ...PLACES, cwd: '/Users/sam/Code/app.worktrees/0gaq820001' }

function stopped(command: string, places: RmPlaces = PLACES): string | null {
  const alert = rmAlert(command, places)
  return alert === null ? null : `${alert.target}: ${alert.reason}`
}

test('a recursive rm of the root, the home folder or a folder that holds it is stopped, however it is spelled', () => {
  const root = ['rm -rf /', 'rm -fr /', 'rm -r -f /', 'rm -Rf /', 'rm --recursive --force /', 'rm -rf /*', 'rm -rf --no-preserve-root /', 'sudo rm -rf /', '/bin/rm -rf /', 'command rm -rf /', 'rm -rf -- /', 'cd / && rm -rf *', 'cd / && rm -rf .', 'rm -rf /Users/sam/../..']
  const home = ['rm -rf ~', 'rm -rf ~/', 'rm -rf ~/*', 'rm -rf $HOME', 'rm -rf "$HOME"', 'rm -rf "${HOME}"/*', 'rm -rf /Users/sam', 'rm -rf /Users/sam/', 'cd && rm -rf .', 'cd ~ && rm -rf *', 'rm -rf ../..']
  const above = ['rm -rf /Users', 'rm -rf /Users/*', 'rm -rf ~/..']

  expect(root.filter(command => stopped(command) !== '/: the root folder')).toEqual([])
  expect(home.filter(command => stopped(command) !== '/Users/sam: your home folder')).toEqual([])
  expect(above.filter(command => stopped(command) !== '/Users: a folder that holds your home folder')).toEqual([])
})

test('a variable the command never set counts as empty, as the shell takes an unset one, so only a path that is then the root or the home folder is stopped', () => {
  expect(stopped('rm -rf "$DIR"/')).toBe('/: the root folder')
  expect(stopped('rm -rf $STEAMROOT/*')).toBe('/: the root folder')
  expect(stopped('rm -rf "$HOME/$PROJECT"')).toBe('/Users/sam: your home folder')
  expect(stopped('rm -rf "$(git rev-parse --show-toplevel)"/*')).toBe('/: the root folder')

  const passed = ['rm -rf "$DIR"', 'rm -rf "$DIR/build"', 'rm -rf $OUT/cache/*', 'rm -rf "$(mktemp -d)"', 'rm -rf `pwd`/target']
  expect(passed.filter(command => stopped(command) !== null)).toEqual([])
})

test('a variable the command itself set is followed, and one it set from something only the shell knows names nothing the guard can place', () => {
  expect(stopped('OUT=/etc; rm -rf "$OUT"')).toBe('/etc: outside /Users/sam/Code and the temp folders')
  expect(stopped('export OUT=~/Documents && rm -rf $OUT/*')).toBe('/Users/sam/Documents: outside /Users/sam/Code and the temp folders')
  expect(stopped('OUT=/tmp/build-1; rm -rf "$OUT"/*')).toBeNull()
  expect(stopped('OUT=$(mktemp -d); rm -rf "$OUT"/*')).toBeNull()
  expect(stopped('for d in /tmp/a /tmp/b; do rm -rf "$d"/*; done')).toBeNull()
  expect(stopped('ls | while read -r dir; do rm -rf "$dir"/; done')).toBeNull()
  expect(stopped('OUT=/tmp/x rm -rf "$OUT"/')).toBe('/: the root folder')
})

test('the repo itself, its Git folder and the folder that holds it are stopped, and so is everything outside that folder', () => {
  expect(stopped('rm -rf /Users/sam/Code/app')).toBe('/Users/sam/Code/app: the main checkout of this repo')
  expect(stopped('rm -rf .')).toBe('/Users/sam/Code/app: the main checkout of this repo')
  expect(stopped('rm -rf ./*')).toBe('/Users/sam/Code/app: the main checkout of this repo')
  expect(stopped('rm -rf * .*')).toBe('/Users/sam/Code/app: the main checkout of this repo')
  expect(stopped('rm -rf .git')).toBe('/Users/sam/Code/app/.git: the Git folder of this repo')
  expect(stopped('rm -rf ..')).toBe('/Users/sam/Code: a folder that holds this repo')
  expect(stopped('rm -rf /Users/sam/Code/*')).toBe('/Users/sam/Code: a folder that holds this repo')
  expect(stopped('rm -rf ~/Documents/notes')).toBe('/Users/sam/Documents/notes: outside /Users/sam/Code and the temp folders')
  expect(stopped('rm -rf ~/.cargo/registry')).toBe('/Users/sam/.cargo/registry: outside /Users/sam/Code and the temp folders')
  expect(stopped('rm -rf /etc/hosts.d /usr/local/lib/x')).toBe('/etc/hosts.d: outside /Users/sam/Code and the temp folders')
  expect(stopped('rm -rf /tmp')).toBe('/tmp: outside /Users/sam/Code and the temp folders')
  expect(stopped('rm -rf /tmp/*')).toBe('/tmp: outside /Users/sam/Code and the temp folders')
  expect(stopped('rm -rf build ../../other')).toBe('/Users/sam/other: outside /Users/sam/Code and the temp folders')
})

test('a recursive rm below the folder that holds the repo, in a job worktree or in a temp folder runs', () => {
  const inRepo = ['rm -rf target', 'rm -rf ./target/debug', 'rm -rf node_modules dist', 'rm -rf build*', 'rm -rf *.tmp', 'rm -rf target/*', 'rm -rf .git/worktrees/gone', 'rm -rf "my folder/out"', 'rm -r -- -odd-name', 'cd plugin && rm -rf node_modules', 'rm -rf /Users/sam/Code/app/target']
  const beside = ['rm -rf ../sibling-repo', 'rm -rf /Users/sam/Code/other/target', 'rm -rf /Users/sam/Code/app.worktrees/0gaq820001', 'rm -rf ../app.worktrees/*']
  const temp = ['rm -rf /tmp/scratch', 'rm -rf /private/tmp/claude-501/x', 'rm -rf "$TMPDIR/rex-test"', 'rm -rf $TMPDIR/rex-*', 'rm -rf /var/folders/x1/T/tmp.abc']
  const notRecursive = ['rm -f /etc/passwd', 'rm ~/notes.txt', 'git rm -r --cached .', 'echo rm -rf / > notes.txt', "grep -r 'rm -rf /' docs", 'ls -R /']

  expect([...inRepo, ...beside, ...temp, ...notRecursive].filter(command => stopped(command) !== null)).toEqual([])
  expect(stopped('rm -rf ./*', IN_WORKTREE)).toBeNull()
  expect(stopped('rm -rf . ', IN_WORKTREE)).toBeNull()
  expect(stopped('rm -rf ../../app', IN_WORKTREE)).toBe('/Users/sam/Code/app: the main checkout of this repo')
})

test('the guard sees through shells, wrappers, subshells, substitutions and a cd, and reads no redirect target, comment or here-document as a path', () => {
  const caught = [
    'bash -c "rm -rf /"',
    "sh -c 'cd / && rm -rf *'",
    'bash -lc "rm -rf $HOME"',
    'eval "rm -rf ~"',
    'env X=1 rm -rf /',
    'timeout 60 rm -rf /',
    '(rm -rf /)',
    '{ rm -rf /; }',
    'echo $(rm -rf /)',
    'echo `rm -rf /`',
    'true && rm -rf / || true',
    'ls; rm -rf /',
    'if true; then rm -rf /; fi',
    'rm -rf / 2>/dev/null',
    'rm -rf / >/dev/null 2>&1',
    'rm -rf \\/',
    "rm -rf '/'",
    'nohup rm -rf / &',
  ]
  const passed = [
    'rm -rf target 2>/dev/null',
    'rm -rf target > /dev/null',
    'rm -rf target >/dev/null 2>&1 || true',
    'rm -rf target # then /',
    "cat > clean.sh <<'EOF'\nrm -rf /\nEOF\nchmod +x clean.sh",
    'cat <<EOF > notes.txt\nnever run rm -rf ~\nEOF',
    'echo "rm -rf /"',
    'echo rm; echo -rf /',
    'cd "$(git rev-parse --show-toplevel)" && rm -rf target',
    'find . -name target -exec rm -rf {} +',
    'ls | xargs rm -rf',
  ]

  expect(caught.filter(command => stopped(command) === null)).toEqual([])
  expect(passed.filter(command => stopped(command) !== null)).toEqual([])
})

test('a repo whose parent folder holds the home folder lets agents clear only the repo, its worktrees and the temp folders', () => {
  const atHome: RmPlaces = { cwd: '/Users/sam/app', home: '/Users/sam', repo: '/Users/sam/app', parent: null, temp: ['/tmp'] }

  expect(clearable(atHome)).toEqual(['/Users/sam/app', '/Users/sam/app.worktrees', '/tmp'])
  expect(stopped('rm -rf target', atHome)).toBeNull()
  expect(stopped('rm -rf ../app.worktrees/0gaq820001/target', atHome)).toBeNull()
  expect(stopped('rm -rf ../other', atHome)).toBe('/Users/sam/other: outside /Users/sam/app and the temp folders')
  expect(stopped('rm -rf ..', atHome)).toBe('/Users/sam: your home folder')
})

test('a command the guard cannot place is left to run: a relative path with no known folder', () => {
  const lost: RmPlaces = { ...PLACES, cwd: null }

  expect(stopped('rm -rf target', lost)).toBeNull()
  expect(stopped('rm -rf /', lost)).toBe('/: the root folder')
})

test('the words of a command are read as the shell reads them', () => {
  const texts = (line: string): readonly (readonly string[])[] => commandsOf(line).map(words => words.map(word => word.map(part => (part.kind === 'text' ? part.text : `<${part.kind}>`)).join('')))

  expect(texts('rm -rf "a b" c\\ d \'e f\'')).toEqual([['rm', '-rf', 'a b', 'c d', 'e f']])
  expect(texts('a && b | c; d')).toEqual([['a'], ['b'], ['c'], ['d']])
  expect(texts('rm -rf ~/x $A/*.o "${B}" ~user')).toEqual([['rm', '-rf', '<home>/x', '<var>/<glob>.o', '<var>', '<unknown>user']])
  expect(texts('x=$(ls a) y')).toEqual([['ls', 'a'], ['x=<unknown>', 'y']])
  expect(texts('a 2>&1 >out <in b')).toEqual([['a', 'b']])
})

test('the reader keeps each redirect, here-document body, here-string and pipeline beside the words', () => {
  const [cat, psql] = simpleCommands("cat <<'EOF' | psql app >> /dev/sda\nDROP TABLE x;\nEOF\necho done")

  expect(cat?.input).toEqual(['DROP TABLE x;'])
  expect(psql?.upstream).toEqual([cat])
  expect(psql?.redirects.map(redirect => `${redirect.op} ${redirect.target.map(part => (part.kind === 'text' ? part.text : '')).join('')}`)).toEqual(['>> /dev/sda'])
  expect(simpleCommands('psql <<< "drop table x" 2>&1 &> log')[0]?.input).toEqual(['drop table x'])
  expect(simpleCommands('a || b')[1]?.upstream).toEqual([])
})

test('a shell that reads its script from a here-document or a pipe is read too', () => {
  expect(rmAlert('bash <<EOF\nrm -rf ~\nEOF', PLACES)?.target).toBe('/Users/sam')
  expect(rmAlert('echo "rm -rf /" | sh', PLACES)?.target).toBe('/')
  expect(rmAlert('bash build.sh <<EOF\nrm -rf ~\nEOF', PLACES)).toBeNull()
})

function deleted(command: string, places: RmPlaces = PLACES): string | null {
  const alert = guardAlert(command, places)
  return alert === null ? null : `${alert.what}: ${alert.reason}`
}

test('a find that deletes is judged by its start folders: the machine\'s own folders always, the repo only with no test that picks files, and anything outside the folders agents may clear', () => {
  expect(deleted('find / -delete')).toBe('a find -delete in /: deletes files below the root folder')
  expect(deleted('find ~ -name "*.log" -delete')).toBe('a find -delete in /Users/sam: deletes files below your home folder')
  expect(deleted('find . -type f -delete')).toBe('a find -delete in /Users/sam/Code/app: deletes files below the main checkout of this repo')
  expect(deleted('find ~/Documents -exec rm -rf {} +')).toBe('a find -exec rm in /Users/sam/Documents: deletes files below a folder outside /Users/sam/Code and the temp folders')
  expect(deleted('find -L "$HOME" -mindepth 1 -delete')).toBe('a find -delete in /Users/sam: deletes files below your home folder')
  expect(deleted('cd / && find . -delete')).toBe('a find -delete in /: deletes files below the root folder')

  const passed = ['find . -name "*.pyc" -delete', 'find . -name node_modules -prune -exec rm -rf {} +', 'find target -type f -delete', 'find /tmp -name "rex-*" -delete', 'find "$TMPDIR" -mtime +1 -delete', 'find . -type d -empty -delete', 'find ~ -name "*.log"', 'find / -name x -print', 'find .. -maxdepth 2 -name dist -exec rm -rf {} +']
  expect(passed.filter(command => deleted(command) !== null)).toEqual([])
  expect(deleted('find . -delete', IN_WORKTREE)).toBeNull()
})

test('a git clean of ignored files with force in the main checkout is stopped, and one in a job worktree, a dry run or one that keeps ignored files runs', () => {
  expect(deleted('git clean -fdx')).toBe('a git clean -fdx in the main checkout /Users/sam/Code/app: deletes every ignored file there, such as `.env` files and the store in `.rex/`, and no commit holds them')
  expect(deleted('git clean -f -X')).not.toBeNull()
  expect(deleted('git clean --force -x -d')).not.toBeNull()
  expect(deleted('cd src && git clean -xfd')).not.toBeNull()
  expect(deleted('git -C /Users/sam/Code/app clean -ffdx', IN_WORKTREE)).not.toBeNull()

  const passed = ['git clean -fd', 'git clean -ndx', 'git clean -fdx --dry-run', 'git clean -x', 'git -C ../app.worktrees/0gaq820001 clean -fdx']
  expect(passed.filter(command => deleted(command) !== null)).toEqual([])
  expect(deleted('git clean -fdx', IN_WORKTREE)).toBeNull()
})

test('the alert tells the agent what was stopped, why, where a recursive rm may delete, and that the person runs it', () => {
  const alert = rmAlert('rm -rf ~/Documents', PLACES)

  expect(alert).toEqual({ target: '/Users/sam/Documents', reason: 'outside /Users/sam/Code and the temp folders' })
  expect(alert === null ? '' : alertText(alert, PLACES)).toBe(
    'rex guard: ALERT: a recursive rm of /Users/sam/Documents was stopped: it is outside /Users/sam/Code and the temp folders. This does not look right. Check the path first. A recursive rm may delete only below /Users/sam/Code, /Users/sam/Code/app.worktrees, /tmp, /private/tmp, /var/folders/x1/T. IF the person wants this delete, THEN tell them, and they run it in their own shell.',
  )
})

/** The engine's answers a Bash call needs: the tool itself, the home folder, and git's word on the main checkout. */
function shell(on: Parameters<typeof world>[0], calls: ProcessCall[]): { readonly ran: string[]; readonly statuses: readonly string[] } {
  const ran: string[] = []
  on('tool.call', (_$, e) => {
    ran.push(String(Reflect.get(e, 'command')))
    return { result: 'ran' }
  })
  on('env.get', (_$, e) => ({ value: e.name === 'HOME' ? '/home/person' : undefined }))
  on('session.cwd', () => ({ value: ROOT }))
  const session = world(on, {
    config: CONFIG_OFF,
    processAnswer: call => {
      calls.push(call)
      return { exitCode: 0, stdout: `${ROOT}/.git\n`, stderr: '' }
    },
  })
  return { ran, statuses: session.statuses }
}

test('a Bash or Monitor call with a recursive rm outside the allowed folders is refused with the alert and a line for the person, and no other call asks git anything', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = shell(on, calls)
  await $.session.start(START)

  const listed = await $.tool.call({ tool: 'Bash', command: 'ls -la' })
  const cleaned = await $.tool.call({ tool: 'Bash', command: 'rm -rf target' })
  const wiped = await $.tool.call({ tool: 'Bash', command: 'rm -rf ~/Documents' })
  const watched = await $.tool.call({ tool: 'Monitor', description: 'wipe', timeout_ms: 1000, command: 'sh -c "rm -rf /"' })

  expect(listed.deny).toBeUndefined()
  expect(cleaned.deny).toBeUndefined()
  expect(wiped.deny).toBe(
    'rex guard: ALERT: a recursive rm of /home/person/Documents was stopped: it is outside /home/person/repo and the temp folders. This does not look right. Check the path first. A recursive rm may delete only below /home/person/repo, /home/person/repo.worktrees, /tmp, /private/tmp, /var/tmp, /private/var/tmp, /var/folders, /private/var/folders. IF the person wants this delete, THEN tell them, and they run it in their own shell.',
  )
  expect(watched.deny).toContain('rex guard: ALERT: a recursive rm of / was stopped: it is the root folder.')
  expect(session.ran).toEqual(['ls -la', 'rm -rf target'])
  expect(session.statuses).toContain('rex: ALERT: stopped a recursive rm of / (the root folder) by the main session; run it yourself if you mean it')
  expect(calls.map(call => call.argv.join(' '))).toEqual([`git -C ${ROOT} rev-parse --path-format=absolute --git-common-dir`])
  expect(GUARD_CHECKS.map(check => `${check.name} ${check.failure}`)).toEqual(['rm-guard closed', 'sql-guard closed', 'disaster-guard closed'])
})

test('a Bash call with destructive SQL or another disaster is refused with its alert and one line for the person, and the ordinary commands beside it run', async ($, on) => {
  const calls: ProcessCall[] = []
  const session = shell(on, calls)
  await $.session.start(START)

  const selected = await $.tool.call({ tool: 'Bash', command: 'psql -c "SELECT 1"' })
  const dropped = await $.tool.call({ tool: 'Bash', command: 'psql -c "DROP TABLE users"' })
  const destroyed = await $.tool.call({ tool: 'Monitor', description: 'tear down', timeout_ms: 1000, command: 'terraform destroy -auto-approve' })

  expect(selected.deny).toBeUndefined()
  expect(dropped.deny).toBe(
    'rex guard: ALERT: a DROP TABLE through psql was stopped: it deletes a table and all its rows. This does not look right. Check the command first, and do not reach the same end with another command. IF the person wants this, THEN tell them, and they run it in their own shell.',
  )
  expect(destroyed.deny).toContain('rex guard: ALERT: a terraform destroy was stopped: it deletes the infrastructure it manages.')
  expect(session.ran).toEqual(['psql -c "SELECT 1"'])
  expect(session.statuses).toContain('rex: ALERT: stopped a DROP TABLE through psql (deletes a table and all its rows) by the main session; run it yourself if you mean it')
  expect(session.statuses).toContain('rex: ALERT: stopped a terraform destroy (deletes the infrastructure it manages) by the main session; run it yourself if you mean it')
})
