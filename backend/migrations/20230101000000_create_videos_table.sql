-- Create videos table
CREATE TABLE IF NOT EXISTS videos (
    id VARCHAR(36) PRIMARY KEY,
    url VARCHAR(255) NOT NULL,
    title LONGTEXT NOT NULL,
    thumbnail LONGTEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL
);

CREATE TABLE IF NOT EXISTS video_gps (
    id VARCHAR(36) PRIMARY KEY,
    video_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    latitude DOUBLE NOT NULL,
    longitude DOUBLE NOT NULL,
    altitude DOUBLE NOT NULL,
    FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS video_analytics (
    id VARCHAR(36) PRIMARY KEY,
    video_id VARCHAR(36) NOT NULL,
    created_at VARCHAR(50) NOT NULL,
    bitrate INT NOT NULL,
    resolution VARCHAR(50) NOT NULL,
    frame_rate INT NOT NULL,
    error_rate FLOAT NOT NULL,
    FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
);