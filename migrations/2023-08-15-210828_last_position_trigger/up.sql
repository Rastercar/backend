-- Your SQL goes here
ALTER TABLE vehicle_tracker_last_location SET (fillfactor=95);

CREATE OR REPLACE FUNCTION create_last_pos_trigger_fn() RETURNS TRIGGER LANGUAGE PLPGSQL AS
      $BODY$
      	BEGIN
      		INSERT INTO vehicle_tracker_last_location (tracker_id, point, time) VALUES (NEW.tracker_id, NEW.point, NEW.time) 
      		ON CONFLICT (tracker_id) DO UPDATE SET 
      		point=NEW.point,
      		time=new.time;
      		RETURN NEW;
      	END
      $BODY$;