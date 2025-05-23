'use client'
import { useEffect, useRef } from "react";
import Hls from "hls.js";

export default function Page() {
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    const src = "http://localhost:6210/mystream.m3u8";
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
  }, []);

  return (
    <div className="flex flex-col items-center justify-center min-h-screen">
      <video
        ref={videoRef}
        controls
        autoPlay
        style={{ width: "100%", maxWidth: 800 }}
      />
      <p className="mt-4 text-gray-500">Live HLS Stream</p>
    </div>
  );
}