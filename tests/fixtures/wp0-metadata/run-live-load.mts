// Shows whether Claude Code loads and reads a skill whose header nests maps under `metadata`, beside a copy
// with no `metadata` and three broken headers, in real headless sessions. Run: node run-live-load.mts [load|read]
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const CLAUDE = process.env.CLAUDE_BIN ?? "/Users/shaun/.local/bin/claude";
const FIXTURES = import.meta.dirname;
const SKILL_MARK = "wp0";
const PLUGINS = ["metadata-plugin", "baseline-plugin", "broken-plugin"];
const PROJECT_SKILL = "metadata-plugin/skills/wp0-metadata";

// Each session is one paid `claude -p` run, so the lists stay short on purpose. `load` runs each fixture in
// its own session with no tools; `read` runs one session with every fixture and the Skill tool, so that the
// model can see the skill listing and quote what Claude Code read from each header.
type Session = { label: string; plugins: string[]; projectSkill: boolean; tools: string; prompt: string };

const LOAD_PROMPT =
  `Use no tool. List every skill in your list of available skills whose name contains "${SKILL_MARK}". ` +
  "For each, give its name and its description exactly as your list shows it. If there is none, reply NONE.";
const READ_PROMPT =
  `Do not call any tool. Your list of available skills holds some skills whose names contain "${SKILL_MARK}". ` +
  "For each of them write one line: its full name, then ' => ', then its entry exactly as your list shows it, " +
  "character for character. Write EMPTY after ' => ' when the entry has no text.";

const LOAD_SESSIONS: Session[] = [
  { label: "metadata skill in a plugin", plugins: ["metadata-plugin"], projectSkill: false, tools: "", prompt: LOAD_PROMPT },
  { label: "baseline skill with no metadata in a plugin", plugins: ["baseline-plugin"], projectSkill: false, tools: "", prompt: LOAD_PROMPT },
  { label: "broken headers in a plugin", plugins: ["broken-plugin"], projectSkill: false, tools: "", prompt: LOAD_PROMPT },
  { label: "metadata skill in a project .claude/skills", plugins: [], projectSkill: true, tools: "", prompt: LOAD_PROMPT },
];
const READ_SESSIONS: Session[] = [
  { label: "every fixture, with the Skill tool", plugins: PLUGINS, projectSkill: true, tools: "Skill", prompt: READ_PROMPT },
];

type Run = { status: number | null; stdout: string; stderr: string };

function run(command: string, args: string[], cwd: string): Run {
  const result = spawnSync(command, args, { cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (result.error) {
    throw new Error(`could not start ${command} ${args.join(" ")} in ${cwd}: ${result.error.message}`);
  }
  return { status: result.status, stdout: result.stdout, stderr: result.stderr };
}

function heading(text: string): void {
  console.log(`\n===== ${text}`);
}

function printStaticChecks(plugin: string): void {
  const folder = join(FIXTURES, plugin);
  const manifest = JSON.parse(readFileSync(join(folder, ".claude-plugin/plugin.json"), "utf8"));
  const checks: [string, string[]][] = [
    [`claude plugin validate ${plugin}`, ["plugin", "validate", folder]],
    [`claude plugin validate --strict ${plugin}`, ["plugin", "validate", "--strict", folder]],
    [`claude plugin validate ${plugin}/skills`, ["plugin", "validate", join(folder, "skills")]],
    [`claude --plugin-dir ${plugin} plugin details ${manifest.name}`, ["--plugin-dir", folder, "plugin", "details", manifest.name]],
  ];
  for (const [title, args] of checks) {
    heading(title);
    const result = run(CLAUDE, args, FIXTURES);
    console.log(`exit ${result.status}`);
    console.log((result.stdout + result.stderr).trim());
  }
}

function sessionFolder(session: Session): string {
  const folder = mkdtempSync(join(tmpdir(), "wp0-metadata-"));
  if (session.projectSkill) {
    cpSync(join(FIXTURES, PROJECT_SKILL), join(folder, ".claude/skills/wp0-metadata"), { recursive: true });
  }
  return folder;
}

function printEvent(line: string): void {
  let event;
  try {
    event = JSON.parse(line);
  } catch {
    console.log(`not JSON: ${line}`);
    return;
  }
  const mine = (names: unknown) => (Array.isArray(names) ? names.filter((name) => String(name).includes(SKILL_MARK)) : names);
  if (event.type === "system" && event.subtype === "init") {
    console.log(`init event: claude_code_version=${event.claude_code_version} model=${event.model}`);
    console.log(`init event skills with ${SKILL_MARK}: ${JSON.stringify(mine(event.skills))}`);
    console.log(`init event slash_commands with ${SKILL_MARK}: ${JSON.stringify(mine(event.slash_commands))}`);
  }
  if (event.type === "result") {
    console.log(`result (${event.subtype}, cost ${event.total_cost_usd}):\n${event.result}`);
  }
}

function printSession(session: Session): void {
  heading(`claude -p: ${session.label}`);
  const folder = sessionFolder(session);
  const debugFile = join(folder, "debug.log");
  const args = [
    "-p", session.prompt,
    "--model", "haiku",
    "--output-format", "stream-json", "--verbose",
    "--debug-file", debugFile,
    "--tools", session.tools,
    "--max-turns", "1",
    "--max-budget-usd", "0.25",
    "--no-session-persistence",
  ];
  for (const plugin of session.plugins) {
    args.push("--plugin-dir", join(FIXTURES, plugin));
  }
  console.log(`folder ${folder}`);
  const result = run(CLAUDE, args, folder);
  console.log(`exit ${result.status}`);
  if (result.stderr.trim()) {
    console.log(`stderr: ${result.stderr.trim()}`);
  }
  for (const line of result.stdout.split("\n").filter((line) => line.trim())) {
    printEvent(line);
  }

  heading(`debug lines about the test skills, YAML, or a warning: ${session.label}`);
  const debug = existsSync(debugFile) ? readFileSync(debugFile, "utf8") : "";
  const wanted = /wp0|frontmatter|yaml|\[WARN/i;
  const lines = debug.split("\n").filter((line) => wanted.test(line));
  console.log(lines.length ? lines.join("\n") : "(none)");
}

const mode = process.argv[2] ?? "all";
if (!["all", "load", "read"].includes(mode)) {
  throw new Error(`unknown mode ${mode}; give load, read, or nothing for both`);
}
for (const plugin of PLUGINS) {
  printStaticChecks(plugin);
}
const sessions = [...(mode === "read" ? [] : LOAD_SESSIONS), ...(mode === "load" ? [] : READ_SESSIONS)];
for (const session of sessions) {
  printSession(session);
}
