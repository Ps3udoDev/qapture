CREATE TABLE template (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  test_type TEXT NOT NULL DEFAULT '',
  builtin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE template_field (
  id TEXT PRIMARY KEY,
  template_id TEXT NOT NULL REFERENCES template(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  label TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('text','longtext','list','date','number','boolean')),
  options TEXT NOT NULL DEFAULT '[]',
  required INTEGER NOT NULL DEFAULT 0,
  clear_on_duplicate INTEGER NOT NULL DEFAULT 0,
  width INTEGER NOT NULL DEFAULT 24,
  position INTEGER NOT NULL,
  UNIQUE (template_id, key)
);

CREATE TABLE project (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  client TEXT NOT NULL DEFAULT '',
  test_type TEXT NOT NULL DEFAULT '',
  release_version TEXT NOT NULL DEFAULT '',
  start_date TEXT NOT NULL DEFAULT '',
  end_date TEXT NOT NULL DEFAULT '',
  next_sequence INTEGER NOT NULL DEFAULT 1,
  source_template_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_field (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  label TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('text','longtext','list','date','number','boolean')),
  options TEXT NOT NULL DEFAULT '[]',
  required INTEGER NOT NULL DEFAULT 0,
  clear_on_duplicate INTEGER NOT NULL DEFAULT 0,
  width INTEGER NOT NULL DEFAULT 24,
  position INTEGER NOT NULL,
  archived INTEGER NOT NULL DEFAULT 0,
  UNIQUE (project_id, key)
);

CREATE TABLE test_case (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
  code TEXT NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (project_id, code)
);

CREATE TABLE field_value (
  case_id TEXT NOT NULL REFERENCES test_case(id) ON DELETE CASCADE,
  field_id TEXT NOT NULL REFERENCES project_field(id) ON DELETE CASCADE,
  value TEXT NOT NULL,
  PRIMARY KEY (case_id, field_id)
);

CREATE INDEX idx_project_field_project ON project_field(project_id);
CREATE INDEX idx_test_case_project ON test_case(project_id);
CREATE INDEX idx_field_value_field ON field_value(field_id);
