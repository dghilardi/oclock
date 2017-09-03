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
        name -> Text,
    }
}

joinable!(events -> tasks (task_id));