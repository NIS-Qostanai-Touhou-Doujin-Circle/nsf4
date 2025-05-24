'use client'
import { useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import Hls from "hls.js";
import { Skeleton } from "@heroui/skeleton";
import { getVideoData } from "@/app/network/get-video-data";
import { addToast } from "@heroui/toast";
import { Video } from "@/app/types/api";
import { mediaServerUrl } from "@/app/network/consts";

export default function WatchVideoPage() {
  const videoRef = useRef<HTMLVideoElement>(null);
  const params = useParams();
  const videoId = params?.video as string;

  const [videoExists, setVideoExists] = useState(true);
  const [title, setTitle] = useState<string | null>(null);

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
    } else if (video.canPlayType("application/vnd.apple.mpegurl")) {
      video.src = src;
    }
  }, [videoId]);

  useEffect(() => {
    getVideoData(videoId)
      .then((data) => {
        setTitle((data as Video).title);
      }).catch((error) => {
        addToast({
          title: "Error fetching video data",
          description: error.message,
          color: "danger",
          severity: "danger",
          timeout: 3000
        });
        setVideoExists(false);
      });
  }, [videoId]);

  return (
    <div className="flex flex-col items-center justify-center">
      <div className="grid grid-cols-[3fr_1fr] gap-4">
        <div className="max-w-screen-xl">
          {!videoExists && (
            <>
              <div className='bg-zinc-900 h-[600px] w-[1000px]'></div>
              <h1 className="text-2xl font-bold text-red-500">Video not found</h1>
            </>
          )}
          {videoExists && (
            <>
              <video
                ref={videoRef}
                controls
                autoPlay
                muted
                className="h-[600px]"
              />
              {title && <h1 className="text-2xl font-bold">{title}</h1>}
              {!title && (
                <Skeleton className="my-2 w-[300px] h-8" />
              )}
            </>
          )}
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
