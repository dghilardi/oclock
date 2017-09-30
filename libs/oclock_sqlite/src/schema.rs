table! {
    events (id) {
        id -> Integer,
        event_timestamp -> Integer,
        task_id -> Nullable<Integer>,
        system_event_name -> Nullable<Text>,
    }
}

table! {
    tasks (id) {
        id -> Integer,
        enabled -> Integer,
        name -> Text,
    }
}

joinable!(events -> tasks (task_id));

table! {
    v_timesheet (id) {
        id -> Integer,
        day -> Text,
        task_name -> Nullable<Text>,
        task_id -> Nullable<Integer>,
        system_event -> Nullable<Text>,
        amount -> Integer,
    }
}