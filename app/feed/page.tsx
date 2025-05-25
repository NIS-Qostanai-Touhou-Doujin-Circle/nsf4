'use client';

import { useMemo, useState, useEffect } from 'react';
import { Card, CardFooter } from '@heroui/card';
import { Image } from '@heroui/image';
import { Skeleton } from '@heroui/skeleton';
import { Link } from '@heroui/link';
import { addToast } from '@heroui/toast';
import { Input } from '@heroui/input';
import { Button } from '@heroui/button';
import { Divider } from '@heroui/divider';
import { Dropdown, DropdownItem, DropdownMenu, DropdownTrigger } from '@heroui/dropdown';
import { Modal, ModalContent, ModalBody, ModalHeader, ModalProvider, useDisclosure, ModalFooter } from '@heroui/modal';

import { Video } from '../types/api';
import { fetchFeed } from '../network/feed-get';
// import fetchFeed from '../network/mock-data';
import { addDrone, deleteDrone } from '../network/drone';

import { useSearch } from '@/app/components/search-context';
import { EllipsisVerticalIcon, MapPinIcon, SignalIcon, TrashIcon } from '@heroicons/react/24/outline';

export default function Page() {
    const [searchValue, setSearchValue] = useState('');
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
        fetchFeed()
            .then(setVideos)
            .catch((error) => {
                addToast({
                    title: 'Error fetching feed',
                    description: error.message,
                    color: 'danger',
                    severity: 'danger',
                    timeout: 3000,
                });
                setVideos([]);
            });
    }, []);

    let content = null;

    if (filteredVideos === null) {
        content = (
            <div className="grid grid-cols-3 gap-x-6 gap-y-6">
                {Array.from({ length: 12 }).map((_, i) => (
                    <Card key={i}>
                        <Skeleton className="w-full h-[200px]" />
                        <CardFooter className="flex flex-col items-center">
                            <Skeleton className="h-6 mt-2" />
                        </CardFooter>
                    </Card>
                ))}
            </div>
        );
    } else if (filteredVideos.length === 0) {
        if (!searchValue) {
            content = (
                <div className="text-center text-gray-500">
                    No videos available yet. Please check back later or add a new source.
                </div>
            );
        } else {
            content = (
                <div className="text-center text-gray-500">
                    No videos found for &ldquo;{searchValue}&rdquo;
                </div>
            );
        }
    } else {
        content = (
            <div className="grid grid-cols-3 gap-x-6 gap-y-6">
                {filteredVideos.map((video: Video, index) => (
                    <Playable key={index} video={video} deleteDrone={
                        () => deleteDrone(video.id).then((ans) => {
                            if (ans.success !== true) {
                                throw new Error(ans.message || 'Failed to delete source');
                            }
                            setVideos((prevVideos) => prevVideos?.filter((v) => v.id !== video.id) || null);
                            addToast({
                                title: 'Source deleted successfully',
                                description: `Source "${video.title}" has been deleted.`,
                                color: 'success',
                                severity: 'success',
                                timeout: 3000,
                            });
                        }).catch((error) => {
                            addToast({
                                title: 'Error deleting source',
                                description: error.message,
                                color: 'danger',
                                severity: 'danger',
                                timeout: 3000,
                            });
                        })
                    } />
                ))}
            </div>
        );
    }

  const {isOpen, onOpen, onOpenChange} = useDisclosure();

    return (
        <div className='max-w-screen-lg mx-auto'>
            <div className="text-center mx-auto">
                <div className='flex justify-evenly w-full'>
                    <div className='text-left max-w-xs'>
                        <h1>
                            Feed
                        </h1>
                        <div>
                            <p>
                                Here you can find all sources (drones) that are currently online and broadcasting their video feed.
                            </p>
                        </div>
                    </div>
                    <div className='text-right space-y-2 mt-24'>
                        <p>Didn't find what you were looking for?</p>
                        <Button onPress={onOpen} color="primary">
                            Add New Source
                        </Button>
                    </div>
                </div>
                <Modal isOpen={isOpen} onOpenChange={onOpenChange} size='md'>
                    <ModalContent>
                        {(onClose) => {
                            return (
                                <>
                                    <ModalHeader className='mt-20'>
                                        <h1 className='text-center mx-auto'>Add New Source</h1>
                                    </ModalHeader>
                                    <ModalBody>
                                        <AddDroneForm videos={videos} setVideos={setVideos} onClose={onClose} />
                                    </ModalBody>
                                    <ModalFooter className='mb-10'>
                                    </ModalFooter>
                                </>
                            );
                        }}
                    </ModalContent>
                </Modal>
            </div>
            <Divider className="my-16" />
            {content}
        </div>
    );
}

