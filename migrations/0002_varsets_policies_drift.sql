-- Variable sets, policy sets and drift detection.

------------------------------------------------------------------
-- Variable sets
------------------------------------------------------------------
CREATE TABLE variable_sets (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    -- A global set applies to every workspace.
    global      INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE variable_set_workspaces (
    set_id       TEXT NOT NULL REFERENCES variable_sets(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    PRIMARY KEY (set_id, workspace_id)
);

-- A variable now belongs to a workspace OR to a variable set.
CREATE TABLE variables_new (
    id           TEXT PRIMARY KEY,
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    set_id       TEXT REFERENCES variable_sets(id) ON DELETE CASCADE,
    key          TEXT NOT NULL,
    value_enc    TEXT NOT NULL,
    category     TEXT NOT NULL CHECK (category IN ('terraform', 'env')),
    sensitive    INTEGER NOT NULL DEFAULT 0,
    hcl          INTEGER NOT NULL DEFAULT 0,
    description  TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    CHECK ((workspace_id IS NULL) != (set_id IS NULL)),
    UNIQUE (workspace_id, category, key),
    UNIQUE (set_id, category, key)
);
INSERT INTO variables_new (id, workspace_id, set_id, key, value_enc, category, sensitive, hcl, description, created_at, updated_at)
    SELECT id, workspace_id, NULL, key, value_enc, category, sensitive, hcl, description, created_at, updated_at FROM variables;
DROP TABLE variables;
ALTER TABLE variables_new RENAME TO variables;
CREATE INDEX variables_set ON variables(set_id);

------------------------------------------------------------------
-- Policy sets (Rego, evaluated in-process)
------------------------------------------------------------------
CREATE TABLE policy_sets (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    -- advisory: report only. mandatory: a failure blocks apply.
    enforcement TEXT NOT NULL DEFAULT 'advisory' CHECK (enforcement IN ('advisory', 'mandatory')),
    global      INTEGER NOT NULL DEFAULT 0,
    -- inline: policies stored in Tuffie. path: *.rego files in a folder.
    source_kind TEXT NOT NULL DEFAULT 'inline' CHECK (source_kind IN ('inline', 'path')),
    source_path TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE policy_set_workspaces (
    set_id       TEXT NOT NULL REFERENCES policy_sets(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    PRIMARY KEY (set_id, workspace_id)
);

CREATE TABLE policies (
    id         TEXT PRIMARY KEY,
    set_id     TEXT NOT NULL REFERENCES policy_sets(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    rego       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (set_id, name)
);

------------------------------------------------------------------
-- Drift detection
------------------------------------------------------------------
ALTER TABLE workspaces ADD COLUMN drift_interval_minutes INTEGER NOT NULL DEFAULT 0;
-- NULL = never checked, 'ok', 'drifted', 'error'
ALTER TABLE workspaces ADD COLUMN drift_status TEXT;
ALTER TABLE workspaces ADD COLUMN drift_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspaces ADD COLUMN drift_checked_at TEXT;
ALTER TABLE workspaces ADD COLUMN drift_run_id TEXT;

------------------------------------------------------------------
-- Runs: new kind `drift`, new source `schedule`, policy results.
-- SQLite cannot change a CHECK constraint, so the table is rebuilt.
------------------------------------------------------------------
CREATE TABLE runs_new (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('plan_only', 'plan_and_apply', 'destroy', 'refresh', 'drift', 'cli')),
    source          TEXT NOT NULL DEFAULT 'ui' CHECK (source IN ('ui', 'terminal', 'api', 'schedule')),
    status          TEXT NOT NULL,
    message         TEXT NOT NULL DEFAULT '',
    command         TEXT,
    has_changes     INTEGER,
    add_count       INTEGER NOT NULL DEFAULT 0,
    change_count    INTEGER NOT NULL DEFAULT 0,
    destroy_count   INTEGER NOT NULL DEFAULT 0,
    import_count    INTEGER NOT NULL DEFAULT 0,
    drift_count     INTEGER NOT NULL DEFAULT 0,
    -- NULL = no policy sets, 'passed', 'warned', 'failed', 'errored'
    policy_status   TEXT,
    exit_code       INTEGER,
    has_state_snapshot INTEGER NOT NULL DEFAULT 0,
    state_serial    INTEGER,
    created_at      TEXT NOT NULL,
    started_at      TEXT,
    plan_finished_at TEXT,
    finished_at     TEXT,
    confirmed_comment TEXT
);
INSERT INTO runs_new (id, workspace_id, kind, source, status, message, command, has_changes, add_count, change_count,
    destroy_count, import_count, exit_code, has_state_snapshot, state_serial, created_at, started_at, plan_finished_at,
    finished_at, confirmed_comment)
    SELECT id, workspace_id, kind, source, status, message, command, has_changes, add_count, change_count,
    destroy_count, import_count, exit_code, has_state_snapshot, state_serial, created_at, started_at, plan_finished_at,
    finished_at, confirmed_comment FROM runs;
DROP TABLE runs;
ALTER TABLE runs_new RENAME TO runs;
CREATE INDEX runs_workspace_created ON runs(workspace_id, created_at DESC);
CREATE INDEX runs_status ON runs(status);
