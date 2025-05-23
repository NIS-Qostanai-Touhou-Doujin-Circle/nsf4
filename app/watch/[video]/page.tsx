'use client'
import { useEffect, useRef } from "react";
import { useParams } from "next/navigation";
import Hls from "hls.js";

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
    <div className="flex flex-col items-center justify-center min-h-screen">
      <video
        ref={videoRef}
        controls
        autoPlay
        style={{ width: "100%", maxWidth: 800 }}
      />
      <p className="mt-4 text-gray-500">Live HLS Stream: {videoId}</p>
    </div>
  );
}
