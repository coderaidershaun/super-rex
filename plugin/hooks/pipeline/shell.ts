// The shell text every assembled brief carries, so an agent knows that no sandbox holds its commands and what the guard's alert on a recursive rm means.

/** The fenced brief text under `## Shell` of the builder's run step, copied once so a brief does not depend on that file's layout. */
export const SHELL_TEXT =
  'Shell commands run with no sandbox, so a wrong path in a recursive `rm` deletes files that no commit holds. The mod stops a recursive `rm` of the root folder, the home folder, the main checkout of the repo, or a folder outside the folder that holds the repo, its job worktrees and the temp folders. Its refusal starts with `rex guard: ALERT`. If you get that alert, check the path, and do not delete the same folder with another command. In your report, name each alert you got.'
