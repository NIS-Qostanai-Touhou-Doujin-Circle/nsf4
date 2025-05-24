const express = require('express');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const cors = require('cors');

const app = express();
const PORT = process.env.MEDIA_SERVER_PORT || 6210;
const isDev = process.env.NODE_ENV === 'development';
const RTSP_HOST = `media:8554`;

// Enable CORS for all routes with more specific options
app.use(cors({
  origin: '*',
  methods: ['GET', 'POST', 'OPTIONS'],
  allowedHeaders: ['Content-Type', 'Range', 'Authorization'],
  exposedHeaders: ['Content-Length', 'Content-Range'],
  credentials: true,
  maxAge: 86400 // 24 hours
}));

// Handle OPTIONS preflight requests explicitly
app.options('*', cors());

// Store active streams and their associated processes
const activeStreams = new Map();
const streamCleanupTimers = new Map();

// Create temp directory for HLS segments
const TEMP_DIR = path.join(__dirname, 'temp');
if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
}

// Cleanup function for streams
function cleanupStream(streamId) {
    console.log(`Cleaning up stream: ${streamId}`);
    
    const streamData = activeStreams.get(streamId);
    if (streamData) {
        // Kill FFmpeg process
        if (streamData.ffmpegProcess && !streamData.ffmpegProcess.killed) {
            streamData.ffmpegProcess.kill('SIGTERM');
        }
        
        // Remove stream data
        activeStreams.delete(streamId);
        
        // Clean up files
        const streamDir = path.join(TEMP_DIR, streamId);
        if (fs.existsSync(streamDir)) {
            try {
                fs.rmSync(streamDir, { recursive: true, force: true });
            } catch (err) {
                console.error(`Error cleaning up stream directory ${streamId}:`, err);
            }
        }
    }
    
    // Clear cleanup timer
    if (streamCleanupTimers.has(streamId)) {
        clearTimeout(streamCleanupTimers.get(streamId));
        streamCleanupTimers.delete(streamId);
    }
}

// Schedule stream cleanup after inactivity
function scheduleCleanup(streamId) {
    // Clear existing timer
    if (streamCleanupTimers.has(streamId)) {
        clearTimeout(streamCleanupTimers.get(streamId));
    }
    
    // Schedule cleanup after 5 minutes of inactivity
    const timer = setTimeout(() => {
        cleanupStream(streamId);
    }, 5 * 60 * 1000);
    
    streamCleanupTimers.set(streamId, timer);
}

// Start FFmpeg process to convert RTSP to HLS
function startFFmpegStream(streamId, resourcePath) {
    return new Promise((resolve, reject) => {
        const streamDir = path.join(TEMP_DIR, streamId);
        
        // Create stream directory
        if (!fs.existsSync(streamDir)) {
            fs.mkdirSync(streamDir, { recursive: true });
        }
        
        const rtspUrl = `rtsp://${RTSP_HOST}${resourcePath.substring(0, resourcePath.length - 5)}`;
        console.log(`RTSP URL: ${rtspUrl}`);
        const playlistPath = path.join(streamDir, 'playlist.m3u8');
        const segmentPattern = path.join(streamDir, 'segment_%03d.ts');
        
        console.log(`Starting FFmpeg for stream ${streamId} from ${rtspUrl}`);
        
        // FFmpeg command to convert RTSP to HLS
        const ffmpegArgs = [
            '-i', rtspUrl,
            '-c:v', 'libx264',           // Video codec
            '-preset', 'fast',      // Fast encoding preset
            '-tune', 'zerolatency',      // Low latency tuning
            '-c:a', 'aac',              // Audio codec
            '-ac', '2',                 // Audio channels
            '-b:a', '128k',             // Audio bitrate
            '-f', 'hls',                // Output format
            '-hls_time', '4',           // Segment duration (4 seconds)
            '-hls_list_size', '10',     // Keep 10 segments in playlist
            '-hls_flags', 'delete_segments+append_list',
            '-hls_segment_filename', segmentPattern,
            playlistPath
        ];
        
        const ffmpegProcess = spawn('ffmpeg', ffmpegArgs);
        
        let streamStarted = false;
        let startTimeout;
        
        // Set timeout for stream start
        startTimeout = setTimeout(() => {
            if (!streamStarted) {
                ffmpegProcess.kill('SIGTERM');
                reject(new Error('Stream start timeout'));
            }
        }, 30000); // 30 second timeout
        
        ffmpegProcess.stdout.on('data', (data) => {
            console.log(`FFmpeg stdout ${streamId}:`, data.toString());
        });
        
        ffmpegProcess.stderr.on('data', (data) => {
            const output = data.toString();
            console.log(`FFmpeg stderr ${streamId}:`, output);
            
            // Check if stream has started (playlist file exists)
            if (!streamStarted && fs.existsSync(playlistPath)) {
                streamStarted = true;
                clearTimeout(startTimeout);
                resolve(ffmpegProcess);
            }
        });
        
        ffmpegProcess.on('error', (err) => {
            console.error(`FFmpeg error for stream ${streamId}:`, err);
            clearTimeout(startTimeout);
            if (!streamStarted) {
                reject(err);
            }
        });
        
        ffmpegProcess.on('close', (code) => {
            console.log(`FFmpeg process ${streamId} closed with code ${code}`);
            clearTimeout(startTimeout);
            cleanupStream(streamId);
        });
    });
}

