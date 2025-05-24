use std::net::SocketAddr;
use std::process::{Command, Child};
use std::sync::Mutex;
use std::collections::HashMap;
use tokio::time::Duration;
use serde::{Deserialize, Serialize};

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
    pub fn add_relay(&mut self, drone_id: String, source_url: String, destination_url: String) -> bool {
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
        let process = RelayManager::start_relay_process_static(&relay);
        
        self.relays.insert(drone_id, RelayProcess {
            relay,
            process,
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
    fn start_relay_process_static(relay: &RtmpRelay) -> Option<Child> {
        tracing::info!("Starting ffmpeg relay process for {} from {} to {}", 
                      relay.drone_id, relay.source_url, relay.destination_url);
        
        let result = Command::new("ffmpeg")
            .arg("-i")
            .arg(&relay.source_url)
            .arg("-c")
            .arg("copy")
            .arg("-f")
            .arg("flv")
            .arg(&relay.destination_url)
            .spawn();
            
        match result {
            Ok(child) => {
                tracing::info!("Started ffmpeg relay for {}", relay.drone_id);
                Some(child)
            }
            Err(e) => {
                tracing::error!("Failed to start ffmpeg relay for {}: {}", relay.drone_id, e);
                None
            }
        }
    }
    
    // Stop relay process
    // Renamed to stop_process_static and removed &self
    fn stop_process_static(relay_process: &mut RelayProcess) {
        if let Some(process) = &mut relay_process.process {
            tracing::info!("Stopping relay for {}", relay_process.relay.drone_id);
            
            // Try to kill the process gracefully
            if let Err(e) = process.kill() {
                tracing::error!("Failed to kill relay process: {}", e);
            }
        }
    }
}

// Create a global RelayManager
lazy_static::lazy_static! {
    static ref RELAY_MANAGER: Mutex<RelayManager> = Mutex::new(RelayManager::new());
}

// Function to add a new RTMP relay
pub fn add_rtmp_relay(drone_id: String, source_url: String, destination_url: String) -> bool {
    tracing::info!(drone_id = %drone_id, source = %source_url, destination = %destination_url, "rtmp::add_rtmp_relay called");
    let result = match RELAY_MANAGER.lock() {
        Ok(mut manager) => manager.add_relay(drone_id.clone(), source_url.clone(), destination_url.clone()),
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

// The main RTMP server function - just starts a monitor for the relay processes
pub async fn start_rtmp_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(addr = %addr, "rtmp::start_rtmp_server listening");
    
    // Start a background task to monitor relay processes
    tokio::spawn(async move {
        loop {
            // Sleep for a few seconds
            tokio::time::sleep(Duration::from_secs(30)).await;
            
            // Check and restart any failed relay processes
            if let Ok(mut manager) = RELAY_MANAGER.lock() {
                // Create a list of drone_ids to iterate over to avoid borrowing issues
                let drone_ids: Vec<String> = manager.relays.keys().cloned().collect();

                for drone_id in drone_ids {
                    if let Some(relay_process) = manager.relays.get_mut(&drone_id) {
                        // Check if process is still running
                        if let Some(process) = &mut relay_process.process {
                            match process.try_wait() {
                                Ok(Some(status)) => {
                                        // Process has exited, restart it
                                        tracing::warn!(drone_id = %drone_id, status = ?status, "rtmp relay process exited, restarting");
                                        relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay);
                                }
                                Ok(None) => {
                                    // Process is still running
                                }
                                Err(e) => {
                                        tracing::error!(drone_id = %drone_id, error = %e, "Failed to check relay process status, restarting");
                                        // Try to restart
                                        RelayManager::stop_process_static(relay_process);
                                        relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay);
                                }
                            }
                        } else {
                                // No process, try to start one
                                tracing::info!(drone_id = %drone_id, "No relay process found, starting new one");
                                relay_process.process = RelayManager::start_relay_process_static(&relay_process.relay);
                        }
                    }
                }
            }
        }
    });
    
    Ok(())
}
