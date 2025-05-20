export type LogType = 'local' | 'remote';
export type Logger = (type: LogType, message: string) => void;

export class WebRTCManager {
    roomId: string;
    backendUrl: string;
    localVideo: HTMLVideoElement;
    remoteVideo: HTMLVideoElement;
    logger: Logger;
    connectionMode: 'p2p' | 'central';

    peerConnection: RTCPeerConnection | null = null;
    socket: WebSocket | null = null;
    localStream: MediaStream | null = null;
    otherUserId: string | null = null;

    constructor(
        localVideo: HTMLVideoElement,
        remoteVideo: HTMLVideoElement,
        roomId: string,
        backendUrl: string,
        logger: Logger,
        connectionMode: 'p2p' | 'central' = 'p2p',
    ) {
        this.localVideo = localVideo;
        this.remoteVideo = remoteVideo;
        this.roomId = roomId;
        this.backendUrl = backendUrl;
        this.logger = logger;
        this.connectionMode = connectionMode;
    }

    async setupWebRTC(): Promise<boolean> {
        try {
            if (
                typeof navigator === 'undefined' ||
                !navigator.mediaDevices ||
                !navigator.mediaDevices.getUserMedia
            ) {
                this.logger(
                    'local',
                    'MediaDevices API (getUserMedia) недоступен. Используйте HTTPS или localhost.',
                );

                return false;
            }

            const stream = await navigator.mediaDevices.getUserMedia({
                video: true,
                audio: true,
            });

            this.localVideo.srcObject = stream;
            this.localStream = stream;
            this.logger('local', 'Local stream acquired.');

            const config = {
                iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
            };
            const pc = new RTCPeerConnection(config);

            this.peerConnection = pc;

            stream.getTracks().forEach((track) => {
                pc.addTrack(track, stream);
            });
            this.logger('local', 'Tracks added to PeerConnection.');

            pc.onicecandidate = (event) => {
                if (event.candidate && this.socket && this.socket.readyState === WebSocket.OPEN) {
                    this.logger('local', 'Sending ICE candidate');
                    this.socket.send(
                        JSON.stringify({
                            type: 'candidate',
                            candidate: event.candidate,
                            room: this.roomId,
                        }),
                    );
                }
            };

            pc.ontrack = (event) => {
                this.logger('remote', 'Received remote track');
                this.remoteVideo.srcObject = event.streams[0];
            };

            this.logger('local', 'WebRTC setup complete.');

            return true;
        } catch (error: any) {
            this.logger('local', 'Error setting up WebRTC: ' + error.message);

            return false;
        }
    }

