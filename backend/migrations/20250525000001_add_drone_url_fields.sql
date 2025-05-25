-- Add new optional fields for drone URLs and source name
ALTER TABLE videos 
ADD COLUMN rtmp_url VARCHAR(255) NULL,
ADD COLUMN ws_url VARCHAR(255) NULL,
ADD COLUMN video_source_name VARCHAR(255) NULL;
