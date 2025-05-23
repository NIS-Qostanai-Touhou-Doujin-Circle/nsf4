export interface Video {
    id: string;
    title: string;
    /** Base64 encoded string */
    thumbnail: string;
    createdAt: string;
}

export async function fetchFeed() {
    // Fetch feed data from API
    const response = await fetch('/api/feed');
    const data = await response.json();

    // Map API response to Video interface
    const videos: Video[] = data.map((item: any) => ({
        id: item.id,
        title: item.title,
        thumbnail: item.thumbnail,
        createdAt: item.createdAt,
    }));

    return videos;
}