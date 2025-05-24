export interface Video {
    id: string;
    ip: string;
    title: string;
    /** Base64 encoded string */
    thumbnail: string;
    createdAt: string;
}

export interface Feed {
    videos: Video[];
}

export interface AddDroneRequest {
    ip: string;
    title: string;
}

export interface AddDroneResponse {
    id: string;
    ip: string;
    title: string;
    createdAt: string;
}

export interface DeleteDroneRequest {
    id: string;
}

export interface DeleteDroneResponse {
    success: boolean;
}