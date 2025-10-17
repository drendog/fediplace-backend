ALTER TABLE pixel_history ADD COLUMN color_id_old SMALLINT;

UPDATE pixel_history ph
SET color_id_old = (
    SELECT pc.palette_index
    FROM palette_colors pc
    WHERE pc.id = ph.color_id
);

UPDATE pixel_history
SET color_id_old = -1
WHERE color_id IS NULL;

DROP INDEX IF EXISTS idx_pixel_history_color_id;

ALTER TABLE pixel_history DROP CONSTRAINT IF EXISTS pixel_history_color_id_fkey;

ALTER TABLE pixel_history DROP COLUMN IF EXISTS color_id;

ALTER TABLE pixel_history RENAME COLUMN color_id_old TO color_id;

ALTER TABLE pixel_history ALTER COLUMN color_id SET NOT NULL;

DROP INDEX IF EXISTS idx_palette_colors_world_index;
DROP INDEX IF EXISTS idx_palette_colors_world_id;

DROP TABLE IF EXISTS palette_colors;
