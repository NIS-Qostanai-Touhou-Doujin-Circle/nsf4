"use client";
import { useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import Hls from "hls.js";
import { Skeleton } from "@heroui/skeleton";
import { addToast } from "@heroui/toast";

import { getVideoData } from "@/app/network/get-video-data";

export default function WatchVideoPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const params = useParams();
  const videoId = params?.video as string;

  const [videoExists, setVideoExists] = useState(true);

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

  useEffect(() => {
    getVideoData(videoId)
      .then((data) => {
        addToast({
          description: JSON.stringify(data, null, 2),
        });
      })
      .catch((error) => {
        addToast({
          title: "Error fetching video data",
          description: error.message,
          color: "danger",
          severity: "danger",
          timeout: 3000,
        });
        setVideoExists(false);
      });
  }, [videoId]);

  return (
    <div className="flex flex-col items-center justify-center">
      <div className="grid grid-cols-[3fr_1fr] gap-4">
        <div>
          <video ref={videoRef} autoPlay controls muted className="h-[600px]" />
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
