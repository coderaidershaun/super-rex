-- Version 2 of the SuperRex store: the record of jobs, their roles, deliveries,
-- sub-agent runs, feedback, the orchestrator index, and the event batches the
-- plugin sends through rex ingest, kept in .rex/rex.db.
--
-- Every time is UTC text in the one form YYYY-MM-DDTHH:MM:SS.sssZ, so that text
-- order is time order and the first ten characters are the date.
--
-- Only fixed lists (job status, origin, merge source, delivery outcome, yes/no
-- flags) are checked here.
-- Lists meant to grow, such as work type or feedback causes, are checked by rex
-- itself, so that a new value needs no change to the database.
--
-- Warning: the rules on sub-agent runs need foreign_keys and recursive_triggers
-- turned on for each connection. Open the store only through rex, which sets both.

-- A job belongs to one owner: the session and the builder that opened it, which
-- is 'main' for the main loop of the session. The plugin may leave the class
-- values unset, so they may be NULL.
CREATE TABLE job (
    id                   TEXT    NOT NULL PRIMARY KEY,
    request              TEXT    NOT NULL,
    origin               TEXT    NOT NULL CHECK (origin IN ('composer', 'bridge', 'sdk', 'master')),
    restatement          TEXT,
    class_version        INTEGER NOT NULL,
    work_type            TEXT,
    size                 TEXT,
    consequence          TEXT,
    -- Sorted names joined by commas, or 'none'.
    languages            TEXT    NOT NULL,
    pref_duration        TEXT,
    pref_budget          TEXT,
    pref_autonomy        TEXT,
    -- The merge answer: merge_agreed is 1 for yes; with merge_source NULL no one has answered.
    merge_agreed         INTEGER NOT NULL CHECK (merge_agreed IN (0, 1)),
    merge_source         TEXT    CHECK (merge_source IS NULL OR merge_source IN (
                             'dialog', 'person-prompt', 'sdk-prompt', 'question', 'policy')),
    merge_question       TEXT,
    policy_hash          TEXT,
    session_id           TEXT    NOT NULL,
    builder_agent_id     TEXT    NOT NULL DEFAULT 'main',
    repo                 TEXT,
    branch               TEXT,
    base                 TEXT,
    worktree             TEXT,
    tree_id              TEXT,
    -- Comma list of the marks of an observer or a part of the plugin that failed.
    degraded             TEXT,
    limit_nano           INTEGER CHECK (limit_nano IS NULL OR limit_nano >= 0),
    limit_source         TEXT    CHECK (limit_source IS NULL OR limit_source IN ('default', 'set', 'policy')),
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
                             'accepted', 'rejected', 'analyzed', 'cancelled', 'lost')),
    outcome              TEXT,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL,
    FOREIGN KEY (orchestrator_name, orchestrator_version)
        REFERENCES orchestrator (name, version)
) STRICT;

-- An owner has at most one open job, so that its sub-agents go to one job. A
-- builder that the master has asked for but not yet started owns its job as
-- 'pending', and a master may ask for several at once.
CREATE UNIQUE INDEX job_one_open_per_owner
    ON job (session_id, builder_agent_id)
    WHERE status IN ('created', 'planned') AND builder_agent_id <> 'pending';

CREATE INDEX job_created_at ON job (created_at);

CREATE TABLE job_transition (
    id          INTEGER NOT NULL PRIMARY KEY,
    job_id      TEXT    NOT NULL REFERENCES job (id),
    from_status TEXT    CHECK (from_status IS NULL OR from_status IN (
                    'created', 'planned', 'blocked', 'failed', 'delivered',
                    'accepted', 'rejected', 'analyzed', 'cancelled', 'lost')),
    to_status   TEXT    NOT NULL CHECK (to_status IN (
                    'created', 'planned', 'blocked', 'failed', 'delivered',
                    'accepted', 'rejected', 'analyzed', 'cancelled', 'lost')),
    event       TEXT    NOT NULL,
    at          TEXT    NOT NULL
) STRICT;

