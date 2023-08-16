-- Your SQL goes here
CREATE INDEX ix_time ON vehicle_tracker_location (time DESC);
SELECT create_hypertable('vehicle_tracker_location','time');