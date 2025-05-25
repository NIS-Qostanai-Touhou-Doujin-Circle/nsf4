#!/usr/bin/env python3
"""
Smooth and predictable drone GPS simulator
Follows the structure of drone_simulator.rs but with enhanced smoothness
"""

import asyncio
import websockets
import json
import time
import math
import sys
from datetime import datetime, timezone
from typing import Dict, Any, Optional
import random


class DronePhysics:
    """Physics-based drone motion simulator with smooth acceleration and predictable patterns"""
    
    def __init__(self, init_lat: float, init_lng: float, init_alt: float):
        # Base center for circular flight (degrees)
        self.base_lat = init_lat
        self.base_lng = init_lng
        
        # Circular flight state - smoother parameters than Rust version
        self.angle = 0.0  # radians
        self.angular_speed = 0.15  # rad/s (~42s per circle, slower for smoothness)
        self.radius = 0.0008  # degrees offset (~80m, slightly larger for visibility)
        
        # Current position (degrees)
        self.lat = init_lat + self.radius * math.cos(self.angle)
        self.lng = init_lng + self.radius * math.sin(self.angle)
        self.alt = init_alt
        
        # Velocity (degrees/second) - computed from circular motion
        self.vel_lat = -self.radius * self.angular_speed * math.sin(self.angle)
        self.vel_lng = self.radius * self.angular_speed * math.cos(self.angle)
        self.vel_alt = 0.0
        
        # Acceleration (degrees/second²) - computed from circular motion
        self.acc_lat = -self.radius * self.angular_speed ** 2 * math.cos(self.angle)
        self.acc_lng = -self.radius * self.angular_speed ** 2 * math.sin(self.angle)
        self.acc_alt = 0.0
        
        # Target waypoint (follows current position for smooth tracking)
        self.target_lat = self.lat
        self.target_lng = self.lng
        self.target_alt = self.alt
        
        # Smoothing parameters
        self.smooth_factor = 0.95  # For exponential smoothing
        self.noise_amplitude = 0.000001  # Very small noise for realism
        
    def update(self, dt: float) -> None:
        """Update drone physics with smooth circular motion"""
        # Advance angle for circular flight with smooth progression
        self.angle = (self.angle + self.angular_speed * dt) % (2.0 * math.pi)
        
        # Compute smooth circular position
        new_lat = self.base_lat + self.radius * math.cos(self.angle)
        new_lng = self.base_lng + self.radius * math.sin(self.angle)
        
        # Apply exponential smoothing for ultra-smooth transitions
        self.lat = self.smooth_factor * self.lat + (1 - self.smooth_factor) * new_lat
        self.lng = self.smooth_factor * self.lng + (1 - self.smooth_factor) * new_lng
        
        # Add tiny amount of realistic noise
        noise_lat = (random.random() - 0.5) * self.noise_amplitude
        noise_lng = (random.random() - 0.5) * self.noise_amplitude
        self.lat += noise_lat
        self.lng += noise_lng
        
        # Compute smooth velocity components from circular motion
        raw_vel_lat = -self.radius * self.angular_speed * math.sin(self.angle)
        raw_vel_lng = self.radius * self.angular_speed * math.cos(self.angle)
        
        # Apply smoothing to velocity
        self.vel_lat = self.smooth_factor * self.vel_lat + (1 - self.smooth_factor) * raw_vel_lat
        self.vel_lng = self.smooth_factor * self.vel_lng + (1 - self.smooth_factor) * raw_vel_lng
        self.vel_alt = 0.0  # Constant altitude
        
        # Compute smooth acceleration (centripetal acceleration for circular motion)
        raw_acc_lat = -self.radius * self.angular_speed ** 2 * math.cos(self.angle)
        raw_acc_lng = -self.radius * self.angular_speed ** 2 * math.sin(self.angle)
        
        # Apply smoothing to acceleration
        self.acc_lat = self.smooth_factor * self.acc_lat + (1 - self.smooth_factor) * raw_acc_lat
        self.acc_lng = self.smooth_factor * self.acc_lng + (1 - self.smooth_factor) * raw_acc_lng
        self.acc_alt = 0.0
        
        # Update target to current position for smooth tracking
        self.target_lat = self.lat
        self.target_lng = self.lng
        self.target_alt = self.alt


