// The shell text every assembled brief carries, so an agent knows that no sandbox holds its commands and what the guard's alert means.

/** The fenced brief text under `## Shell` of the builder's run step, copied once so a brief does not depend on that file's layout. */
export const SHELL_TEXT =
  'Shell commands run with no sandbox, so a wrong command can delete files or data that no commit holds. The mod stops a recursive `rm` or a `find -delete` of the root folder, the home folder, the main checkout of the repo, or a folder outside the folder that holds the repo, its job worktrees and the temp folders. It also stops destructive SQL through a database client, a write over a disk, a cloud delete such as `terraform destroy`, and a forced push to `main`. Its refusal starts with `rex guard: ALERT`. If you get that alert, check the command, and do not reach the same end with another command. In your report, name each alert you got.'