    connectSignaling() {
        const endpoint = this.connectionMode === 'central' ? 'central' : 'signaling';
        const url = `wss://${this.backendUrl}/${endpoint}`;
        const ws = new WebSocket(url);

        this.socket = ws;

        ws.onopen = () => {
            this.logger('local', `Connected to ${this.connectionMode} signaling server`);
            if (this.connectionMode === 'p2p') {
                ws.send(
                    JSON.stringify({
                        type: 'join',
                        room: this.roomId,
                    }),
                );
                if (this.peerConnection) {
                    this.logger('local', 'Creating offer...');
                    this.peerConnection
                        .createOffer()
                        .then((offer) =>
                            this.peerConnection!.setLocalDescription(offer).then(() => {
                                ws.send(
                                    JSON.stringify({
                                        type: 'offer',
                                        offer: offer,
                                        room: this.roomId,
                                    }),
                                );
                                this.logger('local', 'Offer sent.');
                            }),
                        )
                        .catch((error: any) => {
                            this.logger(
                                'local',
                                'Error creating or sending offer: ' + error.message,
                            );
                        });
                }
            } else {
                // central mode
                ws.send(
                    JSON.stringify({
                        type: 'central_offer',
                        room: this.roomId,
                    }),
                );
                this.logger('local', 'Central offer sent.');
            }
        };

        ws.onmessage = async (event) => {
            try {
                const message = JSON.parse(event.data as string);

                this.logger('local', `Received message: ${message.type}`);
                if (!this.peerConnection) {
                    this.logger('local', 'PeerConnection not initialized. Ignoring message.');

                    return;
                }
                const pc = this.peerConnection;

                switch (message.type) {
                    case 'offer':
                        if (message.from && message.from === (ws as any).userId) {
                            this.logger('local', 'Ignoring own offer.');

                            return;
                        }
                        this.otherUserId = message.from;
                        this.logger('local', `Processing offer from ${message.from}`);
                        await pc.setRemoteDescription(new RTCSessionDescription(message.offer));
                        const answer = await pc.createAnswer();

                        await pc.setLocalDescription(answer);
                        ws.send(
                            JSON.stringify({
                                type: 'answer',
                                answer: answer,
                                room: this.roomId,
                            }),
                        );
                        this.logger('local', `Sent answer to ${message.from}`);
                        break;

                    case 'answer':
                        if (message.from && message.from === (ws as any).userId) {
                            this.logger('local', 'Ignoring own answer.');

                            return;
                        }
                        this.logger('local', `Processing answer from ${message.from}`);
                        await pc.setRemoteDescription(new RTCSessionDescription(message.answer));
                        break;

                    case 'candidate':
                        if (message.from && message.from === (ws as any).userId) {
                            this.logger('local', 'Ignoring own ICE candidate.');

                            return;
                        }
                        if (message.candidate) {
                            this.logger('local', `Adding ICE candidate from ${message.from}`);
                            await pc.addIceCandidate(new RTCIceCandidate(message.candidate));
                        }
                        break;

                    case 'central_answer':
                        this.logger('local', `Processing central answer`);
                        await pc.setRemoteDescription(
                            new RTCSessionDescription({ type: 'answer', sdp: message.sdp }),
                        );
                        break;

                    default:
                        this.logger('local', `Unknown message type: ${message.type}`);
                }
            } catch (error: any) {
                this.logger('local', 'Error processing message: ' + error.message);
            }
        };

        ws.onclose = () => {
            this.logger('local', 'Disconnected from signaling server');
        };

        ws.onerror = (error) => {
            this.logger('local', 'WebSocket error: ' + (error as any).message);
        };
    }

    async joinRoom(): Promise<void> {
        this.cleanupConnections();
        const rtcReady = await this.setupWebRTC();

        if (rtcReady) {
            this.connectSignaling();
        }
    }

    leaveRoom() {
        this.logger('local', 'Leaving room and cleaning up...');
        this.cleanupConnections();
    }

    toggleCamera() {
        if (this.localStream) {
            const videoTrack = this.localStream.getVideoTracks()[0];

            if (videoTrack) {
                videoTrack.enabled = !videoTrack.enabled;
                this.logger('local', `Camera ${videoTrack.enabled ? 'turned ON' : 'turned OFF'}`);
            }
        }
    }

    toggleMicrophone() {
        if (this.localStream) {
            const audioTrack = this.localStream.getAudioTracks()[0];

            if (audioTrack) {
                audioTrack.enabled = !audioTrack.enabled;
                this.logger(
                    'local',
                    `Microphone ${audioTrack.enabled ? 'turned ON' : 'turned OFF'}`,
                );
            }
        }
    }

    cleanupConnections() {
        if (this.peerConnection) {
            this.peerConnection.close();
            this.peerConnection = null;
            this.logger('local', 'Previous PeerConnection closed.');
        }
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.close();
            this.socket = null;
            this.logger('local', 'Previous WebSocket connection closed.');
        }
        if (this.localStream) {
            this.localStream.getTracks().forEach((track) => track.stop());
            this.localStream = null;
            this.logger('local', 'Previous local stream stopped.');
        }
        this.localVideo.srcObject = null;
        this.remoteVideo.srcObject = null;
    }
}
