'use client';

import { useMemo, useState, useEffect } from "react";
import { fetchFeed } from "./feed-get";
// import fetchFeed from "./mock-data";
import { useSearch } from "@/app/components/search-context";
import { Card, CardFooter } from '@heroui/card'
import { Image } from "@heroui/image";
import { Skeleton } from '@heroui/skeleton';
import { Link } from "@heroui/link";
import { Video } from "../types/api";
import { addToast } from "@heroui/toast";

export default function Page() {
    const [searchValue, setSearchValue] = useState("");
    const search = useSearch();
    const [videos, setVideos] = useState<Video[] | null>(null);

    // Keep local searchValue in sync with global search
    useEffect(() => {
        setSearchValue(search.search);
    }, [search.search]);

    // Memoize filteredVideos only when videos or searchValue changes
    const filteredVideos = useMemo(() => {
        if (!videos) return null;
        if (!searchValue) return videos;
        const lower = searchValue.toLowerCase();
        return videos.filter((video: Video) => video.title.toLowerCase().includes(lower));
    }, [videos, searchValue]);
    useEffect(() => {
        fetchFeed().then(setVideos).catch((error) => {
            addToast({
                title: "Error fetching feed",
                description: error.message,
                color: "danger",
                severity: "danger",
                timeout: 3000
            })
            setVideos([]);
        });
    }, []);

    if (filteredVideos === null) {
        return (
            <div className='grid grid-cols-3 gap-x-6 gap-y-3'>
                {Array.from({ length: 12 }).map((_, i) => (
                    <Card key={i}>
                        <Skeleton className="w-full h-[200px]" />
                        <CardFooter className="flex flex-col items-center">
                            <Skeleton  className="h-6 mt-2" />
                        </CardFooter>
                    </Card>
                ))}
            </div>
        );
    }
    return (
        <div className='grid grid-cols-3 gap-x-6 gap-y-3'>
            {filteredVideos.map((video: Video, index) => (
                <Card key={index} isPressable as={Link} href={"/watch/" + video.id} className="flex flex-col items-center">
                    <div className="bg-default-100 *:m-auto">
                        <Image 
                            src={video.thumbnail}
                            alt={video.title}
                            radius="none"
                            height={200}/>
                    </div>
                    <CardFooter className="flex flex-col items-center">
                        <b>{video.title}</b>
                    </CardFooter>
                </Card>
            ))}
        </div>
    )

}