'use client';
import { useContext, useEffect, useRef, useState } from 'react';
import { useParams } from 'next/navigation';
import Hls from 'hls.js';
import { Skeleton } from '@heroui/skeleton';
import { addToast } from '@heroui/toast';

import { Video } from '@/app/types/api';
import { mediaServerUrl } from '@/app/network/consts';
import { getVideoData } from '@/app/network/get-video-data';
import { DroneMap, MapContext } from '@/app/components/map';
import { load } from '@2gis/mapgl'

export default function WatchVideoPage() {
    const videoRef = useRef<HTMLVideoElement>(null);
    const params = useParams();
    const videoId = params?.video as string;

    const [videoExists, setVideoExists] = useState(true);
    const [title, setTitle] = useState<string | null>(null);

    const [mapPoint, setMapPoint] = useState<[number, number] | null>([55.31878, 25.23584]);
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
        const point = new api.Marker(map, {
            coordinates: mapPoint,
        });
        setCleanupFunc(() => point.destroy);
    }, [mapPoint, mapContext]);

    useEffect(() => {
        setInterval(() => {
            if (!mapPoint) return;
            setMapPoint((prev) => {
                if (!prev) return null;
                return [prev[0] + 0.0001, prev[1] + 0.0001];
            });
        }, 500);
    }, []);

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
                </div>
            </div>
        </div>
    );
}
