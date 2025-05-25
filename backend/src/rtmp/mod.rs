use std::net::SocketAddr;
use std::process::{Command, Child, Stdio}; // Added Stdio
use std::sync::Mutex;
use std::collections::HashMap;
use tokio::time::Duration;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufRead}; // Added for reading stderr
use regex::Regex; // Added for parsing ffmpeg output
use crate::database; // Added for database interaction
use sqlx::MySqlPool; // Added for database pool

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtmpRelay {
    pub drone_id: String,
    pub source_url: String,
    pub destination_url: String,
    pub active: bool,
}

pub struct RelayProcess {
    pub relay: RtmpRelay,
    pub process: Option<Child>,
    pub pool: MySqlPool, // Added database pool to RelayProcess
}

// Global state for managing active relays
pub struct RelayManager {
    relays: HashMap<String, RelayProcess>,
}

impl RelayManager {
    fn new() -> Self {
        RelayManager {
            relays: HashMap::new(),
        }
    }
    
    // Add or update a relay
    pub fn add_relay(&mut self, drone_id: String, source_url: String, destination_url: String, pool: MySqlPool) -> bool { // Added pool parameter
        tracing::info!("Adding relay for drone {}: {} -> {}", drone_id, source_url, destination_url);
        
        // If relay exists, stop it first
        if let Some(relay_process) = self.relays.get_mut(&drone_id) {
            // Call stop_process as a static method of RelayManager
            RelayManager::stop_process_static(relay_process);
        }
        
        // Create new relay configuration
        let relay = RtmpRelay {
            drone_id: drone_id.clone(),
            source_url,
            destination_url,
            active: false,
        };
        
        // Start ffmpeg process
        // Call start_relay_process as a static method of RelayManager
        let process = RelayManager::start_relay_process_static(&relay, pool.clone()); // Pass pool to start_relay_process_static
        
        self.relays.insert(drone_id, RelayProcess {
            relay,
            process,
            pool, // Store pool in RelayProcess
        });
        
        true
    }
    
    // Remove a relay
    pub fn remove_relay(&mut self, drone_id: &str) -> bool {
        tracing::info!("Removing relay for drone {}", drone_id);
        
        if let Some(mut relay_process) = self.relays.remove(drone_id) {
            // Call stop_process as a static method of RelayManager
            RelayManager::stop_process_static(&mut relay_process);
            true
        } else {
            false
        }
    }

    // Start ffmpeg process for relay
    // Renamed to start_relay_process_static and removed &self
    fn start_relay_process_static(relay: &RtmpRelay, pool: MySqlPool) -> Option<Child> { // Added pool parameter
        let result = Command::new("ffmpeg")
            .arg("-i")
            .arg(&relay.source_url)
            .arg("-c")
            .arg("copy")
            .arg("-f")
            .arg("flv")
            .arg(&relay.destination_url)
            .arg("-progress") // Add progress reporting
            .arg("pipe:2")    // Pipe progress to stderr
            .arg("-loglevel")
            .arg("error")     // Only show errors
            .arg("-hide_banner") // Hide ffmpeg banner
            .stderr(Stdio::piped()) // Capture stderr
            .stdout(Stdio::null())  // Redirect stdout to null
            .spawn();
            
        match result {
            Ok(mut child) => { // child is now mutable
                tracing::info!("ffmpeg relay process succeeded for {} from {} to {}", 
                              relay.drone_id, relay.source_url, relay.destination_url);

                let stderr = child.stderr.take().expect("Failed to capture stderr");
                let reader = BufReader::new(stderr);
                let drone_id_clone = relay.drone_id.clone();
                let pool_clone = pool.clone();

                tokio::spawn(async move {
                    let bitrate_regex = Regex::new(r"bitrate=\s*(\d+\.?\d*)\s*kbits/s").unwrap();
                    for line in reader.lines() {
                        match line {
                            Ok(line_content) => {
                                // tracing::debug!("ffmpeg stderr for {}: {}", drone_id_clone, line_content);
                                if let Some(caps) = bitrate_regex.captures(&line_content) {
                                    if let Some(bitrate_match) = caps.get(1) {
                                        if let Ok(bitrate_kbps) = bitrate_match.as_str().parse::<f32>() {
                                            // tracing::info!("Drone {}: Bitrate: {} kbit/s", drone_id_clone, bitrate_kbps);
                                            // Convert to integer kbit/s for database
                                            let bitrate_int = bitrate_kbps.round() as i32;
                                            match database::add_video_analytics(&pool_clone, drone_id_clone.clone(), bitrate_int).await {
                                                Ok(_) => {}, // tracing::debug!("Successfully saved analytics for {}", drone_id_clone),
                                                Err(e) => tracing::error!("Failed to save analytics for {}: {}", drone_id_clone, e),
                                            }
                                        }
                                    }
                                }
                                // TODO: Add parsing for packet loss if ffmpeg provides it directly in progress.
                                // FFmpeg's default progress output might not directly show packet loss percentage for RTMP copy.
                                // This might require more complex ffmpeg configurations or external tools if detailed packet loss is needed.
                            }
                            Err(e) => {
                                tracing::error!("Error reading stderr line for {}: {}", drone_id_clone, e);
                                break;
                            }
                        }
                    }
                });
                Some(child)
            }
            Err(e) => {
                tracing::error!("ffmpeg relay process failed for {} from {} to {}: {}", 
                               relay.drone_id, relay.source_url, relay.destination_url, e);
                None
            }
        }
    }
    
