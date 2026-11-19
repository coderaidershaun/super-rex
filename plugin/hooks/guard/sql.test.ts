// Proves the SQL rule of the guard: SQL that drops or empties a table, a schema or a database through a database client is stopped, and the same words with no client, or a throwaway SQLite or DuckDB database, run.
import { expect, test } from 'claude-code/testing'

import { guardAlert, mayAlarm } from './index'
import type { RmPlaces } from './index'

const PLACES: RmPlaces = {
  cwd: '/Users/sam/Code/app',
  home: '/Users/sam',
  repo: '/Users/sam/Code/app',
  parent: '/Users/sam/Code',
  temp: ['/tmp', '/private/tmp', '/var/folders/x1/T'],
}

const IN_WORKTREE: RmPlaces = { ...PLACES, cwd: '/Users/sam/Code/app.worktrees/0gaq820001' }

function stopped(command: string, places: RmPlaces = PLACES): string | null {
  return guardAlert(command, places)?.what ?? null
}

test('a DROP, a TRUNCATE or a DELETE with no WHERE through a database client is stopped, in an argument, a here-document, a here-string or a pipe', () => {
  expect(stopped('psql -c "DROP TABLE users"')).toBe('a DROP TABLE through psql')
  expect(stopped('psql -d app -c "drop table if exists users cascade"')).toBe('a DROP TABLE through psql')
  expect(stopped("mysql -u root -e 'DROP DATABASE shop'")).toBe('a DROP DATABASE through mysql')
  expect(stopped('mariadb -e "drop schema shop"')).toBe('a DROP SCHEMA through mariadb')
  expect(stopped('cqlsh -e "DROP KEYSPACE metrics"')).toBe('a DROP KEYSPACE through cqlsh')
  expect(stopped('psql "$DATABASE_URL" -c "TRUNCATE orders"')).toBe('a TRUNCATE through psql')
  expect(stopped('psql -c "truncate table orders restart identity"')).toBe('a TRUNCATE through psql')
  expect(stopped('psql -c "DELETE FROM users"')).toBe('a DELETE FROM users with no WHERE through psql')
  expect(stopped('psql -c "DELETE FROM a WHERE id = 1; DELETE FROM b;"')).toBe('a DELETE FROM b with no WHERE through psql')
  expect(stopped('psql app <<EOF\nBEGIN;\nDROP TABLE users;\nCOMMIT;\nEOF')).toBe('a DROP TABLE through psql')
  expect(stopped("psql <<< 'drop table users'")).toBe('a DROP TABLE through psql')
  expect(stopped('echo "DROP TABLE users;" | psql app')).toBe('a DROP TABLE through psql')
  expect(stopped("printf 'TRUNCATE users;\\n' | mysql shop")).toBe('a TRUNCATE through mysql')
  expect(stopped('cat <<SQL | psql\nDROP DATABASE app;\nSQL')).toBe('a DROP DATABASE through psql')
  expect(stopped('sudo -u postgres psql -c "DROP DATABASE app"')).toBe('a DROP DATABASE through psql')
  expect(stopped('bash -c "psql -c \'DROP TABLE users\'"')).toBe('a DROP TABLE through psql')
  expect(stopped('docker exec -i db psql -U app -c "DROP TABLE users"')).toBe('a DROP TABLE through psql')
  expect(stopped('docker compose exec db mysql -e "DROP DATABASE shop"')).toBe('a DROP DATABASE through mysql')
  expect(stopped('ssh prod "psql -c \'DROP TABLE users\'"')).toBe('a DROP TABLE through psql')
  expect(stopped('clickhouse client --query "DROP TABLE events"')).toBe('a DROP TABLE through clickhouse client')
  expect(stopped('clickhouse-client -q "TRUNCATE TABLE events"')).toBe('a TRUNCATE through clickhouse-client')
  expect(stopped('cockroach sql -e "DROP DATABASE bank"')).toBe('a DROP DATABASE through cockroach sql')
  expect(stopped("bq query --use_legacy_sql=false 'DROP TABLE ds.events'")).toBe('a DROP TABLE through bq query')
  expect(stopped('snowsql -q "drop table events"')).toBe('a DROP TABLE through snowsql')
  expect(stopped('sqlcmd -Q "DROP TABLE dbo.Users"')).toBe('a DROP TABLE through sqlcmd')
})

