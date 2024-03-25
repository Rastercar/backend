use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect};
use shared::entity::vehicle_tracker;
use std::collections::HashMap;
use std::time::Instant;

/// A tracker ID cache, this is basically a HashMap
/// where the key is the tracker IMEI and the val its ID
///
/// the catch is that since this cache might be hit multiple
/// times with a non existing ID consecutively, it avoids accessing
/// the database if there are too many failed attempts to get
/// a ID by a certain IMEI within a time window
pub struct TrackerIdCache {
    db: DatabaseConnection,

    /// IMEI -> ID
    cache: HashMap<String, i32>,

    /// the maximun amount of times a IMEI within a time window
    /// a IMEI can fail to retrieve a ID from the DB before any further
    /// get attempts just return None without checking the database
    max_attempts: u32,

    /// the time window for failed attempts
    ///
    /// # IMPORTANT
    ///
    /// in a worse case scenario this value is the maximun amount of time
    /// a get for a certain IMEI will keep failing if even if there is a value
    /// on the database inserted just before the amount of failed attempts for
    /// said IMEI reached max_attempts
    time_window_seconds: u64,

    /// IMEI -> (attempt_count, first_failed_time)
    failed_attempts: HashMap<String, (u32, Instant)>,
}

impl TrackerIdCache {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            cache: HashMap::new(),
            failed_attempts: HashMap::new(),
            max_attempts: 10,
            time_window_seconds: 5 * 60,
        }
    }

    /// gets a tracker ID by IMEI, attempts to get the value
    /// on the cache first and if not found hits the DB
    ///
    /// ### IMPORTANT
    ///
    /// If there was too many failed attempts within the a time window
    /// None is returned without accessing the database.
    ///
    /// [PROD-TODO]
    /// in order to make this write to the cache and the DB, this needs to be mutable
    /// and since this is used in a multithreaded context and wrapped by a mutex this
    /// is locked quite often, which is not desirable
    pub async fn get(&mut self, imei: &str) -> Option<i32> {
        if let Some((attempt_count, first_error)) = self.failed_attempts.get_mut(imei) {
            let is_within_time_windown = first_error.elapsed().as_secs() < self.time_window_seconds;

            let max_attempts_reached = *attempt_count >= self.max_attempts;

            // If the current attempt is within the time window and the maximun amount
            // of attempts has been reached, avoid trying to get the value from the
            // cache or the database as it will most likely be none.
            if is_within_time_windown && max_attempts_reached {
                return None;
            }
        }

        let cached_value = self.cache.get(imei).cloned();
        if cached_value.is_some() {
            return cached_value;
        }

        if let Some(id) = self.get_from_db(imei).await.unwrap_or(None) {
            self.cache.insert(imei.to_string(), id);
            return Some(id);
        }

        self.failed_attempts
            .entry(imei.to_string())
            .and_modify(|(attempt_count, first_failure_time)| {
                let elapsed_seconds = first_failure_time.elapsed().as_secs();

                let is_within_time_windown = elapsed_seconds < self.time_window_seconds;

                if is_within_time_windown {
                    *attempt_count += 1;
                } else {
                    *attempt_count = 1;
                    *first_failure_time = Instant::now();
                }
            })
            .or_insert((1, Instant::now()));

        None
    }

    pub fn delete(&mut self, imei: &str) {
        self.cache.remove(imei);
        self.failed_attempts.remove(imei);
    }

    async fn get_from_db(&self, imei: &str) -> Result<Option<i32>, DbErr> {
        vehicle_tracker::Entity::find()
            .select_only()
            .column(vehicle_tracker::Column::Id)
            .filter(vehicle_tracker::Column::Imei.eq(imei))
            .into_tuple()
            .one(&self.db)
            .await
    }
}
