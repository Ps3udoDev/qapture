CREATE TABLE gh_account (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  login TEXT NOT NULL,
  name TEXT,
  avatar_url TEXT,
  avatar_data TEXT,
  connected_at TEXT NOT NULL
);

CREATE TABLE project_repo_link (
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  repo TEXT NOT NULL,
  is_default INTEGER NOT NULL DEFAULT 0,
  position INTEGER NOT NULL,
  default_labels TEXT NOT NULL DEFAULT '',
  field_mapping TEXT,
  import_filter TEXT,
  PRIMARY KEY (project_id, owner, repo)
);

CREATE TABLE project_gh_settings (
  project_id TEXT PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
  routing_field_id TEXT,
  routing_map TEXT NOT NULL DEFAULT '{}',
  evidence_owner TEXT,
  evidence_repo TEXT,
  section_map TEXT
);

CREATE TABLE case_issue_link (
  case_id TEXT NOT NULL REFERENCES test_case(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  repo TEXT NOT NULL,
  issue_number INTEGER NOT NULL,
  node_id TEXT,
  html_url TEXT NOT NULL,
  direction TEXT NOT NULL CHECK (direction IN ('exported', 'imported')),
  issue_state TEXT NOT NULL DEFAULT 'open',
  etag TEXT,
  remote_updated_at TEXT,
  synced_at TEXT,
  UNIQUE (case_id, owner, repo)
);

CREATE INDEX case_issue_link_issue ON case_issue_link(owner, repo, issue_number);