class DroneSimulator:
    """WebSocket-based drone simulator server"""
    
    def __init__(self, port: int = 9002, drone_id: str = "drone-sim-1"):
        self.port = port
        self.drone_id = drone_id
        
        # Base coordinates (same as Rust version)
        self.base_latitude = 53.218282
        self.base_longitude = 63.658686
        self.base_altitude = 120.0
        
        # Physics update rate (100 Hz for smooth simulation)
        self.physics_update_interval = 0.01  # 10ms
        
    async def handle_client(self, websocket):
        """Handle individual WebSocket client connection"""
        client_addr = websocket.remote_address
        print(f"Incoming connection from: {client_addr}")
        
        try:
            # Initialize drone physics
            drone_physics = DronePhysics(
                self.base_latitude, 
                self.base_longitude, 
                self.base_altitude
            )
            
            # Send welcome message
            welcome_msg = {
                "type": "info",
                "message": "Connected to drone simulator",
                "drone_id": self.drone_id
            }
            await websocket.send(json.dumps(welcome_msg))
            print(f"WebSocket connection established with: {client_addr}")
            
            # Start physics simulation loop
            last_time = time.time()
            
            async def physics_loop():
                """Continuous physics update loop"""
                nonlocal last_time
                
                while True:
                    try:
                        current_time = time.time()
                        dt = current_time - last_time
                        last_time = current_time
                        
                        # Update physics
                        drone_physics.update(dt)
                        
                        # Create GPS update message
                        gps_update = {
                            "type": "gps",
                            "drone_id": self.drone_id,
                            "latitude": drone_physics.lat,
                            "longitude": drone_physics.lng,
                            "altitude": drone_physics.alt,
                            "velocity": {
                                "lat": drone_physics.vel_lat,
                                "lng": drone_physics.vel_lng,
                                "alt": drone_physics.vel_alt
                            },
                            "acceleration": {
                                "lat": drone_physics.acc_lat,
                                "lng": drone_physics.acc_lng,
                                "alt": drone_physics.acc_alt
                            },
                            "target": {
                                "lat": drone_physics.target_lat,
                                "lng": drone_physics.target_lng,
                                "alt": drone_physics.target_alt
                            },
                            "timestamp": datetime.now(timezone.utc).isoformat()
                        }
                        
                        # Send GPS update
                        await websocket.send(json.dumps(gps_update))
                        
                        # Wait for next physics update
                        await asyncio.sleep(self.physics_update_interval)
                        
                    except websockets.exceptions.ConnectionClosed:
                        print(f"Client {client_addr} disconnected during physics loop")
                        break
                    except Exception as e:
                        print(f"Error in physics loop: {e}")
                        break
            
            async def message_handler():
                """Handle incoming messages from client"""
                try:
                    async for message in websocket:
                        await asyncio.sleep(0.1)  # Small delay like in Rust version
                        
                        try:
                            data = json.loads(message)
                            print(f"Received message: {message}")
                            # Add command processing here if needed
                        except json.JSONDecodeError:
                            print(f"Invalid JSON received: {message}")
                            
                except websockets.exceptions.ConnectionClosed:
                    print(f"Client {client_addr} disconnected")
                except Exception as e:
                    print(f"Error handling messages: {e}")
            
            # Run physics loop and message handler concurrently
            await asyncio.gather(
                physics_loop(),
                message_handler(),
                return_exceptions=True
            )
            
        except websockets.exceptions.ConnectionClosed:
            print(f"Client {client_addr} disconnected")
        except Exception as e:
            print(f"Error during WebSocket handling: {e}")
    
    async def start_server(self):
        """Start the WebSocket server"""
        addr = f"0.0.0.0:{self.port}"
        print(f"Drone simulator starting on {addr} with ID {self.drone_id}")
        
        server = await websockets.serve(
            self.handle_client,
            "0.0.0.0",
            self.port
        )
        
        print(f"WebSocket server listening on: {addr}")
        
        # Keep server running
        await server.wait_closed()


async def main():
    """Main entry point"""
    # Parse command line arguments
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9002
    drone_id = sys.argv[2] if len(sys.argv) > 2 else "drone-sim-1"
    
    # Create and start simulator
    simulator = DroneSimulator(port, drone_id)
    
    try:
        await simulator.start_server()
    except KeyboardInterrupt:
        print("\nShutting down drone simulator...")
    except Exception as e:
        print(f"Error: {e}")


if __name__ == "__main__":
    asyncio.run(main())