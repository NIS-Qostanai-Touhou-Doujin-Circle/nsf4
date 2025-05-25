import { AddDroneResponse } from '../types/api';
import { apiUrl } from './consts';

// POST /api/drones - Add a new drone
export async function addDrone({ url, title, ws }: { url: string; title: string; ws: string }): Promise<AddDroneResponse> {
    const response = await fetch(`${apiUrl}/drones`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ url, title, ws }),
    });

    if (!response.ok) {
        throw new Error('Failed to add drone');
    }

    return response.json();
}

export async function deleteDrone(id: string) {
    const response = await fetch(`${apiUrl}/drones/${id}`, {
        method: 'DELETE',
    });

    if (!response.ok) {
        throw new Error('Failed to delete drone');
    }

    return response.json();
}