// Generate unique stream ID based on resource path and timestamp
function generateStreamId(resourcePath) {
    const timestamp = Date.now();
    const pathHash = Buffer.from(resourcePath).toString('base64').replace(/[/+=]/g, '');
    return `${pathHash}_${timestamp}`;
}

// Route to serve HLS playlist
app.get('*.m3u8', async (req, res) => {
    const resourcePath = req.path;
    console.log(`HLS playlist request for: ${resourcePath}`);
    
    // Find or create stream for this resource
    let streamId = null;
    let streamData = null;
    
    // Look for existing stream with same resource path
    for (const [id, data] of activeStreams.entries()) {
        if (data.resourcePath === resourcePath) {
            streamId = id;
            streamData = data;
            break;
        }
    }
    
    // If no existing stream, create new one
    if (!streamId) {
        streamId = generateStreamId(resourcePath);
        console.log(`Creating new stream ${streamId} for ${resourcePath}`);
        
        try {
            const ffmpegProcess = await startFFmpegStream(streamId, resourcePath);
            streamData = {
                resourcePath,
                ffmpegProcess,
                createdAt: Date.now(),
                lastAccessed: Date.now()
            };
            activeStreams.set(streamId, streamData);
        } catch (err) {
            console.error(`Failed to start stream for ${resourcePath}:`, err);
            return res.status(500).json({ error: 'Failed to start stream' });
        }
    }
    
    // Update last accessed time
    streamData.lastAccessed = Date.now();
    
    // Schedule cleanup
    scheduleCleanup(streamId);
    
    // Serve the playlist file
    const playlistPath = path.join(TEMP_DIR, streamId, 'playlist.m3u8');
    
    // Wait for playlist to be available
    let attempts = 0;
    const maxAttempts = 50; // 10 seconds max wait
    
    while (!fs.existsSync(playlistPath) && attempts < maxAttempts) {
        await new Promise(resolve => setTimeout(resolve, 200));
        attempts++;
    }
    
    if (!fs.existsSync(playlistPath)) {
        return res.status(404).json({ error: 'Playlist not ready' });
    }
    
    // Set appropriate headers
    res.setHeader('Content-Type', 'application/vnd.apple.mpegurl');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Access-Control-Allow-Origin', '*');
    
    // Read and modify playlist to include full URLs
    try {
        let playlist = fs.readFileSync(playlistPath, 'utf8');
        
        // Replace relative segment paths with full URLs
        const baseUrl = `${req.protocol}://${req.get('Host')}`;
        const streamUrl = resourcePath.replace('.m3u8', '');
        
        playlist = playlist.replace(/segment_(\d+)\.ts/g, (match, segmentNum) => {
            return `${baseUrl}${streamUrl}/segment_${segmentNum}.ts`;
        });
        
        res.send(playlist);
    } catch (err) {
        console.error('Error reading playlist:', err);
        res.status(500).json({ error: 'Error reading playlist' });
    }
});