test('the client commands that drop or empty a database are stopped', () => {
  expect(stopped('dropdb app')).toBe('a dropdb')
  expect(stopped('mysqladmin -u root drop shop')).toBe('a mysqladmin drop')
  expect(stopped('redis-cli FLUSHALL')).toBe('a FLUSHALL through redis-cli')
  expect(stopped('redis-cli -n 2 flushdb')).toBe('a FLUSHDB through redis-cli')
  expect(stopped('mongosh app --eval "db.dropDatabase()"')).toBe('a dropDatabase() through mongosh')
  expect(stopped("mongo app --eval 'db.users.drop()'")).toBe('a .drop() through mongo')
  expect(stopped('docker exec db dropdb app')).toBe('a dropdb')
})

test('the same words where no database client runs, and the reading commands of a client, run', () => {
  const passed = [
    'grep -rn "DROP TABLE" migrations/',
    'git commit -m "drop table users"',
    "cat > m.sql <<EOF\nDROP TABLE users;\nEOF",
    'echo "DROP TABLE x"',
    'echo "DROP TABLE x" > m.sql',
    'rg -i "truncate|delete from" src',
    'psql -c "SELECT count(*) FROM users"',
    'psql -c "\\d users"',
    'psql -f migrations/0001_drop.sql',
    'mysql -e "DELETE FROM sessions WHERE expires < now()"',
    'psql -c "DELETE FROM users WHERE id IN (SELECT id FROM banned)"',
    'redis-cli GET flush-flag',
    'redis-cli KEYS "*"',
    'mongosh app --eval "db.users.find()"',
    'mysqladmin status',
    'docker exec db pg_dump app > dump.sql',
    'vim drop_table.sql',
  ]

  expect(passed.filter(command => stopped(command) !== null)).toEqual([])
})

test('sqlite3 and duckdb on a throwaway database run, and on a file in the main checkout or elsewhere are stopped', () => {
  const throwaway = [
    'sqlite3 :memory: "DROP TABLE t"',
    'sqlite3 <<EOF\nCREATE TABLE t(x);\nDROP TABLE t;\nEOF',
    'duckdb -c "DROP TABLE t"',
    'sqlite3 /tmp/test.db "DROP TABLE users"',
    'sqlite3 "$TMPDIR/x.db" "DELETE FROM users"',
    'duckdb /var/folders/x1/T/a.duckdb -c "TRUNCATE t"',
    'sqlite3 /Users/sam/Code/app.worktrees/0gaq820001/dev.db "DROP TABLE users"',
  ]

  expect(throwaway.filter(command => stopped(command) !== null)).toEqual([])
  expect(stopped('sqlite3 dev.db "DROP TABLE users"', IN_WORKTREE)).toBeNull()
  expect(stopped('sqlite3 dev.db "DROP TABLE users"')).toBe('a DROP TABLE through sqlite3 on dev.db')
  expect(stopped('sqlite3 -cmd ".timeout 1000" data/app.sqlite "DELETE FROM users"')).toBe('a DELETE FROM users with no WHERE through sqlite3 on app.sqlite')
  expect(stopped('sqlite3 ~/Library/Messages/chat.db "DROP TABLE message"')).toBe('a DROP TABLE through sqlite3 on chat.db')
  expect(stopped('duckdb ../warehouse.duckdb -c "DROP SCHEMA raw"')).toBe('a DROP SCHEMA through duckdb on warehouse.duckdb')
})

test('the SQL alert says what was stopped, why, and that the person runs it', () => {
  expect(guardAlert('psql -c "DROP TABLE users"', PLACES)?.text).toBe(
    'rex guard: ALERT: a DROP TABLE through psql was stopped: it deletes a table and all its rows. This does not look right. Check the command first, and do not reach the same end with another command. IF the person wants this, THEN tell them, and they run it in their own shell.',
  )
  expect(mayAlarm('psql -c "SELECT 1"')).toBe(false)
  expect(mayAlarm('ls -la && cargo test')).toBe(false)
})
