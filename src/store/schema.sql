-- Version 1 of the SuperRex store: the record of jobs, their roles, deliveries,
-- sub-agent runs, feedback and the orchestrator index, kept in .rex/rex.db.
--
-- Every time is UTC text in the one form YYYY-MM-DDTHH:MM:SS.sssZ, so that text
-- order is time order and the first ten characters are the date.
--
-- Only fixed lists (job status, delivery outcome, yes/no flags) are checked here.
-- Lists meant to grow, such as work type or feedback causes, are checked by rex
-- itself, so that a new value needs no change to the database.
--
-- Warning: the rules on sub-agent runs need foreign_keys and recursive_triggers
-- turned on for each connection. Open the store only through rex, which sets both.

CREATE TABLE job (
    id                   TEXT    NOT NULL PRIMARY KEY,
    request              TEXT    NOT NULL,
    asked_by             TEXT    NOT NULL,
    class_version        INTEGER NOT NULL,
    work_type            TEXT    NOT NULL,
    size                 TEXT    NOT NULL,
    consequence          TEXT    NOT NULL,
    languages            TEXT    NOT NULL,
    pref_duration        TEXT,
    pref_budget          TEXT,
    pref_autonomy        TEXT,
    merge_agreed         INTEGER NOT NULL CHECK (merge_agreed IN (0, 1)),
    session_id           TEXT    NOT NULL,
    builder_model        TEXT,
    builder_effort       TEXT,
    experience_recalled  TEXT,
    breakdown            TEXT,
    orchestrator_name    TEXT,
    orchestrator_version INTEGER,
    skills               TEXT,
    -- running is shown, never stored: it is worked out from the open sub-agent runs.
    status               TEXT    NOT NULL CHECK (status IN (
                             'created', 'planned', 'blocked', 'failed', 'delivered',
                             'accepted', 'rejected', 'analyzed', 'cancelled')),
    outcome              TEXT,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL,
    FOREIGN KEY (orchestrator_name, orchestrator_version)
        REFERENCES orchestrator (name, version)
) STRICT;

-- A session has at most one open job, so that its sub-agents go to one job.
CREATE UNIQUE INDEX job_one_open_per_session
    ON job (session_id) WHERE status IN ('created', 'planned');

CREATE INDEX job_created_at ON job (created_at);

CREATE TABLE job_transition (
    id          INTEGER NOT NULL PRIMARY KEY,
    job_id      TEXT    NOT NULL REFERENCES job (id),
    from_status TEXT    CHECK (from_status IS NULL OR from_status IN (
                    'created', 'planned', 'blocked', 'failed', 'delivered',
                    'accepted', 'rejected', 'analyzed', 'cancelled')),
    to_status   TEXT    NOT NULL CHECK (to_status IN (
                    'created', 'planned', 'blocked', 'failed', 'delivered',
                    'accepted', 'rejected', 'analyzed', 'cancelled')),
    event       TEXT    NOT NULL,
    at          TEXT    NOT NULL
) STRICT;

CREATE TABLE role (
    job_id          TEXT    NOT NULL REFERENCES job (id),
    role            TEXT    NOT NULL,
    runs_as_session INTEGER NOT NULL CHECK (runs_as_session IN (0, 1)),
    in_duration     TEXT,
    in_budget       TEXT,
    in_autonomy     TEXT,
    in_intelligence TEXT,
    in_reliability  TEXT,
    in_consequence  TEXT,
    in_verification TEXT,
    agent           TEXT,
    demand          TEXT,
    rules           TEXT,
    conflict        TEXT,
    oms_version     INTEGER,
    -- The skills the role's tasks are assigned, as a comma list.
    skills          TEXT,
    PRIMARY KEY (job_id, role),
    CHECK ((runs_as_session = 1) = (agent IS NULL)),
    CHECK (agent IS NULL OR (demand IS NOT NULL AND rules IS NOT NULL
                             AND conflict IS NOT NULL AND oms_version IS NOT NULL))
) STRICT;

CREATE TABLE delivery (
    id           INTEGER NOT NULL PRIMARY KEY,
    job_id       TEXT    NOT NULL REFERENCES job (id),
    repo         TEXT    NOT NULL,
    branch       TEXT,
    pr           TEXT,
    merge_agreed INTEGER NOT NULL CHECK (merge_agreed IN (0, 1)),
    merge_commit TEXT,
    outcome      TEXT    NOT NULL CHECK (outcome IN ('merged', 'pr-open', 'kept-local', 'blocked')),
    note         TEXT,
    recorded_at  TEXT    NOT NULL,
    UNIQUE (job_id, repo),
    CHECK (outcome <> 'merged' OR (merge_agreed = 1 AND merge_commit IS NOT NULL)),
    CHECK (outcome = 'merged' OR note IS NOT NULL),
    CHECK (outcome <> 'pr-open' OR pr IS NOT NULL),
    -- pr-open and kept-local mean no merge was agreed; blocked means one was.
    CHECK (outcome NOT IN ('pr-open', 'kept-local') OR merge_agreed = 0),
    CHECK (outcome <> 'blocked' OR merge_agreed = 1)
) STRICT;

