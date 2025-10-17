DROP INDEX IF EXISTS idx_pixel_history_world_user;
DROP INDEX IF EXISTS idx_pixel_history_world_coords;
DROP INDEX IF EXISTS idx_pixel_history_world_created_at;

ALTER TABLE pixel_history DROP CONSTRAINT pixel_history_pkey;

ALTER TABLE pixel_history DROP COLUMN world_id;

ALTER TABLE pixel_history ADD PRIMARY KEY (global_x, global_y);

DROP INDEX IF EXISTS idx_worlds_unique_default;

DROP TABLE worlds;
