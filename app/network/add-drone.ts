// POST /api/drones - Add a new drone
export async function addDrone({ url, title }: { url: string; title: string }) {
    const response = await fetch('http://192.168.0.101:5123/api/drones', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ url, title }),
    });
    if (!response.ok) {
        throw new Error('Failed to add drone');
    }
    return response.json();
}
