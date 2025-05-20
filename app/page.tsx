'use client';
import React, { useRef, useState, useCallback, useEffect } from 'react';
import { Input } from '@heroui/input';
import { Button } from '@heroui/button';
import { Card, CardHeader, CardBody } from '@heroui/card';

import { WebRTCManager, Logger } from './webrtcService';

export default function WebRTCTestPage() {
    const [roomId, setRoomId] = useState('test-room');
    const [isCameraOn, setIsCameraOn] = useState(true);
    const [isMicrophoneOn, setIsMicrophoneOn] = useState(true);
    const [webrtcManager, setWebrtcManager] = useState<WebRTCManager | null>(null);
    const [connectionMode, setConnectionMode] = useState<'p2p' | 'central'>('p2p');

    const localVideoRef = useRef<HTMLVideoElement>(null);
    const remoteVideoRef = useRef<HTMLVideoElement>(null);

    const logger: Logger = useCallback((type, message) => {
        const timestamp = new Date().toLocaleTimeString();

        console.log(`[${timestamp}] [${type.toUpperCase()}] ${message}`);
    }, []);

    const handleJoinRoom = useCallback(async () => {
        if (!roomId.trim()) {
            alert('Please enter a room ID');

            return;
        }
        if (localVideoRef.current && remoteVideoRef.current) {
            const manager = new WebRTCManager(
                localVideoRef.current,
                remoteVideoRef.current,
                roomId,
                process.env.NEXT_PUBLIC_BackendUrl!,
                logger,
                connectionMode,
            );

            setWebrtcManager(manager);
            await manager.joinRoom();
            setIsCameraOn(true);
            setIsMicrophoneOn(true);
        }
    }, [roomId, logger, connectionMode]);

    const handleLeaveRoom = useCallback(() => {
        if (webrtcManager) {
            webrtcManager.leaveRoom();
            setWebrtcManager(null);
            setIsCameraOn(false);
            setIsMicrophoneOn(false);
        }
    }, [webrtcManager]);

    const toggleCamera = useCallback(() => {
        if (webrtcManager) {
            webrtcManager.toggleCamera();
            setIsCameraOn((prev) => !prev);
        }
    }, [webrtcManager]);

    const toggleMicrophone = useCallback(() => {
        if (webrtcManager) {
            webrtcManager.toggleMicrophone();
            setIsMicrophoneOn((prev) => !prev);
        }
    }, [webrtcManager]);

    const toggleConnectionMode = useCallback(() => {
        setConnectionMode((prev) => (prev === 'p2p' ? 'central' : 'p2p'));
    }, []);

    useEffect(() => {
        return () => {
            if (webrtcManager) {
                webrtcManager.leaveRoom();
            }
        };
    }, [webrtcManager]);

    return (
        <section className="flex flex-col items-center justify-center gap-4 py-8 md:py-10">
            <h1 className="text-3xl font-bold">WebRTC Signaling Test</h1>
            <div className="flex items-center gap-2 my-4">
                <Input
                    label="Room ID"
                    placeholder="Enter Room ID"
                    type="text"
                    value={roomId}
                    onChange={(e) => setRoomId(e.target.value)}
                />
                <Button color="primary" onPress={handleJoinRoom}>
                    Join Room
                </Button>
                <Button color="danger" onPress={handleLeaveRoom}>
                    Leave Room
                </Button>
                <Button color="default" onPress={toggleConnectionMode}>
                    {connectionMode === 'p2p' ? 'Switch to Client Server' : 'Switch to P2P'}
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
                        <div className="flex gap-2 mt-2 justify-center">
                            <Button
                                color={isCameraOn ? 'primary' : 'default'}
                                onPress={toggleCamera}
                            >
                                {isCameraOn ? 'Turn Camera Off' : 'Turn Camera On'}
                            </Button>
                            <Button
                                color={isMicrophoneOn ? 'primary' : 'default'}
                                onPress={toggleMicrophone}
                            >
                                {isMicrophoneOn ? 'Turn Mic Off' : 'Turn Mic On'}
                            </Button>
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
                    </CardBody>
                </Card>
            </div>
        </section>
    );
}
