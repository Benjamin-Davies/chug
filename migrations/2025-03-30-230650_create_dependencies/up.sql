CREATE TABLE dependencies (
  id INTEGER NOT NULL PRIMARY KEY,
  -- Set to NULL to indicate that the bottle was manually installed
  dependent_id INTEGER REFERENCES installed_bottles,
  dependency_id INTEGER NOT NULL REFERENCES installed_bottles,
  UNIQUE (dependent_id, dependency_id)
);
