'use client';
import { useCallback, useContext, useEffect, useRef, useState } from 'react';
import { useParams } from 'next/navigation';
import Hls from 'hls.js';
import { Skeleton } from '@heroui/skeleton';
import { addToast } from '@heroui/toast';

import { Video } from '@/app/types/api';
import { mediaServerUrl, serverUrl } from '@/app/network/consts';
import { getVideoData } from '@/app/network/get-video-data';
import { DroneMap, MapContext } from '@/app/components/map';
import { load } from '@2gis/mapgl'
import { TrashIcon } from '@heroicons/react/24/outline';
import { Button } from '@heroui/button';
import usePrevious from '@/app/helpers/usePrevious';



export default function WatchVideoPage() {
    const videoRef = useRef<HTMLVideoElement>(null);
    const params = useParams();
    const videoId = params?.video as string;

    const [videoExists, setVideoExists] = useState(true);
    const [title, setTitle] = useState<string | null>(null);

    const [mapPoint, setMapPoint] = useState<[number, number] | null>([63.658686, 53.218282]);
    const previousMapPoint = usePrevious(mapPoint);
    const [cleanupFunc, setCleanupFunc] = useState<(() => void) | null>(null);
    const [mapContext, _] = useContext(MapContext);
    useEffect(() => {
        if (!mapContext) return;
        const map = mapContext.map;
        if (!map) return;
        const api = mapContext.api;
        if (!api) return;
        if (!mapPoint) return;
        cleanupFunc?.();
        let rotation = 0;
        if (previousMapPoint) {
            const dx = mapPoint[0] - previousMapPoint[0];
            const dy = mapPoint[1] - previousMapPoint[1];
            rotation = Math.atan2(-dy, dx) * (180 / Math.PI) + 180;
        }
        const point = new api.Marker(map, {
            coordinates: mapPoint,
            icon: "/depa.png",
            size: [100, 100],
            label: {
                text: title || 'Drone',
                color: 'white',
                fontSize: 16
            },
            rotation: rotation,
        });
        setCleanupFunc(() => () => {point.destroy();});
    }, [mapPoint, mapContext]);

    // --- WebSocket logic for live drone position ---
    useEffect(() => {
        if (!videoId) return;
        // Convert http:// to ws:// for WebSocket
        const wsUrl = serverUrl.replace(/^http/, 'ws') + `/ws/${videoId}`;
        let ws: WebSocket | null = new WebSocket(wsUrl);
        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data).data;
                if (typeof data.latitude === 'number' && typeof data.longitude === 'number') {
                    setMapPoint([data.latitude, data.longitude]);
                }
            } catch (e) {
                addToast({
                    title: 'WebSocket error',
                    description: (e as Error).message
                });
            }
        };
        ws.onopen = () => {
                addToast({
                    title: 'WebSocket connected',
                    color: 'success',
                    severity: 'success',
                    timeout: 3000,
                });
        };
        ws.onerror = () => {
            addToast({
                title: 'WebSocket error',
                description: 'Could not connect to drone position stream.',
                color: 'danger',
                severity: 'danger',
                timeout: 3000,
            });
        };
        return () => {
            ws?.close();
        };
    }, [videoId]);
    // --- End WebSocket logic ---

    useEffect(() => {
        const video = videoRef.current;

        if (!video || !videoId) return;
        const src = `${mediaServerUrl}/${videoId}.m3u8`;

        if (Hls.isSupported()) {
            const hls = new Hls();

            hls.loadSource(src);
            hls.attachMedia(video);

            return () => {
                hls.destroy();
            };
        } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
            video.src = src;
        }
    }, [videoId]);

    useEffect(() => {
        getVideoData(videoId)
            .then((data) => {
                setTitle((data as Video).title);
            })
            .catch((error) => {
                addToast({
                    title: 'Error fetching video data',
                    description: error.message,
                    color: 'danger',
                    severity: 'danger',
                    timeout: 3000,
                });
                setVideoExists(false);
            });
    }, [videoId]);

    const findDrone = useCallback(() => {
        if (!mapContext) return;
        const map = mapContext.map;
        if (!map) return;
        if (!mapPoint) {
            addToast({
                title: 'Error',
                description: 'Drone location is not available',
                color: 'danger',
                severity: 'danger',
                timeout: 3000,
            });
            return;
        }
        map.setCenter(mapPoint);
    }, [mapPoint]);

    return (
        <div className="flex flex-col items-center justify-center">
            <div className="grid grid-cols-2 max-w-screen-xl w-full gap-8">
                <div>
                    {!videoExists && (
                        <>
                            <div className="bg-zinc-900 h-[360px] w-full" />
                            <h1 className="text-2xl font-bold text-red-500">Video not found</h1>
                        </>
                    )}
                    {videoExists && (
                        <>
                            <video ref={videoRef} autoPlay controls muted className="h-[360px]" />
                            {title && <h1 className="text-2xl font-bold">{title}</h1>}
                            {!title && <Skeleton className="my-2 w-[300px] h-8" />}
                        </>
                    )}
                </div>
                <div className='w-full h-[400px]'>
                    <DroneMap />
                    <Button onPress={findDrone}>Find the drone</Button>
                </div>
            </div>
        </div>
    );
}
