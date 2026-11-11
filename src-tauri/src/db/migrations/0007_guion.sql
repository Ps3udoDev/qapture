CREATE TABLE import_profile (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  definition TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_source (
  project_id TEXT PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
  file_name TEXT NOT NULL,
  stored_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  sheet TEXT NOT NULL,
  header_row INTEGER NOT NULL,
  profile_id TEXT REFERENCES import_profile(id) ON DELETE SET NULL,
  definition TEXT NOT NULL,
  imported_at TEXT NOT NULL
);

CREATE TABLE case_source (
  case_id TEXT PRIMARY KEY REFERENCES test_case(id) ON DELETE CASCADE,
  row INTEGER NOT NULL
);

CREATE TABLE case_fixture (
  id TEXT PRIMARY KEY,
  case_id TEXT NOT NULL REFERENCES test_case(id) ON DELETE CASCADE,
  file_name TEXT NOT NULL,
  stored_path TEXT NOT NULL,
  size INTEGER NOT NULL,
  sha256 TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL,
  added_at TEXT NOT NULL
);

CREATE INDEX idx_case_fixture_case ON case_fixture(case_id);
