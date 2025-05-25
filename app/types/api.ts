export interface Video {
    id: string;
    url: string;
    title: string;
    /** Base64 encoded string */
    thumbnail: string;
    createdAt: string;
}

export interface Feed {
    videos: Video[];
}

export interface AddDroneRequest {
    url: string;
    title: string;
}

export interface AddDroneResponse extends Omit<Video, 'thumbnail'> {
}

export interface DeleteDroneRequest {
    id: string;
}

export interface DeleteDroneResponse {
    success: boolean;
}