function Playable({ video, deleteDrone }: { video: Video, deleteDrone: () => Promise<void> }) {
    const thumbnail = video.thumbnail ? (
        <Image removeWrapper alt={video.title} height={200} radius="none" src={video.thumbnail} className='w-full object-cover'/>
    ) : (
        <Skeleton className="w-full h-[200px]" />
    );

    return (
        <div className='relative'>
            <Dropdown>
                <>
                    <Card
                        as={Link}
                        href={'/watch/' + video.id}
                        isPressable
                    >
                        <div className="bg-default-100 w-full">{thumbnail}</div>
                        <CardFooter className="flex flex-col items-center">
                            <b>{video.title}</b>
                        </CardFooter>
                    </Card>
                    <div className='absolute bottom-1 right-2 radius-full'>
                        <DropdownTrigger>
                            <Button variant="light" radius='full' isIconOnly>
                                <EllipsisVerticalIcon className="size-4 text-white" />
                            </Button>
                        </DropdownTrigger>
                    </div>
                </>
                <DropdownMenu onAction={(action) => {
                    if (action === 'delete') {
                        deleteDrone();
                    }
                }}>
                    <DropdownItem variant='flat' key='delete' color='danger' startContent={<TrashIcon className="size-4 text-danger" />}>Delete</DropdownItem>
                </DropdownMenu>
            </Dropdown>
        </div>
    );
}

function AddDroneForm({ videos, setVideos, onClose }: { videos: Video[] | null, setVideos: React.Dispatch<React.SetStateAction<Video[] | null>>, onClose: () => void }) {
    return (
        <form
            className="w-full px-8 space-y-2 mx-auto"
            method="POST"
            onSubmit={async (e) => {
                e.preventDefault();
                const formData = new FormData(e.currentTarget);
                const url = formData.get('url') as string;
                const title = formData.get('title') as string;
                const ws = formData.get('ws') as string;
                // Validate inputs
                if (ws && !ws.startsWith('ws://') && !ws.startsWith('wss://')) {
                    addToast({
                        title: 'Error',
                        description: 'WebSocket URL must start with ws:// or wss://',
                        color: 'danger',
                        severity: 'danger',
                        timeout: 3000,
                    });
                    return;
                }
                if (!url || !title) {
                    addToast({
                        title: 'Error',
                        description: 'Please fill in all fields',
                        color: 'danger',
                        severity: 'danger',
                        timeout: 3000,
                    });

                    return;
                }
                if (!url.startsWith('rtmp://')) {
                    addToast({
                        title: 'Error',
                        description: 'URL must start with rtmp://',
                        color: 'danger',
                        severity: 'danger',
                        timeout: 3000,
                    });
                    return;
                }
                if (!url.includes(':1935')) {
                    addToast({
                        title: 'Warning',
                        description: 'URL probably should include port :1935',
                        color: 'warning',
                        severity: 'warning',
                        timeout: 3000,
                    });
                }
                try {
                    let drone = await addDrone({ url, title, ws });
                    const video = {
                        ...drone,
                        thumbnail: '',
                    } as Video;
                    setVideos((prevVideos) => [...(prevVideos || []), video]);
                    addToast({
                        title: 'Success',
                        description: 'Source added successfully',
                        color: 'success',
                        severity: 'success',
                        timeout: 3000,
                    });
                    onClose();
                } catch (error: any) {
                    addToast({
                        title: 'Error',
                        description: error.message,
                        color: 'danger',
                        severity: 'danger',
                        timeout: 3000,
                    });

                }
            }}
        >
            <Input fullWidth name="title" label="Source Title (Name)" isRequired />
            <Divider className='mt-4 mb-6' />
            <Input fullWidth name="url" label={
                <div className='inline-flex items-center gap-1'><SignalIcon className='size-4' /> RTMP URL</div>
            } isRequired />
            <Input fullWidth name="ws" label={
                <div className='inline-flex items-center gap-1'><MapPinIcon className='size-4'/> WebSocket URL</div>
            } />
            <Divider className='mt-4 mb-6' />
            <div className='w-full flex justify-center mt-4'>
                <Button color="primary" type="submit" size='lg' className='inline-block text-center mx-auto w-2/3'>
                    Add Source
                </Button>
            </div>
        </form>
    );
}
