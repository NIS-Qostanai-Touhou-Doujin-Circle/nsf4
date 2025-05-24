'use client';

import { useMemo, useState, useEffect } from "react";
import { fetchFeed } from "../network/feed-get";
// import fetchFeed from "./mock-data";
import { useSearch } from "@/app/components/search-context";
import { Card, CardFooter } from '@heroui/card'
import { Image } from "@heroui/image";
import { Skeleton } from '@heroui/skeleton';
import { Link } from "@heroui/link";
import { Video } from "../types/api";
import { addToast } from "@heroui/toast";
import { Input } from "@heroui/input";
import { Button } from "@heroui/button";
import { Divider } from "@heroui/divider";
import { addDrone } from "../network/add-drone";

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
    let content = null;
    if (filteredVideos.length === 0) {
        if (!searchValue) {
            content = (
                <div className="text-center text-gray-500">
                    No videos available. Please check back later.
                </div>
            );
        } else {
            content = (
                <div className="text-center text-gray-500">
                    No videos found for "{searchValue}"
                </div>
            );
        }
    } else {
        content = (<div className='grid grid-cols-3 gap-x-6 gap-y-3'>
                {filteredVideos.map((video: Video, index) => {
                    const thumbnail = (video.thumbnail ? (<Image
                        src={video.thumbnail}
                        alt={video.title}
                        radius="none"
                        height={200}
                    />) : (<Skeleton className="w-full h-[200px]"></Skeleton>));

                    return <Card key={index} isPressable as={Link} href={"/watch/" + video.id} className="flex flex-col items-center">
                        <div className="bg-default-100 *:m-auto">
                            {thumbnail}
                        </div>
                        <CardFooter className="flex flex-col items-center">
                            <b>{video.title}</b>
                        </CardFooter>
                    </Card>
            })}
            </div>);
    }
    return (
        <div>
            <div className="text-center mx-auto">
                <h1 className="text-2xl font-bold">Feed</h1>
                <AddDroneForm />
            </div>
            <Divider className="my-16"/>
            {content}
        </div>
    )

}

function AddDroneForm() {
    return (
        <form className="w-fit space-y-2 mx-auto" method='POST' onSubmit={async (e) => {
            e.preventDefault();
            const formData = new FormData(e.currentTarget);
            const url = formData.get('url') as string;
            const title = formData.get('title') as string;
            if (!url || !title) {
                addToast({
                    title: "Error",
                    description: "Please fill in all fields",
                    color: "danger",
                    severity: "danger",
                    timeout: 3000
                });
                return;
            }
            try {
                await addDrone({ url, title });
                addToast({
                    title: "Success",
                    description: "Drone added successfully",
                    color: "success",
                    severity: "success",
                    timeout: 3000
                });
            } catch (error: any) {
                addToast({
                    title: "Error",
                    description: error.message,
                    color: "danger",
                    severity: "danger",
                    timeout: 3000
                });
            }
        }}>
            <Input placeholder="Drone URL" name='url'/>
            <Input placeholder="Drone Title" name='title' />
            <Button type='submit' color="primary">Add Drone</Button>
        </form>
    );
}