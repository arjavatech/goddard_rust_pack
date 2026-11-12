-- Add multi-file bill images support to requests
ALTER TABLE requests ADD COLUMN IF NOT EXISTS bill_images JSONB NOT NULL DEFAULT '[]';

-- Backfill existing rows that have a single bill_image
UPDATE requests
SET bill_images = jsonb_build_array(bill_image)
WHERE bill_image IS NOT NULL AND bill_image != '' AND bill_images = '[]';



-- Drop the legacy single bill_image column (data already backfilled into bill_images)
ALTER TABLE requests DROP COLUMN IF EXISTS bill_image;