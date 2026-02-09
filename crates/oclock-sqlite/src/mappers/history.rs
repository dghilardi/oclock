use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;

use crate::models::HistoryEntry;

/// Get history entries where the time block overlaps the given range.
///
/// Returns entries where `ts_start < end_ts` and (`ts_end > start_ts` or `ts_end IS NULL`),
/// excluding system events other than Startup.
pub fn get_history_by_range(
    conn: &mut SqliteConnection,
    start_ts: i32,
    end_ts: i32,
) -> Result<Vec<HistoryEntry>, Error> {
    use crate::schema::v_history::dsl::*;

    v_history
        .filter(
            ts_start
                .lt(end_ts)
                .and(ts_end.gt(start_ts).or(ts_end.is_null())),
        )
        .filter(
            system_event
                .is_null()
                .or(system_event.eq("startup")),
        )
        .order(ts_start)
        .load(conn)
}
