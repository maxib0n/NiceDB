CREATE TABLE plans (
    id INTEGER PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES connections(id) ON DELETE CASCADE,
    database TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    sql TEXT NOT NULL,
    at INTEGER NOT NULL,
    analyzed INTEGER NOT NULL,
    total REAL,
    shape TEXT NOT NULL,
    report TEXT NOT NULL
) STRICT;

CREATE INDEX plans_by_query ON plans (connection_id, database, fingerprint, at);
