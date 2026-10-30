CREATE TABLE agent_activity (
  id          TEXT PRIMARY KEY,
  at          TEXT NOT NULL,
  client      TEXT NOT NULL,
  tool        TEXT NOT NULL,
  project_id  TEXT,
  case_id     TEXT,
  ok          INTEGER NOT NULL,
  detail      TEXT NOT NULL
);
CREATE INDEX agent_activity_at ON agent_activity(at);
