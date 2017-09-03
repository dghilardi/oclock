CREATE TABLE tasks (
  id INTEGER PRIMARY KEY NOT NULL,
  name VARCHAR NOT NULL,
  
  UNIQUE (name COLLATE NOCASE)
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY NOT NULL,
    event_timestamp INTEGER NOT NULL,

    task_id INTEGER,
    system_event_name VARCHAR,

    FOREIGN KEY(task_id) REFERENCES tasks(id) ON UPDATE CASCADE,

    CHECK (task_id IS NOT NULL OR system_event_name IS NOT NULL)
);

CREATE VIEW v_history AS
SELECT
    e.id              AS "id",
    e.event_timestamp AS "ts_start",
    (
    SELECT 
        min(event_timestamp)
    FROM events
    WHERE 
        event_timestamp >= e.event_timestamp
        AND id <> e.id
    ) AS "ts_end",
    e.system_event_name           AS "system_event",
    t.name            AS "task_name"
FROM
    events e
    LEFT JOIN tasks t ON t.id = e.task_id
ORDER BY
    e.event_timestamp ASC
;

CREATE VIEW v_timesheet AS
SELECT
    vh.day,
    vh.task_name,
    sum(vh.ts_end - vh.ts_start) AS amount
FROM
    (
        SELECT
            task_name,
            ts_start,
            ts_end,
            date(ts_start, 'unixepoch', 'localtime') AS day
        FROM
            v_history
    ) vh
GROUP BY
    vh.day,
    vh.task_name
ORDER BY
    vh.day,
    vh.task_name
;
