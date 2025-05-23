"use client";

import { Button } from "@heroui/button";
import { Input } from "@heroui/input";
import { useCallback, useEffect, useRef, useState } from "react";

import * as api from "./s2c";

export default function Page() {
  const [me, setMe] = useState<string>("");

  useEffect(() => {
    if (!me && typeof window !== "undefined" && window.crypto?.randomUUID) {
      setMe(window.crypto.randomUUID());
    }
  }, [me]);

  const [roomId, setRoomId] = useState<string>("");
  const [displayName, setDisplayName] = useState<string>("");
  const [roomInfo, setRoomInfo] = useState<api.RoomInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<api.RoomWebSocket | null>(null);

  const handleCreateRoom = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const created = await api.createRoom(roomId, me);

      setRoomInfo(created);
    } catch (err: any) {
      setError(err.message || "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [roomId, me]);

  const handleJoinRoom = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const joined = await api.joinRoom(roomId, me, displayName);

      setRoomInfo(joined);
      // Fetch latest room info to ensure displayName is updated
      const info = await api.getRoomInfo(roomId);

      setRoomInfo(info);
    } catch (err: any) {
      setError(err.message || "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [roomId, me, displayName]);

  const handleLeaveRoom = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await api.leaveRoom(roomId, me);
      setRoomInfo(null);
    } catch (err: any) {
      setError(err.message || "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [roomId, me]);

  const handleRefreshRoom = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const info = await api.getRoomInfo(roomId);

      setRoomInfo(info);
    } catch (err: any) {
      setError(err.message || "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [roomId]);

  // Track join requests for the creator
  const [joinRequests, setJoinRequests] = useState<
    { user_id: string; display_name: string }[]
  >([]);

  // WebRTC state
  const [localStream, setLocalStream] = useState<MediaStream | null>(null);
  const [peers, setPeers] = useState<Record<string, RTCPeerConnection>>({});
  const [remoteStreams, setRemoteStreams] = useState<
    Record<string, MediaStream>
  >({});
  const [videoOn, setVideoOn] = useState(true);
  const [audioOn, setAudioOn] = useState(true);

  // Connect/disconnect WebSocket when joined to a room
  useEffect(() => {
    if (roomInfo && roomId && me) {
      wsRef.current = api.connectRoomWebSocket(roomId, me);
      wsRef.current.addMessageHandler((msg: any) => {
        // Example: handle Connect/Disconnect/MediaStatus events
        if (msg.event === "Connect" || msg.event === "Disconnect") {
          handleRefreshRoom();
        }
        if (msg.event === "JoinRequest" && roomInfo.creator_id === me) {
          setJoinRequests((prev) => {
            // Avoid duplicates
            if (prev.some((r) => r.user_id === msg.user_id)) return prev;

            return [
              ...prev,
              { user_id: msg.user_id, display_name: msg.display_name },
            ];
          });
        }
        if (
          (msg.event === "JoinApproved" || msg.event === "JoinDenied") &&
          roomInfo.creator_id === me
        ) {
          // Remove from joinRequests if handled
          if ("user_id" in msg) {
            setJoinRequests((prev) =>
              prev.filter((r) => r.user_id !== msg.user_id),
            );
          }
        }
      });
    } else {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      setJoinRequests([]);
    }

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      setJoinRequests([]);
    };
  }, [roomInfo, roomId, me]);

  // Approve/Deny join request handlers
  const handleApproveJoin = (user_id: string) => {
    wsRef.current?.send({ event: "ApproveJoinRequest", user_id });
    setJoinRequests((prev) => prev.filter((r) => r.user_id !== user_id));
  };
  const handleDenyJoin = (user_id: string) => {
    wsRef.current?.send({ event: "DenyJoinRequest", user_id });
    setJoinRequests((prev) => prev.filter((r) => r.user_id !== user_id));
  };

  // Get user media on mount
  useEffect(() => {
    let stopped = false;

    navigator.mediaDevices
      .getUserMedia({ video: true, audio: true })
      .then((stream) => {
        if (!stopped) setLocalStream(stream);
      })
      .catch(() => setLocalStream(null));

    return () => {
      stopped = true;
      setLocalStream((s) => {
        s?.getTracks().forEach((t) => t.stop());

        return null;
      });
    };
  }, []);

  // Toggle video/audio
  const handleToggleVideo = useCallback(() => {
    setVideoOn((on) => {
      localStream?.getVideoTracks().forEach((t) => (t.enabled = !on));
      // Only update backend if joined to a room
      if (roomId && roomInfo) {
        api.updateMediaStatus(roomId, me, { video: !on, audio: audioOn });
      }

      return !on;
    });
  }, [localStream, roomId, me, audioOn, roomInfo]);
  const handleToggleAudio = useCallback(() => {
    setAudioOn((on) => {
      localStream?.getAudioTracks().forEach((t) => (t.enabled = !on));
      // Only update backend if joined to a room
      if (roomId && roomInfo) {
        api.updateMediaStatus(roomId, me, { video: videoOn, audio: !on });
      }

      return !on;
    });
  }, [localStream, roomId, me, videoOn, roomInfo]);

  // --- WebRTC Peer Connection Management ---
  useEffect(() => {
    if (
      !roomInfo ||
      !("participants" in roomInfo) ||
      !Array.isArray((roomInfo as any).participants) ||
      !localStream
    )
      return;
    const participants = (roomInfo as any).participants;
    const otherIds = participants
      .map((p: any) => p.user.id)
      .filter((id: string) => id !== me);

    // Create peer connections for new participants
    otherIds.forEach((id: string) => {
      if (peers[id]) return; // already connected
      console.log(`[WebRTC] Creating RTCPeerConnection for peer ${id}`);
      const pc = new RTCPeerConnection({
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
      });

      // Add local tracks
      localStream.getTracks().forEach((track) => {
        pc.addTrack(track, localStream);
        console.log(`[WebRTC] Added local track (${track.kind}) to peer ${id}`);
      });
      // Handle remote tracks
      pc.ontrack = (event) => {
        console.log(
          `[WebRTC] ontrack fired for peer ${id}. event.streams:`,
          event.streams,
          "event.track:",
          event.track,
        );
        setRemoteStreams((prev) => {
          let stream = event.streams && event.streams[0];

          if (!stream) {
            // Fallback: accumulate tracks manually
            const existing = prev[id] || new MediaStream();

            existing.addTrack(event.track);
            stream = existing;
            console.log(
              `[WebRTC] Fallback: created/updated MediaStream for peer ${id} with track`,
              event.track,
            );
          } else {
            console.log(`[WebRTC] Using event.streams[0] for peer ${id}`);
          }

          return { ...prev, [id]: stream };
        });
      };
      // ICE candidate handling
      pc.onicecandidate = (event) => {
        if (event.candidate && wsRef.current) {
          console.log(`[WebRTC] Sending ICE candidate to peer ${id}`);
          wsRef.current.send({
            event: "WebRTC",
            message: {
              type: "ice",
              candidate: event.candidate,
              to: id,
              from: me,
            },
          });
        }
      };
      setPeers((prev) => ({ ...prev, [id]: pc }));
      console.log(`[WebRTC] Peer connection for ${id} added to peers.`);
    });

    // Remove peer connections for users who left
    Object.keys(peers).forEach((id) => {
      if (!otherIds.includes(id)) {
        peers[id].close();
        setPeers((prev) => {
          const copy = { ...prev };

          delete copy[id];

          return copy;
        });
        setRemoteStreams((prev) => {
          const copy = { ...prev };

          delete copy[id];

          return copy;
        });
      }
    });
  }, [roomInfo, localStream]);

  // --- WebRTC Signaling via WebSocket ---
  useEffect(() => {
    if (!wsRef.current) return;
    const ws = wsRef.current;
    const handler = async (msg: any) => {
      if (msg.event === "WebRTC" && msg.message) {
        const { type, sdp, candidate, from, to } = msg.message;

        if (to !== me) return;
        let pc = peers[from];

        if (!pc && localStream) {
          pc = new RTCPeerConnection({
            iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
          });
          // Set ontrack handler before adding tracks/signaling
          pc.ontrack = (event) => {
            setRemoteStreams((prev) => {
              let stream = event.streams && event.streams[0];

              if (!stream) {
                // Fallback: accumulate tracks manually
                const existing = prev[from] || new MediaStream();

                existing.addTrack(event.track);
                stream = existing;
              }

              return { ...prev, [from]: stream };
            });
          };
          pc.onicecandidate = (event) => {
            if (event.candidate && wsRef.current) {
              wsRef.current.send({
                event: "WebRTC",
                message: {
                  type: "ice",
                  candidate: event.candidate,
                  to: from,
                  from: me,
                },
              });
            }
          };
          // Add local tracks
          localStream
            .getTracks()
            .forEach((track) => pc.addTrack(track, localStream));
          setPeers((prev) => ({ ...prev, [from]: pc }));
        }
        if (!pc) return;
        if (type === "offer") {
          await pc.setRemoteDescription(
            new RTCSessionDescription({ type: "offer", sdp }),
          );
          const answer = await pc.createAnswer();

          await pc.setLocalDescription(answer);
          ws.send({
            event: "WebRTC",
            message: { type: "answer", sdp: answer.sdp, to: from, from: me },
          });
        } else if (type === "answer") {
          await pc.setRemoteDescription(
            new RTCSessionDescription({ type: "answer", sdp }),
          );
        } else if (type === "ice" && candidate) {
          try {
            await pc.addIceCandidate(new RTCIceCandidate(candidate));
          } catch (e) {
            // ignore
          }
        }
      }
    };

    ws.addMessageHandler(handler);

    return () => ws.removeMessageHandler(handler);
  }, [wsRef.current, peers, localStream, me]);

  // --- Initiate Offer to New Peers ---
  useEffect(() => {
    if (
      !roomInfo ||
      !("participants" in roomInfo) ||
      !Array.isArray((roomInfo as any).participants) ||
      !localStream
    )
      return;
    const participants = (roomInfo as any).participants;
    const otherIds = participants
      .map((p: any) => p.user.id)
      .filter((id: string) => id !== me);

    otherIds.forEach(async (id: string) => {
      const pc = peers[id];

      if (pc && pc.signalingState === "stable" && wsRef.current) {
        // Only offer if we are the lower id (to avoid double-offer)
        if (me < id) {
          const offer = await pc.createOffer();

          await pc.setLocalDescription(offer);
          wsRef.current.send({
            event: "WebRTC",
            message: { type: "offer", sdp: offer.sdp, to: id, from: me },
          });
        }
      }
    });
  }, [peers, roomInfo, localStream, me]);

  // Diagnostics for WebRTC peer connections
  useEffect(() => {
    if (
      !roomInfo ||
      !("participants" in roomInfo) ||
      !Array.isArray((roomInfo as any).participants)
    )
      return;
    const participants = (roomInfo as any).participants;

    console.log("[WebRTC] My ID:", me);
    console.log(
      "[WebRTC] Participants:",
      participants.map((p: any) => p.user.id),
    );
    console.log("[WebRTC] Peers:", Object.keys(peers));
    console.log("[WebRTC] Remote Streams:", Object.keys(remoteStreams));
    if (localStream) {
      console.log(
        "[WebRTC] Local stream tracks:",
        localStream
          .getTracks()
          .map((t) => ({ kind: t.kind, enabled: t.enabled })),
      );
    } else {
      console.log("[WebRTC] No local stream");
    }
  }, [roomInfo, peers, remoteStreams, localStream, me]);

  // Render
  return (
    <div>
      <Input label="room id" value={roomId} onValueChange={setRoomId} />
      <Input
        label="display name"
        value={displayName}
        onValueChange={setDisplayName}
      />
      <div className="flex gap-2 mt-4">
        <Button isDisabled={loading || !roomId} onPress={handleCreateRoom}>
          {loading ? "Creating..." : "Create Room"}
        </Button>
        <Button isDisabled={loading || !roomId} onPress={handleJoinRoom}>
          {loading ? "Joining..." : "Join Room"}
        </Button>
        <Button isDisabled={loading || !roomId} onPress={handleRefreshRoom}>
          Refresh
        </Button>
        <Button
          isDisabled={loading || !roomId || !roomInfo}
          onPress={handleLeaveRoom}
        >
          Leave Room
        </Button>
        <Button isDisabled={!localStream} onPress={handleToggleVideo}>
          {videoOn ? "Turn Video Off" : "Turn Video On"}
        </Button>
        <Button isDisabled={!localStream} onPress={handleToggleAudio}>
          {audioOn ? "Mute" : "Unmute"}
        </Button>
      </div>
      {error && <div className="text-red-500 mt-2">{error}</div>}
      {roomInfo && (
        <div className="mt-4 p-4 bg-slate-900 rounded-xl">
          <div>
            Room ID:{" "}
            {typeof roomInfo === "object" &&
            roomInfo !== null &&
            "id" in roomInfo &&
            typeof roomInfo.id === "string"
              ? roomInfo.id
              : typeof roomInfo === "object" &&
                  roomInfo !== null &&
                  "room_id" in roomInfo &&
                  typeof roomInfo.room_id === "string"
                ? roomInfo.room_id
                : "-"}
          </div>
          <div>Creator: {roomInfo.creator_id ?? "-"} </div>
          <div>
            Users:{" "}
            {"participants" in roomInfo &&
            Array.isArray(roomInfo.participants) &&
            roomInfo.participants.length > 0
              ? roomInfo.participants
                  .map((p: any) => {
                    const name = p?.user?.display_name?.trim();

                    return name ? name : p?.user?.id || "-";
                  })
                  .join(", ")
              : "users" in roomInfo &&
                  Array.isArray(roomInfo.users) &&
                  roomInfo.users.length > 0
                ? roomInfo.users.join(", ")
                : "-"}
          </div>
          {/* Show join requests if I'm the creator */}
          {roomInfo.creator_id === me && joinRequests.length > 0 && (
            <div className="mt-4">
              <div className="font-bold mb-2">Join Requests:</div>
              <ul className="space-y-2">
                {joinRequests.map((req) => (
                  <li key={req.user_id} className="flex items-center gap-2">
                    <span>
                      {req.display_name} ({req.user_id})
                    </span>
                    <Button
                      size="sm"
                      onPress={() => handleApproveJoin(req.user_id)}
                    >
                      Approve
                    </Button>
                    <Button
                      size="sm"
                      onPress={() => handleDenyJoin(req.user_id)}
                    >
                      Deny
                    </Button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
      <div className="grid grid-cols-2 gap-x-8 mt-8">
        {/* Local video always first */}
        <VideoContainer>
          <video
            ref={useCallback(
              (el: HTMLVideoElement | null) => {
                if (el && localStream && el.srcObject !== localStream) {
                  el.srcObject = localStream;
                }
              },
              [localStream],
            )}
            autoPlay
            muted
            playsInline
            style={{ width: "100%", background: "#222" }}
          />
          <div className="text-center text-xs mt-1">
            Me ({displayName || me})
          </div>
        </VideoContainer>
        {/* Remote videos */}
        {roomInfo &&
        "participants" in roomInfo &&
        Array.isArray(roomInfo.participants)
          ? roomInfo.participants
              .filter((p: any) => p.user.id !== me)
              .map((p: any) => (
                <VideoContainer key={p.user.id}>
                  <video
                    ref={(el) => {
                      if (el && remoteStreams[p.user.id])
                        el.srcObject = remoteStreams[p.user.id];
                    }}
                    autoPlay
                    playsInline
                    style={{ width: "100%", background: "#222" }}
                  />
                  <div className="text-center text-xs mt-1">
                    {p.user.display_name || p.user.id}
                  </div>
                </VideoContainer>
              ))
          : null}
      </div>
    </div>
  );
}

function VideoContainer({ children }: { children: React.ReactNode }) {
  return (
    <div className="bg-slate-400 rounded-xl overflow-hidden shadow-lg">
      {children}
    </div>
  );
}
