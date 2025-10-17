CREATE INDEX IF NOT EXISTS idx_users_charges_updated_at ON users(charges_updated_at);

CREATE TABLE worlds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT UNIQUE NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT worlds_name_check CHECK (length(name) > 0 AND length(name) <= 100)
);

CREATE INDEX idx_worlds_name ON worlds(name);

CREATE UNIQUE INDEX idx_worlds_unique_default ON worlds(is_default) WHERE is_default = TRUE;

INSERT INTO worlds (name, is_default) VALUES ('default', TRUE);

ALTER TABLE pixel_history DROP CONSTRAINT pixel_history_pkey;

ALTER TABLE pixel_history ADD COLUMN world_id UUID NOT NULL;

ALTER TABLE pixel_history ADD CONSTRAINT pixel_history_world_id_fkey
    FOREIGN KEY (world_id) REFERENCES worlds(id) ON DELETE CASCADE;

ALTER TABLE pixel_history ADD PRIMARY KEY (world_id, global_x, global_y);

CREATE INDEX idx_pixel_history_world_user ON pixel_history(world_id, user_id);
CREATE INDEX idx_pixel_history_world_coords ON pixel_history(world_id, global_x, global_y);
CREATE INDEX idx_pixel_history_world_created_at ON pixel_history(world_id, created_at DESC);
