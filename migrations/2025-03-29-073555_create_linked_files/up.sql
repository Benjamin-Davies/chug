CREATE TABLE linked_files (
  id INTEGER NOT NULL PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  bottle_id INTEGER NOT NULL REFERENCES installed_bottles
);
