CREATE TABLE palette_colors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id UUID NOT NULL,
    palette_index SMALLINT NOT NULL,
    hex_color TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT palette_colors_world_id_fkey FOREIGN KEY (world_id) REFERENCES worlds(id) ON DELETE CASCADE,
    CONSTRAINT palette_colors_unique_world_index UNIQUE (world_id, palette_index),
    CONSTRAINT palette_colors_palette_index_range CHECK (palette_index >= 0 AND palette_index <= 255),
    CONSTRAINT palette_colors_hex_format CHECK (hex_color ~ '^#[0-9A-Fa-f]{8}$')
);

CREATE INDEX idx_palette_colors_world_id ON palette_colors(world_id);
CREATE INDEX idx_palette_colors_world_index ON palette_colors(world_id, palette_index);

INSERT INTO palette_colors (world_id, palette_index, hex_color)
SELECT
    w.id as world_id,
    palette_index::smallint,
    hex_color
FROM worlds w
CROSS JOIN (VALUES
    (0, '#6D001AFF'),
    (1, '#BE0039FF'),
    (2, '#FF4500FF'),
    (3, '#FFA800FF'),
    (4, '#FFD635FF'),
    (5, '#FFF8B8FF'),
    (6, '#00A368FF'),
    (7, '#00CC78FF'),
    (8, '#7EED56FF'),
    (9, '#00756FFF'),
    (10, '#009EAAFF'),
    (11, '#00CCC0FF'),
    (12, '#2450A4FF'),
    (13, '#3690EAFF'),
    (14, '#51E9F4FF'),
    (15, '#493AC1FF'),
    (16, '#6A5CFFFF'),
    (17, '#94B3FFFF'),
    (18, '#811E9FFF'),
    (19, '#B44AC0FF'),
    (20, '#E4ABFFFF'),
    (21, '#DE107FFF'),
    (22, '#FF3881FF'),
    (23, '#FF99AAFF'),
    (24, '#6D482FFF'),
    (25, '#9C6926FF'),
    (26, '#FFB470FF'),
    (27, '#000000FF'),
    (28, '#515252FF'),
    (29, '#898D90FF'),
    (30, '#D4D7D9FF'),
    (31, '#FFFFFFFF')
) AS colors(palette_index, hex_color)
WHERE w.is_default = TRUE;

ALTER TABLE pixel_history RENAME COLUMN color_id TO color_id_old;

ALTER TABLE pixel_history ADD COLUMN color_id UUID;

UPDATE pixel_history ph
SET color_id = (
    SELECT pc.id
    FROM palette_colors pc
    WHERE pc.world_id = ph.world_id
    AND pc.palette_index = ph.color_id_old
)
WHERE ph.color_id_old >= 0;

ALTER TABLE pixel_history ADD CONSTRAINT pixel_history_color_id_fkey
    FOREIGN KEY (color_id) REFERENCES palette_colors(id) ON DELETE SET NULL;

CREATE INDEX idx_pixel_history_color_id ON pixel_history(color_id);

ALTER TABLE pixel_history DROP COLUMN color_id_old;
