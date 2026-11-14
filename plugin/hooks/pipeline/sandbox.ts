// The sandbox text every assembled brief carries, so an agent knows which network failure it may retry outside the sandbox.

/** The fenced brief text under `## Sandbox` of the builder's run step, copied once so a brief does not depend on that file's layout. */
export const SANDBOX_TEXT =
  'Shell commands run in a sandbox that reaches the network only through its proxy. If a command fails because its program opens its own network connection (a host name lookup or a connection fails, but `curl` reaches the same host), run that one command again with `dangerouslyDisableSandbox: true`. Do this for no other cause, and never because the sandbox refused a write. In your report, name each command that ran outside the sandbox.'
