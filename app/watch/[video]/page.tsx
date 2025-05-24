'use client'
import { useEffect, useRef } from "react";
import { useParams } from "next/navigation";
import Hls from "hls.js";
import { Skeleton } from "@heroui/skeleton";

export default function WatchVideoPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const params = useParams();
  const videoId = params?.video as string;

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !videoId) return;
    const src = `http://localhost:6210/${videoId}.m3u8`;
    if (Hls.isSupported()) {
      const hls = new Hls();
      hls.loadSource(src);
      hls.attachMedia(video);
      return () => {
        hls.destroy();
      };
    } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
      video.src = src;
    }
  }, [videoId]);

  return (
    <div className="flex flex-col items-center justify-center">
      <div className="grid grid-cols-[3fr_1fr] gap-4">
        <div>
          <video
            ref={videoRef}
            controls
            autoPlay
            muted
            className="h-[600px]"
          />
          <h1>{videoId}</h1>
        </div>
        <div className="space-y-4">
          <Skeleton className="w-[300px] h-[200px]" />
          <Skeleton className="w-[300px] h-[200px]" />
          <Skeleton className="w-[300px] h-[200px]" />
        </div>
      </div>
    </div>
  );
}
