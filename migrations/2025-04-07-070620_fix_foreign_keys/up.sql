-- Fix foreign keys for linked_files
CREATE TABLE linked_files_new (
  id INTEGER NOT NULL PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  bottle_id INTEGER NOT NULL REFERENCES downloaded_bottles
);

INSERT INTO
  linked_files_new (id, path, bottle_id)
SELECT
  id,
  path,
  bottle_id
FROM
  linked_files;

DROP TABLE linked_files;

ALTER TABLE linked_files_new
RENAME TO linked_files;

-- Fix foreign keys for dependencies
CREATE TABLE dependencies_new (
  id INTEGER NOT NULL PRIMARY KEY,
  -- Set to NULL to indicate that the bottle was manually installed
  dependent_id INTEGER REFERENCES downloaded_bottles,
  dependency_id INTEGER NOT NULL REFERENCES downloaded_bottles,
  UNIQUE (dependent_id, dependency_id)
);

INSERT INTO
  dependencies_new (id, dependent_id, dependency_id)
SELECT
  id,
  dependent_id,
  dependency_id
FROM
  dependencies;

DROP TABLE dependencies;

ALTER TABLE dependencies_new
RENAME TO dependencies;
