-- Alter thumbnail column to LONGTEXT for storing base64 image data
ALTER TABLE videos MODIFY thumbnail LONGTEXT NOT NULL;
