import { Feed, Video } from '../types/api';

import { apiUrl } from './consts';

export async function fetchFeed() {
    // Fetch feed data from API
    const response = await fetch(`${apiUrl}/feed`).catch((error) => {
        throw new Error('Failed to fetch feed data: ' + error.message);
    });

    if (response.status === 404) {
        throw new Error('Feed API not found (404)');
    }
    if (!response.ok) {
        throw new Error('Failed to fetch feed data: ' + response.statusText);
    }
    const data: Feed = await response.json().catch((error) => {
        throw new Error('Failed to parse JSON response: ' + error.message);
    });

    if (data.videos === undefined) {
        throw new Error('Invalid response from API');
    }
    const videos: Video[] = data.videos;

    return videos;
}
