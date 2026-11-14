-- Moves a version 1 store to version 2 in one transaction: the old tables are
-- renamed, the version 2 schema is made, every row is copied across, and the old tables go.
--
-- Run it only through rex migrate, with foreign_keys off and legacy_alter_table
-- on, so the renames leave the references of other tables as they are.

DROP TRIGGER sub_agent_run_keeps_its_job;
DROP TRIGGER sub_agent_run_is_never_deleted;
DROP INDEX job_one_open_per_session;
DROP INDEX job_created_at;
DROP INDEX sub_agent_run_job_id;
DROP INDEX feedback_job_id;

ALTER TABLE job RENAME TO v1_job;
ALTER TABLE job_transition RENAME TO v1_job_transition;
ALTER TABLE role RENAME TO v1_role;
ALTER TABLE delivery RENAME TO v1_delivery;
ALTER TABLE orchestrator RENAME TO v1_orchestrator;
ALTER TABLE sub_agent_run RENAME TO v1_sub_agent_run;
ALTER TABLE session_tokens RENAME TO v1_session_tokens;
ALTER TABLE feedback RENAME TO v1_feedback;
ALTER TABLE meta RENAME TO v1_meta;

-- The version 2 schema goes here; rex migrate puts it in.
-- rex:schema

INSERT INTO orchestrator (name, version, content_hash, description, indexed_at)
SELECT name, version, content_hash, description, indexed_at FROM v1_orchestrator;

-- asked_by 'human' was the person at the terminal; any other asker was a script.
-- A limit a person set lived in meta under limit.<job>, in billionths of a dollar.
INSERT INTO job (
    id, request, origin, class_version, work_type, size, consequence, languages,
    pref_duration, pref_budget, pref_autonomy, merge_agreed, merge_source,
    session_id, builder_agent_id, limit_nano, limit_source,
    builder_model, builder_effort, experience_recalled, breakdown,
    orchestrator_name, orchestrator_version, skills, status, outcome, created_at, updated_at)
SELECT
    id, request,
    CASE WHEN asked_by = 'human' THEN 'composer' ELSE 'sdk' END,
    class_version, work_type, size, consequence, languages,
    pref_duration, pref_budget, pref_autonomy, merge_agreed,
    CASE WHEN merge_agreed = 1 THEN 'dialog' END,
    session_id, 'main',
    (SELECT CAST(value AS INTEGER) FROM v1_meta WHERE key = 'limit.' || v1_job.id),
    (SELECT 'set' FROM v1_meta WHERE key = 'limit.' || v1_job.id),
    builder_model, builder_effort, experience_recalled, breakdown,
    orchestrator_name, orchestrator_version, skills, status, outcome, created_at, updated_at
FROM v1_job;

INSERT INTO job_transition (id, job_id, from_status, to_status, event, at)
SELECT id, job_id, from_status, to_status, event, at FROM v1_job_transition;

INSERT INTO role (
    job_id, role, in_duration, in_budget, in_autonomy, in_intelligence, in_reliability,
    in_consequence, in_verification, agent, demand, rules, conflict, oms_version, skills, source)
SELECT
    job_id, role, in_duration, in_budget, in_autonomy, in_intelligence, in_reliability,
    in_consequence, in_verification, agent, demand, rules, conflict, oms_version, skills, 'plan'
FROM v1_role;

INSERT INTO delivery (id, job_id, repo, branch, pr, merge_agreed, merge_commit, outcome, note, recorded_at)
SELECT id, job_id, repo, branch, pr, merge_agreed, merge_commit, outcome, note, recorded_at
FROM v1_delivery;

INSERT INTO sub_agent_run (
    id, job_id, session_id, agent_id, parent_agent_id, depth, role, tag_job, agent_type,
    model_ran, effort_ran, brief, skills_opened, background, first_seen_at, started_at,
    stopped_at, last_started_at, last_stopped_at, duration_ms, tool_uses, lines_added,
    lines_removed, final_message, transcript_path, tokens_input, tokens_output,
    tokens_cache_read, tokens_cache_creation, cost_nanodollars, cost_unpriced_requests,
    tokens_provisional, transcript_bytes, transcript_damaged_lines, tokens_read_at)
SELECT
    id, job_id, session_id, agent_id, parent_agent_id, depth, role, tag_job, agent_type,
    model_ran, effort_ran, brief, skills_opened, background, first_seen_at, started_at,
    stopped_at, last_started_at, last_stopped_at, duration_ms, tool_uses, lines_added,
    lines_removed, final_message, transcript_path, tokens_input, tokens_output,
    tokens_cache_read, tokens_cache_creation, cost_nanodollars, cost_unpriced_requests,
    tokens_provisional, transcript_bytes, transcript_damaged_lines, tokens_read_at
FROM v1_sub_agent_run;

INSERT INTO session_tokens (
    job_id, tokens_input, tokens_output, tokens_cache_read, tokens_cache_creation,
    cost_nanodollars, cost_unpriced_requests, tokens_provisional, transcript_bytes,
    transcript_damaged_lines, tokens_read_at)
SELECT
    job_id, tokens_input, tokens_output, tokens_cache_read, tokens_cache_creation,
    cost_nanodollars, cost_unpriced_requests, tokens_provisional, transcript_bytes,
    transcript_damaged_lines, tokens_read_at
FROM v1_session_tokens;

INSERT INTO feedback (
    id, job_id, given_by, said, verdict, causes, lesson, evidence, changed_skills,
    changed_orchestrators, created_at)
SELECT
    id, job_id, given_by, said, verdict, causes, lesson, evidence, changed_skills,
    changed_orchestrators, created_at
FROM v1_feedback;

INSERT OR IGNORE INTO meta (key, value)
SELECT key, value FROM v1_meta WHERE key NOT LIKE 'limit.%';

DROP TABLE v1_feedback;
DROP TABLE v1_session_tokens;
DROP TABLE v1_sub_agent_run;
DROP TABLE v1_delivery;
DROP TABLE v1_role;
DROP TABLE v1_job_transition;
DROP TABLE v1_job;
DROP TABLE v1_orchestrator;
DROP TABLE v1_meta;
