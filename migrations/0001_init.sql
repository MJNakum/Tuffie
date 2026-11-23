CREATE TABLE workspaces (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT NOT NULL DEFAULT '',
    source_kind     TEXT NOT NULL CHECK (source_kind IN ('local', 'git')),
    local_path      TEXT,
    git_url         TEXT,
    git_ref         TEXT,
    working_dir     TEXT NOT NULL DEFAULT '',
    tool            TEXT NOT NULL DEFAULT 'terraform' CHECK (tool IN ('terraform', 'tofu')),
    tool_path       TEXT,
    auto_apply      INTEGER NOT NULL DEFAULT 0,
    lock_run_id     TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE runs (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('plan_only', 'plan_and_apply', 'destroy', 'refresh', 'cli')),
    source          TEXT NOT NULL DEFAULT 'ui' CHECK (source IN ('ui', 'terminal', 'api')),
    status          TEXT NOT NULL,
    message         TEXT NOT NULL DEFAULT '',
    command         TEXT,
    has_changes     INTEGER,
    add_count       INTEGER NOT NULL DEFAULT 0,
    change_count    INTEGER NOT NULL DEFAULT 0,
    destroy_count   INTEGER NOT NULL DEFAULT 0,
    import_count    INTEGER NOT NULL DEFAULT 0,
    exit_code       INTEGER,
    has_state_snapshot INTEGER NOT NULL DEFAULT 0,
    state_serial    INTEGER,
    created_at      TEXT NOT NULL,
    started_at      TEXT,
    plan_finished_at TEXT,
    finished_at     TEXT,
    confirmed_comment TEXT
);
CREATE INDEX runs_workspace_created ON runs(workspace_id, created_at DESC);
CREATE INDEX runs_status ON runs(status);

CREATE TABLE variables (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    key             TEXT NOT NULL,
    -- Every value is encrypted at rest (XChaCha20-Poly1305), sensitive or not.
    value_enc       TEXT NOT NULL,
    category        TEXT NOT NULL CHECK (category IN ('terraform', 'env')),
    sensitive       INTEGER NOT NULL DEFAULT 0,
    hcl             INTEGER NOT NULL DEFAULT 0,
    description     TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    UNIQUE (workspace_id, category, key)
);
