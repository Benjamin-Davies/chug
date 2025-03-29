CREATE TABLE installed_bottles (
  id INTEGER NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT NOT NULL,
  path TEXT NOT NULL,
  UNIQUE (name, version)
);
