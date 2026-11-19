// The guard's decision on databases: SQL that drops or empties a table, a schema or a database, sent through a database client, and the client commands that do the same.
// It reads only the text of the line: SQL in a file that a client reads is not seen.
import { alertOf } from './alert'
import type { Alert } from './alert'
import { baseName, inputOf, isBelow, literal, namedBy, runsOf, splitArgs, written } from './shell'
import type { RmPlaces, Run } from './shell'

/** How a client reads the text it gets: as SQL, as Redis commands, or as MongoDB scripts. */
type Dialect = 'sql' | 'redis' | 'mongo'

/** The database clients, each by the program's name. */
const CLIENTS: ReadonlyMap<string, Dialect> = new Map([
  ...['psql', 'pgcli', 'mysql', 'mariadb', 'mycli', 'sqlite3', 'sqlite', 'litecli', 'duckdb', 'sqlcmd', 'clickhouse-client', 'cqlsh', 'snowsql', 'usql', 'sqlplus', 'isql', 'trino', 'presto', 'beeline', 'spark-sql', 'mysqlsh'].map(name => [name, 'sql'] as const),
  ['redis-cli', 'redis'],
  ['valkey-cli', 'redis'],
  ['mongosh', 'mongo'],
  ['mongo', 'mongo'],
])

/** Programs that are a database client by their first word: `clickhouse client`, `cockroach sql`, `bq query`. */
const CLIENT_COMMANDS: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ['clickhouse', new Set(['client', 'local'])],
  ['cockroach', new Set(['sql'])],
  ['bq', new Set(['query'])],
  ['heroku', new Set(['pg:psql'])],
  ['turso', new Set(['shell'])],
])

/** Programs that run a client somewhere else: `docker exec db psql …`, `ssh host "mysql …"`. */
const RUNNERS = new Set(['docker', 'podman', 'kubectl', 'oc', 'ssh', 'nerdctl', 'lima', 'vagrant', 'fly', 'flyctl', 'railway'])
const CLIENT_NAMED = new RegExp(`(?:^|[\\s/;&|(])(${[...CLIENTS.keys(), 'dropdb', 'mysqladmin'].map(name => name.replace(/[-]/g, '\\-')).join('|')})(?=$|[\\s;&|)])`)

/** The options of `sqlite3` and `duckdb` that take a value, so the database file is the first other word. */
const FILE_CLIENT_VALUED = new Set(['-cmd', '-separator', '-newline', '-nullvalue', '-init', '-vfs', '-lookaside', '-pagecache', '-mmap', '-heap', '-maxsize', '-escape', '-c', '-s', '-f'])

