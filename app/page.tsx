"use client"; // Required for components using hooks like useState, useEffect, useRef

import React, { useState, useEffect, useRef, useCallback } from "react";
import { Input } from "@heroui/input"; // Assuming HeroUI has an Input component
import { Button } from "@heroui/button"; // Assuming HeroUI has a Button component
import { Card, CardHeader, CardBody } from "@heroui/card"; // Assuming HeroUI has Card components

// If HeroUI provides specific layout components or styling utilities,
// you might want to import and use them.
// For now, we'll use Tailwind CSS classes for styling.

export default function WebRTCTestPage() {
  const [roomId, setRoomId] = useState("test-room");
  const [localLogs, setLocalLogs] = useState<string[]>([]);
  const [remoteLogs, setRemoteLogs] = useState<string[]>([]);

  const localVideoRef = useRef<HTMLVideoElement>(null);
  const remoteVideoRef = useRef<HTMLVideoElement>(null);
  const localLogRef = useRef<HTMLDivElement>(null);
  const remoteLogRef = useRef<HTMLDivElement>(null);

  // Using refs for mutable objects that don't need to trigger re-renders on change
  const peerConnectionRef = useRef<RTCPeerConnection | null>(null);
  const socketRef = useRef<WebSocket | null>(null);
  const localStreamRef = useRef<MediaStream | null>(null);
  const otherUserIdRef = useRef<string | null>(null);

  const logMessage = useCallback(
    (logType: "local" | "remote", message: string) => {
      const timestamp = new Date().toLocaleTimeString();
      const fullMessage = `[${timestamp}] ${message}`;

      if (logType === "local") {
        setLocalLogs((prevLogs) => [...prevLogs, fullMessage]);
      } else {
        setRemoteLogs((prevLogs) => [...prevLogs, fullMessage]);
      }
    },
    [],
  );

  useEffect(() => {
    if (localLogRef.current) {
      localLogRef.current.scrollTop = localLogRef.current.scrollHeight;
    }
  }, [localLogs]);

  useEffect(() => {
    if (remoteLogRef.current) {
      remoteLogRef.current.scrollTop = remoteLogRef.current.scrollHeight;
    }
  }, [remoteLogs]);

  const setupWebRTC = useCallback(async () => {
    try {
      // Check if MediaDevices API is available
      if (
        typeof navigator === "undefined" ||
        !navigator.mediaDevices ||
        !navigator.mediaDevices.getUserMedia
      ) {
        logMessage(
          "local",
          "MediaDevices API (getUserMedia) is not available in this browser or context. Ensure you are on HTTPS or localhost.",
        );
        console.error(
          "MediaDevices API (getUserMedia) not available. Ensure you are on HTTPS or localhost.",
        );

        return false;
      }

      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: true,
      });

      if (localVideoRef.current) {
        localVideoRef.current.srcObject = stream;
      }
      localStreamRef.current = stream;
      logMessage("local", "Local stream acquired.");

      const config = {
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
      };
      const pc = new RTCPeerConnection(config);

      peerConnectionRef.current = pc;

      stream.getTracks().forEach((track) => {
        pc.addTrack(track, stream);
      });
      logMessage("local", "Tracks added to PeerConnection.");

      pc.onicecandidate = (event) => {
        if (
          event.candidate &&
          socketRef.current &&
          socketRef.current.readyState === WebSocket.OPEN
        ) {
          logMessage("local", "Sending ICE candidate");
          socketRef.current.send(
            JSON.stringify({
              type: "candidate",
              candidate: event.candidate,
              room: roomId, // Send room for context on server
              // to: otherUserIdRef.current // You might need a way to target specific users
            }),
          );
        }
      };

      pc.ontrack = (event) => {
        logMessage("remote", "Received remote track");
        if (remoteVideoRef.current) {
          remoteVideoRef.current.srcObject = event.streams[0];
        }
      };
      logMessage("local", "WebRTC setup complete.");

      return true;
    } catch (error: any) {
      logMessage("local", "Error setting up WebRTC: " + error.message);
      console.error("WebRTC Setup Error:", error);

      return false;
    }
  }, [logMessage, roomId]);

  const connectSignaling = useCallback(
    (currentRoomId: string) => {
      const url = `wss://v4.backend.notsofar.live/signaling`;
      const ws = new WebSocket(url);

      socketRef.current = ws;

      ws.onopen = () => {
        logMessage("local", "Connected to signaling server");
        ws.send(
          JSON.stringify({
            type: "join",
            room: currentRoomId,
          }),
        );
        logMessage("local", `Sent join request for room: ${currentRoomId}`);
      };

      ws.onmessage = async (event) => {
        try {
          const message = JSON.parse(event.data as string);

          logMessage(
            "local",
            `Received message: ${message.type} from ${message.from || "server"}`,
          );

          if (!peerConnectionRef.current) {
            logMessage(
              "local",
              "PeerConnection not initialized. Ignoring message.",
            );

            return;
          }
          const pc = peerConnectionRef.current;

          switch (message.type) {
            case "offer":
              // Make sure not to process our own offer if server broadcasts it back
              // This check depends on how your server sends 'from'
              if (
                message.from &&
                socketRef.current &&
                message.from === (socketRef.current as any).userId
              ) {
                // (socketRef.current as any).userId is a placeholder for your client's ID
                logMessage("local", "Ignoring own offer.");

                return;
              }
              otherUserIdRef.current = message.from;
              logMessage("local", `Processing offer from ${message.from}`);
              await pc.setRemoteDescription(
                new RTCSessionDescription(message.offer),
              );
              const answer = await pc.createAnswer();

              await pc.setLocalDescription(answer);
              ws.send(
                JSON.stringify({
                  type: "answer",
                  answer: answer,
                  room: currentRoomId,
                  // to: message.from // Target the sender of the offer
                }),
              );
              logMessage("local", `Sent answer to ${message.from}`);
              break;
            case "answer":
              if (
                message.from &&
                socketRef.current &&
                message.from === (socketRef.current as any).userId
              ) {
                logMessage("local", "Ignoring own answer.");

                return;
              }
              logMessage("local", `Processing answer from ${message.from}`);
              await pc.setRemoteDescription(
                new RTCSessionDescription(message.answer),
              );
              break;
            case "candidate":
              if (
                message.from &&
                socketRef.current &&
                message.from === (socketRef.current as any).userId
              ) {
                logMessage("local", "Ignoring own ICE candidate.");

                return;
              }
              if (message.candidate) {
                logMessage(
                  "local",
                  `Adding ICE candidate from ${message.from}`,
                );
                await pc.addIceCandidate(
                  new RTCIceCandidate(message.candidate),
                );
              }
              break;
            default:
              logMessage("local", `Unknown message type: ${message.type}`);
          }
        } catch (error: any) {
          logMessage("local", "Error processing message: " + error.message);
          console.error("WebSocket Message Error:", error);
        }
      };

      ws.onclose = () => {
        logMessage("local", "Disconnected from signaling server");
      };

      ws.onerror = (error) => {
        logMessage("local", "WebSocket error: " + (error as any).message);
        console.error("WebSocket Error:", error);
      };
    },
    [logMessage, roomId],
  );

  const handleJoinRoom = useCallback(async () => {
    if (!roomId.trim()) {
      alert("Please enter a room ID");

      return;
    }
    logMessage("local", `Attempting to join room: ${roomId}`);

    // Reset logs for new session
    setLocalLogs([]);
    setRemoteLogs([]);

    // Close existing connections if any
    if (peerConnectionRef.current) {
      peerConnectionRef.current.close();
      peerConnectionRef.current = null;
      logMessage("local", "Previous PeerConnection closed.");
    }
    if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
      socketRef.current.close();
      socketRef.current = null;
      logMessage("local", "Previous WebSocket connection closed.");
    }
    if (localStreamRef.current) {
      localStreamRef.current.getTracks().forEach((track) => track.stop());
      localStreamRef.current = null;
      logMessage("local", "Previous local stream stopped.");
    }
    if (localVideoRef.current) localVideoRef.current.srcObject = null;
    if (remoteVideoRef.current) remoteVideoRef.current.srcObject = null;

    const rtcReady = await setupWebRTC();

    if (rtcReady) {
      connectSignaling(roomId);

      // Small delay to ensure WebSocket is connected before sending offer
      setTimeout(async () => {
        if (
          peerConnectionRef.current &&
          socketRef.current &&
          socketRef.current.readyState === WebSocket.OPEN
        ) {
          try {
            logMessage("local", "Creating offer...");
            const offer = await peerConnectionRef.current.createOffer();

            await peerConnectionRef.current.setLocalDescription(offer);
            socketRef.current.send(
              JSON.stringify({
                type: "offer",
                offer: offer,
                room: roomId,
                // to: "all" // Or target specific user if known
              }),
            );
            logMessage("local", "Offer sent.");
          } catch (error: any) {
            logMessage(
              "local",
              "Error creating or sending offer: " + error.message,
            );
            console.error("Offer Error:", error);
          }
        } else {
          logMessage(
            "local",
            "Cannot send offer: PeerConnection or WebSocket not ready.",
          );
        }
      }, 2000); // Increased delay slightly
    }
  }, [roomId, setupWebRTC, connectSignaling, logMessage]);

  // Cleanup on component unmount
  useEffect(() => {
    return () => {
      logMessage("local", "WebRTCTestPage unmounting. Cleaning up...");
      if (localStreamRef.current) {
        localStreamRef.current.getTracks().forEach((track) => track.stop());
        logMessage("local", "Local stream stopped.");
      }
      if (peerConnectionRef.current) {
        peerConnectionRef.current.close();
        logMessage("local", "PeerConnection closed.");
      }
      if (
        socketRef.current &&
        socketRef.current.readyState === WebSocket.OPEN
      ) {
        socketRef.current.close();
        logMessage("local", "WebSocket connection closed.");
      }
    };
  }, [logMessage]);

  return (
    <section className="flex flex-col items-center justify-center gap-4 py-8 md:py-10">
      <h1 className="text-3xl font-bold">WebRTC Signaling Test</h1>

      <div className="flex items-center gap-2 my-4">
        <Input
          label="Room ID"
          type="text"
          value={roomId}
          onChange={(e) => setRoomId(e.target.value)}
          placeholder="Enter Room ID"
          // Assuming HeroUI Input has these props or similar
        />
        <Button color="primary" onPress={handleJoinRoom}>
          Join Room
        </Button>
      </div>

      <div className="container mx-auto grid grid-cols-1 md:grid-cols-2 gap-4 w-full max-w-6xl px-4">
        <Card className="w-full">
          <CardHeader>
            <h2 className="text-xl font-semibold">Local Peer</h2>
          </CardHeader>
          <CardBody>
            <video
              ref={localVideoRef}
              autoPlay
              muted
              playsInline
              className="w-full h-64 bg-black rounded-md mb-2"
            />
            <div
              ref={localLogRef}
              className="log h-48 overflow-y-auto bg-gray-100 dark:bg-gray-800 p-2 border border-gray-300 dark:border-gray-700 rounded-md text-xs"
            >
              {localLogs.map((log, index) => (
                <div key={`local-${index}`}>{log}</div>
              ))}
            </div>
          </CardBody>
        </Card>

        <Card className="w-full">
          <CardHeader>
            <h2 className="text-xl font-semibold">Remote Peer</h2>
          </CardHeader>
          <CardBody>
            <video
              ref={remoteVideoRef}
              autoPlay
              playsInline
              className="w-full h-64 bg-black rounded-md mb-2"
            >
              <track kind="captions" />
            </video>
            <div
              ref={remoteLogRef}
              className="log h-48 overflow-y-auto bg-gray-100 dark:bg-gray-800 p-2 border border-gray-300 dark:border-gray-700 rounded-md text-xs"
            >
              {remoteLogs.map((log, index) => (
                <div key={`remote-${index}`}>{log}</div>
              ))}
            </div>
          </CardBody>
        </Card>
      </div>
    </section>
  );
}
