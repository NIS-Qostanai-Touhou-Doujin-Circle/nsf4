-- Increase the size of created_at column to accommodate RFC3339 datetime format
ALTER TABLE videos MODIFY created_at VARCHAR(50) NOT NULL;