/** What SQL deletes, by the words that do it. */
const DROPS: readonly (readonly [RegExp, string, string])[] = [
  [/\bDROP\s+DATABASE\b/i, 'DROP DATABASE', 'deletes a database and all its data'],
  [/\bDROP\s+SCHEMA\b/i, 'DROP SCHEMA', 'deletes a schema and all its tables'],
  [/\bDROP\s+KEYSPACE\b/i, 'DROP KEYSPACE', 'deletes a keyspace and all its tables'],
  [/\bDROP\s+TABLE\b/i, 'DROP TABLE', 'deletes a table and all its rows'],
  [/\bTRUNCATE\s+(?:TABLE\s+)?[A-Za-z_"`[]/i, 'TRUNCATE', 'deletes every row of a table'],
]

/** The words that name what the SQL deletes, for the alert: `DROP TABLE`, `TRUNCATE`, `DELETE FROM users with no WHERE`. */
function sqlDelete(texts: readonly string[]): { readonly what: string; readonly reason: string } | null {
  const sql = texts.join(';\n')
  for (const [pattern, what, reason] of DROPS) {
    if (pattern.test(sql)) {
      return { what, reason }
    }
  }
  for (const statement of sql.split(/;|(?=\bDELETE\b)/i)) {
    const found = /^\s*DELETE\s+FROM\s+([^\s;(]+)/i.exec(statement)
    if (found !== null && !/\bWHERE\b/i.test(statement)) {
      return { what: `DELETE FROM ${found[1] ?? ''} with no WHERE`, reason: 'deletes every row of a table' }
    }
  }
  return null
}

/** The client a run starts, and how it reads its text; null when it starts none. */
function clientOf(run: Run): { readonly name: string; readonly dialect: Dialect | 'admin' } | null {
  const dialect = CLIENTS.get(run.program)
  if (dialect !== undefined) {
    return { name: run.program, dialect }
  }
  if (run.program === 'dropdb' || run.program === 'mysqladmin') {
    return { name: run.program, dialect: 'admin' }
  }
  const first = splitArgs(run.args).plain[0]
  if (CLIENT_COMMANDS.get(run.program)?.has(literal(first) ?? '') === true) {
    return { name: `${run.program} ${literal(first) ?? ''}`, dialect: 'sql' }
  }
  if (RUNNERS.has(run.program)) {
    const found = CLIENT_NAMED.exec(` ${run.args.map(written).join(' ')}`)
    const name = found?.[1]
    if (name !== undefined) {
      return { name, dialect: name === 'dropdb' || name === 'mysqladmin' ? 'admin' : (CLIENTS.get(name) ?? 'sql') }
    }
  }
  return null
}

/** True when `sqlite3` or `duckdb` works on a throwaway database: none, `:memory:`, or a file below a temp folder or the job worktrees. */
function isThrowaway(run: Run): boolean {
  if (run.program !== 'sqlite3' && run.program !== 'sqlite' && run.program !== 'duckdb') {
    return false
  }
  const file = splitArgs(run.args, FILE_CLIENT_VALUED).plain[0]
  if (file === undefined || literal(file) === ':memory:' || literal(file) === '') {
    return true
  }
  const path = namedBy(file, run.places, run.vars).path
  return path !== null && [...run.places.temp, `${run.places.repo}.worktrees`].some(folder => isBelow(path, folder))
}

function sqlAlertsOf(run: Run): readonly Alert[] {
  const client = clientOf(run)
  if (client === null) {
    return []
  }
  const words = run.args.map(written)
  const texts = [words.join(' '), ...inputOf(run.simple)]
  const by = `through ${client.name}`
  if (client.dialect === 'admin') {
    const isDrop = client.name === 'dropdb' || words.some(word => word.toLowerCase() === 'drop')
    return isDrop ? [alertOf(client.name === 'dropdb' ? 'a dropdb' : 'a mysqladmin drop', 'deletes a database and all its data')] : []
  }
  if (client.dialect === 'redis') {
    const found = /\bflush(all|db)\b/i.exec(texts.join('\n'))
    return found === null ? [] : [alertOf(`a ${found[0].toUpperCase()} ${by}`, found[1]?.toLowerCase() === 'all' ? 'deletes every key of the Redis server' : 'deletes every key of a Redis database')]
  }
  if (client.dialect === 'mongo') {
    const found = /dropDatabase\s*\(|\.drop\s*\(/.exec(texts.join('\n'))
    return found === null ? [] : found[0].startsWith('drop') ? [alertOf(`a dropDatabase() ${by}`, 'deletes a database and all its data')] : [alertOf(`a .drop() ${by}`, 'deletes a collection and all its documents')]
  }
  const deleted = sqlDelete(texts)
  if (deleted === null || isThrowaway(run)) {
    return []
  }
  const file = run.program === 'sqlite3' || run.program === 'sqlite' || run.program === 'duckdb' ? splitArgs(run.args, FILE_CLIENT_VALUED).plain[0] : undefined
  const on = file === undefined ? '' : ` on ${baseName(written(file))}`
  return [alertOf(`a ${deleted.what} ${by}${on}`, deleted.reason)]
}

/** Every database delete of the command the guard stops. */
export function sqlAlerts(command: string, places: RmPlaces): readonly Alert[] {
  return runsOf(command, places).flatMap(sqlAlertsOf)
}

/** A cheap first look: a command with none of the words that delete data costs the guard nothing more. */
export function mayDropData(command: string): boolean {
  return /drop|truncate|delete\s+from|flush/i.test(command)
}
