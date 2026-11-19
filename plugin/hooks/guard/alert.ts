// The alert of a command the guard stops: what was stopped, why, the text the agent reads in place of the result, and the status line the person sees.

/** One command the guard stops. `reason` reads after "it", as in "it deletes a table and all its rows". */
export type Alert = { readonly what: string; readonly reason: string; readonly text: string }

export function alertOf(what: string, reason: string): Alert {
  return {
    what,
    reason,
    text: `rex guard: ALERT: ${what} was stopped: it ${reason}. This does not look right. Check the command first, and do not reach the same end with another command. IF the person wants this, THEN tell them, and they run it in their own shell.`,
  }
}

/** The line the person sees for the alert. */
export function statusOf(alert: Alert, by: string): string {
  return `rex: ALERT: stopped ${alert.what} (${alert.reason}) by ${by}; run it yourself if you mean it`
}
