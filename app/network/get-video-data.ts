import { Video } from "../types/api";
import { apiUrl } from "./consts";

export async function getVideoData(id: string): Promise<Video | null> { 
    const response = await fetch(`${apiUrl}/drones/${id}`);
    if (response.status === 404) {
        throw new Error("Video not found");
    }
    return response.json();
}