    // Stop relay process
    // Renamed to stop_process_static and removed &self
    fn stop_process_static(relay_process: &mut RelayProcess) {
        if let Some(process) = &mut relay_process.process {
            tracing::info!("Stopping relay for {}", relay_process.relay.drone_id);
            
            // Try to kill the process gracefully first
            if let Err(e) = process.kill() {
                tracing::warn!("Failed to kill process: {}", e);
            }
            
            // Use try_wait instead of wait to avoid hanging
            match process.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!("Process exited with status: {}", status);
                }
                Ok(None) => {
                    tracing::warn!("Process did not exit immediately after kill signal");
                    // Could implement a timeout here if needed
                }
                Err(e) => {
                    tracing::error!("Error checking process status: {}", e);
                }
            }
        }
    }
}

// Create a global RelayManager
lazy_static::lazy_static! {
    static ref RELAY_MANAGER: Mutex<RelayManager> = Mutex::new(RelayManager::new());
}

// Function to add a new RTMP relay
pub fn add_rtmp_relay(drone_id: String, source_url: String, destination_url: String, pool: MySqlPool) -> bool { // Added pool parameter
    tracing::info!(drone_id = %drone_id, source = %source_url, destination = %destination_url, "rtmp::add_rtmp_relay called");
    let result = match RELAY_MANAGER.lock() {
        Ok(mut manager) => manager.add_relay(drone_id.clone(), source_url.clone(), destination_url.clone(), pool), // Pass pool to manager.add_relay
        Err(e) => {
            tracing::error!(error = %e, "rtmp::add_rtmp_relay failed to acquire relay manager lock");
            false
        }
    };
    tracing::info!(drone_id = %drone_id, added = %result, "rtmp::add_rtmp_relay result");
    result
}

// Function to remove an RTMP relay
pub fn remove_rtmp_relay(drone_id: &str) -> bool {
    tracing::info!(drone_id = %drone_id, "rtmp::remove_rtmp_relay called");
    let result = match RELAY_MANAGER.lock() {
        Ok(mut manager) => manager.remove_relay(drone_id),
        Err(e) => {
            tracing::error!(error = %e, "rtmp::remove_rtmp_relay failed to acquire relay manager lock");
            false
        }
    };
    tracing::info!(drone_id = %drone_id, removed = %result, "rtmp::remove_rtmp_relay result");
    result
}

pub async fn get_drone_analytics_by_id(drone_id: &str, pool: &MySqlPool) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    tracing::info!(drone_id = %drone_id, "rtmp::get_drone_analytics_by_id called");
    
    // Fetch analytics from the database
    let analytics = database::get_video_analytics_by_id(pool, drone_id.to_string()).await?;
    
    tracing::info!(drone_id = %drone_id, count = analytics.len(), "rtmp::get_drone_analytics_by_id result");
    Ok(analytics.iter().map(|(_, bitrate)| *bitrate).collect())
}

// The main RTMP server function - just starts a monitor for the relay processes
pub async fn start_rtmp_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> { // Added pool parameter
    tracing::info!(addr = %addr, "rtmp::start_rtmp_server listening");
    
    // Start a background task to monitor relay processes
    tokio::spawn(async move {
        loop {
            // Sleep for a few seconds
            tokio::time::sleep(Duration::from_secs(30)).await; // Keep monitoring interval
            
            // Check and restart any failed relay processes
            if let Ok(mut manager) = RELAY_MANAGER.lock() {
                // Create a list of drone_ids to iterate over to avoid borrowing issues
                let drone_ids: Vec<String> = manager.relays.keys().cloned().collect();

                for drone_id in drone_ids {
                    if let Some(relay_process) = manager.relays.get_mut(&drone_id) {
                        let current_pool = relay_process.pool.clone(); // Get pool for this relay_process
                        // Check if process is still running
                        if let Some(process) = &mut relay_process.process {
                            match process.try_wait() {
                                Ok(Some(status)) => {
                                        // Process has exited, restart it
                                        tracing::warn!(drone_id = %drone_id, status = ?status, "rtmp relay process exited, restarting");
                                        relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay, current_pool); // Pass pool
                                }
                                Ok(None) => {
                                    // Process is still running
                                    // Analytics are now collected in the spawned task within start_relay_process_static
                                }
                                Err(e) => {
                                        tracing::error!(drone_id = %drone_id, error = %e, "Failed to check relay process status, restarting");
                                        // Try to restart
                                        RelayManager::stop_process_static(relay_process);
                                        relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay, current_pool); // Pass pool
                                }
                            }
                        } else {
                                // No process, try to start one
                                tracing::info!(drone_id = %drone_id, "No relay process found, starting new one");
                                relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay, current_pool); // Pass pool
                        }
                    }
                }
            }
        }
    });
    
    Ok(())
}
