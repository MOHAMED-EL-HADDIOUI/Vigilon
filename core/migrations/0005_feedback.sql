-- Migration 0005: user feedback on security findings.
-- Verdicts close the loop: the engine suppresses matching future
-- emissions, and the table is the seed of labeled data for evaluation.

CREATE TABLE IF NOT EXISTS feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_id TEXT,
    rule TEXT NOT NULL,
    key TEXT NOT NULL DEFAULT '',
    verdict TEXT NOT NULL,
    note TEXT
);
CREATE INDEX IF NOT EXISTS idx_feedback_rule_key ON feedback(rule, key);
