-- Your SQL goes here

DROP VIEW v_timesheet;
DROP VIEW v_history;


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
    t.name            AS "task_name",
    t.id              AS "task_id"
FROM
    events e
    LEFT JOIN tasks t ON t.id = e.task_id
ORDER BY
    e.event_timestamp ASC
;

CREATE VIEW v_timesheet AS
SELECT
    min(vh.id) as id,
    vh.day,
    vh.task_id,
    vh.task_name,
    vh.system_event,
    sum(vh.ts_end - vh.ts_start) AS amount
FROM
    (
        SELECT
            id,
            task_id,
            task_name,
            system_event,
            ts_start,
            ts_end,
            date(ts_start, 'unixepoch', 'localtime') AS day
        FROM
            v_history
    ) vh
WHERE
    vh.ts_end IS NOT NULL
GROUP BY
    vh.day,
    vh.task_id,
    vh.task_name,
    vh.system_event
ORDER BY
    vh.day,
    vh.task_name
;
