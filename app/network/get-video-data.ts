import { Video } from '../types/api';

import { apiUrl } from './consts';

export async function getVideoData(id: string): Promise<Video | null> {
    const response = await fetch(`${apiUrl}/drones/${id}`, {
        signal: AbortSignal.timeout(5000), // 5 seconds timeout
    });

    if (response.status === 404) {
        throw new Error('Video not found');
    }
    if (!response.ok) {
        throw new Error(`Failed to fetch video data: ${response.statusText}`);
    }

    return response.json();
}
