'use client';
import React, { useEffect, useState } from "react";
import { Link } from "@heroui/link";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";

export default function Page() {
    const [rtmpCount, setRtmpCount] = useState<number | null>(null);
    const [error, setError] = useState(false);

    useEffect(() => {
        fetch("http://localhost:5123/api/rtmp-count")
            .then((res) => {
                if (!res.ok) throw new Error("Network response was not ok");
                return res.json();
            })
            .then((data) => {
                // Assume the API returns { count: number }
                setRtmpCount(data.count);
            })
            .catch(() => {
                setError(true);
            });
    }, []);

    return (
        <div className="text-center text-xl">
            <h1>Добро пожаловать!</h1>
            <Divider className="my-8 w-1/2 mx-auto"/>
            {!error && rtmpCount !== null && (
                <div>Сейчас стримится <Chip>{rtmpCount}</Chip> RTMP источников</div>
            )}
            <div>Чтобы посмотреть основную карту перейдите на <Link href="/map">/map</Link></div>
        </div>
    )
}
