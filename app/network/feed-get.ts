import { Feed, Video } from "../types/api";
import { apiUrl } from "./consts";


export async function fetchFeed() {
    // Fetch feed data from API
    const response = await fetch(`${apiUrl}/feed`).catch((error) => {
        console.warn("Error fetching feed data:", error);
        throw new Error("Failed to fetch feed data");
    });
    if (response.status === 404) {
        console.warn("Feed API not found (404)");
        throw new Error("Feed API not found (404)");
    }
    if (!response.ok) {
        console.warn("Error response from API:", response.statusText);
        throw new Error("Failed to fetch feed data: " + response.statusText);
    }
    const data: Feed = await response.json().catch((error) => {
        console.warn("Error parsing JSON response:", error);
        throw new Error("Failed to parse JSON response");
    });

    if (data.videos === undefined) {
        throw new Error("Invalid response from API");
    }
    const videos: Video[] = data.videos;

    return videos;
}