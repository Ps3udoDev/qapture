CREATE TABLE doc_version (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  version TEXT NOT NULL,
  date TEXT NOT NULL,
  author TEXT NOT NULL,
  reviewer TEXT NOT NULL DEFAULT '',
  approver TEXT NOT NULL DEFAULT '',
  changes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  UNIQUE (project_id, version COLLATE NOCASE)
);

CREATE INDEX idx_doc_version_project ON doc_version(project_id, date);