CREATE TABLE role (
    job_id          TEXT    NOT NULL REFERENCES job (id),
    role            TEXT    NOT NULL,
    in_duration     TEXT,
    in_budget       TEXT,
    in_autonomy     TEXT,
    in_intelligence TEXT,
    in_reliability  TEXT,
    in_consequence  TEXT,
    in_verification TEXT,
    agent           TEXT    NOT NULL,
    demand          TEXT    NOT NULL,
    rules           TEXT    NOT NULL,
    conflict        TEXT    NOT NULL,
    oms_version     INTEGER NOT NULL,
    -- The skills the role's tasks are assigned, as a comma list.
    skills          TEXT,
    source          TEXT    CHECK (source IS NULL OR source IN ('plan', 'rex_assign')),
    reason          TEXT,
    PRIMARY KEY (job_id, role)
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
    -- The plugin checks a delivery against the merge answer before it closes
    -- the job; the store keeps only what no record may lack.
    CHECK (outcome <> 'merged' OR merge_commit IS NOT NULL),
    CHECK (outcome <> 'pr-open' OR pr IS NOT NULL)
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
    -- A teammate of an agent team stops each time it goes idle and starts again,
    -- under the same agent ID, when a message wakes it. started_at is its first
    -- start and stopped_at its newest stop; the two last_ columns hold the newest
    -- start line and stop line, so the run is finished only when it stopped after
    -- it last started.
    started_at               TEXT,
    stopped_at               TEXT,
    last_started_at          TEXT,
    last_stopped_at          TEXT,
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
    -- The tokens' cost at list prices, in billionths of a dollar, and the count of
    -- requests on a model with no price, whose cost it leaves out.
    cost_nanodollars         INTEGER,
    cost_unpriced_requests   INTEGER,
    tokens_provisional       INTEGER NOT NULL DEFAULT 1 CHECK (tokens_provisional IN (0, 1)),
    transcript_bytes         INTEGER,
    transcript_damaged_lines INTEGER,
    tokens_read_at           TEXT,
    -- What the plugin saw at the start: how the parent was found, the grid agent
    -- the plan gave, whether the brief was assembled and passed its lint.
    parent_source            TEXT    CHECK (parent_source IS NULL OR parent_source IN ('engine', 'mod')),
    planned_agent            TEXT,
    assembled                TEXT    CHECK (assembled IS NULL OR assembled IN ('yes', 'no', 'late')),
    lint                     TEXT,
    brief_hash               TEXT,
    -- Comma lists.
    skills_attached          TEXT,
    marks                    TEXT,
    output_file              TEXT,
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
    cost_nanodollars         INTEGER,
    cost_unpriced_requests   INTEGER,
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

-- One answer for each skill of each task, and for each role rating: what the
-- assignment table, the decision agent, an override or the plan gave, and why.
CREATE TABLE decision (
    assignment_id   TEXT NOT NULL PRIMARY KEY,
    job_id          TEXT NOT NULL REFERENCES job (id),
    subject         TEXT NOT NULL,
    item            TEXT NOT NULL,
    value           TEXT NOT NULL,
    source          TEXT NOT NULL CHECK (source IN ('rule', 'decision-agent', 'override', 'plan')),
    reason          TEXT NOT NULL,
    table_version   TEXT,
    overlay_version TEXT,
    at              TEXT NOT NULL
) STRICT;

-- The skill files a plan gives its tasks, with the hash of each as served.
CREATE TABLE manifest (
    job_id TEXT NOT NULL REFERENCES job (id),
    skill  TEXT NOT NULL,
    file   TEXT NOT NULL,
    hash   TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('main', 'plugin', 'user', 'worktree')),
    reason TEXT,
    PRIMARY KEY (job_id, skill, file)
) STRICT;

-- Each skill file an agent was given or opened.
CREATE TABLE skill_open (
    session_id TEXT NOT NULL,
    agent_id   TEXT NOT NULL,
    skill      TEXT NOT NULL,
    file       TEXT NOT NULL,
    hash       TEXT NOT NULL,
    how        TEXT NOT NULL CHECK (how IN ('attached', 'rex_skill', 'read')),
    at         TEXT NOT NULL
) STRICT;

CREATE INDEX skill_open_agent ON skill_open (session_id, agent_id);

-- A stream is one load of the plugin in one process. Only the hash of its
-- secret is kept, so the store gives no one the secret itself.
CREATE TABLE ingest_stream (
    session_id    TEXT NOT NULL,
    stream        TEXT NOT NULL,
    secret_sha256 TEXT NOT NULL,
    opened_at     TEXT NOT NULL,
    PRIMARY KEY (session_id, stream)
) STRICT;

-- Every event the plugin sent, as sent. The sequence counts across the streams
-- of a session, from 1, with no gap.
CREATE TABLE ingest_event (
    session_id TEXT    NOT NULL,
    seq        INTEGER NOT NULL CHECK (seq >= 1),
    stream     TEXT    NOT NULL,
    kind       TEXT    NOT NULL,
    at         TEXT    NOT NULL,
    agent      TEXT    NOT NULL,
    job_id     TEXT,
    data       TEXT    NOT NULL,
    PRIMARY KEY (session_id, seq),
    FOREIGN KEY (session_id, stream) REFERENCES ingest_stream (session_id, stream)
) STRICT;

-- A master tree: the charter the person gave, the policy it runs under, and its cap.
CREATE TABLE tree (
    id              TEXT    NOT NULL PRIMARY KEY,
    session_id      TEXT    NOT NULL,
    master_agent_id TEXT,
    charter         TEXT    NOT NULL,
    charter_hash    TEXT    NOT NULL,
    policy_hash     TEXT    NOT NULL,
    state           TEXT    NOT NULL CHECK (state IN ('running', 'paused', 'blocked', 'done', 'lost')),
    cap_nano        INTEGER NOT NULL CHECK (cap_nano >= 0),
    report_file     TEXT,
    started_at      TEXT    NOT NULL,
    closed_at       TEXT
) STRICT;

-- A question parked for the person, and its answer once given.
CREATE TABLE question (
    id             TEXT NOT NULL PRIMARY KEY,
    tree_id        TEXT,
    job_id         TEXT,
    merge_for      TEXT,
    question       TEXT NOT NULL,
    -- A JSON list of the options.
    options        TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    category       TEXT NOT NULL CHECK (category IN ('merge', 'scope', 'spend', 'other')),
    asked_at       TEXT NOT NULL,
    answer         TEXT,
    answer_source  TEXT,
    answered_at    TEXT
) STRICT;

INSERT INTO meta (key, value) VALUES ('store_version', '2');

PRAGMA user_version = 2;