// Route to serve HLS segments
app.get('*/segment_*.ts', (req, res) => {
    const urlParts = req.path.split('/');
    const segmentName = urlParts[urlParts.length - 1];
    const resourcePath = '/' + urlParts.slice(1, -1).join('/') + '.m3u8';
    
    console.log(`Segment request: ${segmentName} for resource: ${resourcePath}`);
    
    // Find stream for this resource
    let streamId = null;
    for (const [id, data] of activeStreams.entries()) {
        if (data.resourcePath === resourcePath) {
            streamId = id;
            data.lastAccessed = Date.now();
            scheduleCleanup(streamId);
            break;
        }
    }
    
    if (!streamId) {
        return res.status(404).json({ error: 'Stream not found' });
    }
    
    const segmentPath = path.join(TEMP_DIR, streamId, segmentName);
    
    if (!fs.existsSync(segmentPath)) {
        return res.status(404).json({ error: 'Segment not found' });
    }
    
    // Set appropriate headers
    res.setHeader('Content-Type', 'video/mp2t');
    res.setHeader('Cache-Control', 'max-age=60');
    res.setHeader('Access-Control-Allow-Origin', '*');
    
    // Stream the segment file
    const stream = fs.createReadStream(segmentPath);
    stream.pipe(res);
    
    stream.on('error', (err) => {
        console.error('Error streaming segment:', err);
        if (!res.headersSent) {
            res.status(500).json({ error: 'Error streaming segment' });
        }
    });
});

// Health check endpoint
app.get('/health', (req, res) => {
    res.json({
        status: 'OK',
        activeStreams: activeStreams.size,
        environment: isDev ? 'development' : 'production',
        rtspHost: RTSP_HOST
    });
});

// Status endpoint to show active streams
app.get('/status', (req, res) => {
    const streams = [];
    for (const [id, data] of activeStreams.entries()) {
        streams.push({
            id,
            resourcePath: data.resourcePath,
            createdAt: new Date(data.createdAt).toISOString(),
            lastAccessed: new Date(data.lastAccessed).toISOString(),
            running: data.ffmpegProcess && !data.ffmpegProcess.killed
        });
    }
    
    res.json({
        activeStreams: streams.length,
        streams,
        environment: isDev ? 'development' : 'production',
        rtspHost: RTSP_HOST
    });
});

// Cleanup all streams on server shutdown
function gracefulShutdown() {
    console.log('Shutting down gracefully...');
    
    // Clear all cleanup timers
    for (const timer of streamCleanupTimers.values()) {
        clearTimeout(timer);
    }
    
    // Cleanup all active streams
    for (const streamId of activeStreams.keys()) {
        cleanupStream(streamId);
    }
    
    // Remove temp directory
    if (fs.existsSync(TEMP_DIR)) {
        try {
            fs.rmSync(TEMP_DIR, { recursive: true, force: true });
        } catch (err) {
            console.error('Error cleaning up temp directory:', err);
        }
    }
    
    process.exit(0);
}

// Register shutdown handlers
process.on('SIGTERM', gracefulShutdown);
process.on('SIGINT', gracefulShutdown);

// Start server
app.listen(PORT, () => {
    console.log(`HLS Proxy Server running on port ${PORT}`);
    console.log(`Environment: ${isDev ? 'development' : 'production'}`);
    console.log(`RTSP Host: ${RTSP_HOST}`);
    console.log(`Temp directory: ${TEMP_DIR}`);
});

module.exports = app;