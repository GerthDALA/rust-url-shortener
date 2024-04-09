-- Add up migration script here
CREATE TABLE IF NOT EXISTS links
(
    id  TEXT NOT NULL PRIMARY KEY,
    target_url TEXT NOT NULL
);
