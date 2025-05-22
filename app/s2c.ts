const BASE_URL = process.env.NODE_ENV === "development"
  ? "http://localhost:3031"
  : "https://your-production-url.com";

export interface RoomInfo {
  room_id: string;
  creator_id: string;
  users: string[];
  // Add other fields as needed
}

export interface MediaStatus {
  audio: boolean;
  video: boolean;
}

export async function createRoom(room_id: string, creator_id: string) {
  const res = await fetch(`${BASE_URL}/rooms/create`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ room_id, creator_id }),
  });
  if (!res.ok) throw new Error("Failed to create room");
  return res.json();
}

export async function getRoomInfo(room_id: string): Promise<RoomInfo> {
  // The backend expects POST /rooms/details with { room_id }
  const res = await fetch(`${BASE_URL}/rooms/details`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ room_id }),
  });
  if (!res.ok) throw new Error("Failed to fetch room info");
  return res.json();
}

export async function joinRoom(room_id: string, user_id: string, display_name: string) {
  const res = await fetch(`${BASE_URL}/rooms/join`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ room_id, user_id, display_name }),
  });
  if (!res.ok) throw new Error("Failed to join room");
  return res.json();
}

export async function updateMediaStatus(room_id: string, user_id: string, status: MediaStatus) {
  const res = await fetch(`${BASE_URL}/media/update`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ room_id, user_id, ...status }),
  });
  if (!res.ok) throw new Error("Failed to update media status");
  return res.json();
}

export async function leaveRoom(room_id: string, user_id: string) {
  // The backend expects POST /rooms/leave with { room_id, user_id }
  const res = await fetch(`${BASE_URL}/rooms/leave`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ room_id, user_id }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Failed to leave room: ${text}`);
  }
  return res.json();
}

// WebSocket message/event types
export type WsEvent =
  | { event: "Connect"; user_id: string }
  | { event: "Disconnect"; user_id: string }
  | { event: "JoinRequest"; user_id: string; display_name: string }
  | { event: "ApproveJoinRequest"; user_id: string }
  | { event: "DenyJoinRequest"; user_id: string }
  | { event: "JoinApproved" }
  | { event: "JoinDenied" }
  | { event: "MediaStatus"; user_id: string; camera_on: boolean; mic_on: boolean }
  | { event: "WebRTC"; message: any }
  | { event: "Ping" }
  | { event: "Pong" }
  | { event: string; [key: string]: any };

export type WsMessageHandler = (msg: WsEvent) => void;

export class RoomWebSocket {
  private ws: WebSocket | null = null;
  private handlers: Set<WsMessageHandler> = new Set();
  private pingInterval: any = null;
  private url: string;

  constructor(room_id: string, user_id: string) {
    const base = BASE_URL.replace(/^http/, "ws");
    this.url = `${base}/ws/${room_id}/${user_id}`;
  }

  connect() {
    this.ws = new WebSocket(this.url);
    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.handlers.forEach((h) => h(data));
      } catch (e) {
        // ignore parse errors
      }
    };
    this.ws.onopen = () => {
      // Start ping interval
      this.pingInterval = setInterval(() => {
        this.send({ event: "Ping" });
      }, 15000);
    };
    this.ws.onclose = () => {
      if (this.pingInterval) clearInterval(this.pingInterval);
    };
    this.ws.onerror = () => {
      // Optionally handle error
    };
  }

  send(msg: WsEvent) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  addMessageHandler(handler: WsMessageHandler) {
    this.handlers.add(handler);
  }

  removeMessageHandler(handler: WsMessageHandler) {
    this.handlers.delete(handler);
  }

  close() {
    if (this.pingInterval) clearInterval(this.pingInterval);
    if (this.ws) this.ws.close();
  }
}

// Helper to create a websocket connection for a room
export function connectRoomWebSocket(room_id: string, user_id: string) {
  const ws = new RoomWebSocket(room_id, user_id);
  ws.connect();
  return ws;
}