CREATE TABLE orchestrator (
    name         TEXT    NOT NULL,
    version      INTEGER NOT NULL,
    content_hash TEXT    NOT NULL,
    description  TEXT    NOT NULL,
    indexed_at   TEXT    NOT NULL,
    PRIMARY KEY (name, version)
) STRICT;

CREATE TABLE sub_agent_run (
    id                       INTEGER NOT NULL PRIMARY KEY,
    job_id                   TEXT    NOT NULL REFERENCES job (id),
    session_id               TEXT    NOT NULL,
    agent_id                 TEXT    NOT NULL,
    parent_agent_id          TEXT,
    depth                    INTEGER,
    role                     TEXT,
    tag_job                  TEXT,
    agent_type               TEXT,
    model_ran                TEXT,
    effort_ran               TEXT,
    brief                    TEXT,
    -- Comma list. '' = the transcript was read and opened no skill; NULL = no transcript was read.
    skills_opened            TEXT,
    background               INTEGER CHECK (background IN (0, 1)),
    first_seen_at            TEXT    NOT NULL,
    started_at               TEXT,
    stopped_at               TEXT,
    duration_ms              INTEGER,
    tool_uses                INTEGER,
    lines_added              INTEGER,
    lines_removed            INTEGER,
    final_message            TEXT,
    transcript_path          TEXT,
    tokens_input             INTEGER,
    tokens_output            INTEGER,
    tokens_cache_read        INTEGER,
    tokens_cache_creation    INTEGER,
    tokens_provisional       INTEGER NOT NULL DEFAULT 1 CHECK (tokens_provisional IN (0, 1)),
    transcript_bytes         INTEGER,
    transcript_damaged_lines INTEGER,
    tokens_read_at           TEXT,
    UNIQUE (session_id, agent_id)
) STRICT;

CREATE INDEX sub_agent_run_job_id ON sub_agent_run (job_id);

-- A sub-agent keeps the job it was first tied to, even after that job closes and
-- the session opens another one, so a late event cannot move its run.
CREATE TRIGGER sub_agent_run_keeps_its_job
    BEFORE UPDATE OF job_id ON sub_agent_run
    WHEN NEW.job_id IS NOT OLD.job_id
BEGIN
    SELECT RAISE(ABORT, 'a sub-agent run keeps the job it was first tied to; job_id cannot change');
END;

-- INSERT OR REPLACE deletes the old row and inserts a new one, which would move a
-- run to another job without firing the update trigger above. Refusing every
-- delete stops that, but only when recursive_triggers is on for the connection.
CREATE TRIGGER sub_agent_run_is_never_deleted
    BEFORE DELETE ON sub_agent_run
BEGIN
    SELECT RAISE(ABORT, 'sub-agent runs are never deleted; to update a run use INSERT ... ON CONFLICT(session_id, agent_id) DO UPDATE SET ..., not INSERT OR REPLACE');
END;

-- The main session's own tokens inside each job's time, one row per job.
CREATE TABLE session_tokens (
    job_id                   TEXT    NOT NULL PRIMARY KEY REFERENCES job (id),
    tokens_input             INTEGER,
    tokens_output            INTEGER,
    tokens_cache_read        INTEGER,
    tokens_cache_creation    INTEGER,
    tokens_provisional       INTEGER NOT NULL DEFAULT 1 CHECK (tokens_provisional IN (0, 1)),
    transcript_bytes         INTEGER,
    transcript_damaged_lines INTEGER,
    tokens_read_at           TEXT
) STRICT;

CREATE TABLE feedback (
    id                    INTEGER NOT NULL PRIMARY KEY,
    job_id                TEXT    NOT NULL REFERENCES job (id),
    given_by              TEXT    NOT NULL,
    said                  TEXT    NOT NULL,
    verdict               TEXT    NOT NULL,
    causes                TEXT,
    lesson                TEXT,
    evidence              TEXT,
    changed_skills        TEXT,
    changed_orchestrators TEXT,
    created_at            TEXT    NOT NULL
) STRICT;

CREATE INDEX feedback_job_id ON feedback (job_id);

CREATE TABLE meta (
    key   TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

PRAGMA user_version = 1;
