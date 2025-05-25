'use client';
import { useContext, useEffect, useRef, useState } from "react";
import { DroneMap, MapContext } from "../components/map";
import usePrevious from "../helpers/usePrevious";
import { Card } from "@heroui/card";
import { Marker } from "@2gis/mapgl/types";

export default function Page() {
    return (
        <div className="h-2/3">
            <div className="text-center mb-4">
                <h1>Global Map</h1>
                <p>
                    This map shows the current location of all geolocation sources.
                    <br/>
                    You can see the direction of the drone by the rotation of the icon.
                </p>
            </div>
            <Card className="h-full" shadow="lg">
                <DronesMap/>
            </Card>
        </div>
    )
}

function DronesMap() {
    const [mapContext] = useContext(MapContext);
    const count = 500;
    const [points, setPoints] = useState<[number, number][]>(Array.from({ length: count }).map(() => {
        const lat = 55.31878 + (Math.random() - 0.5) * 0.01;
        const lng = 25.23584 + (Math.random() - 0.5) * 0.01;
        return [lat, lng];
    }));
    const velocities = useRef(Array.from({ length: count }).map(() => [0.0001, 0.0001]));
    const previousPoints = usePrevious(points);
    const markersRef = useRef<any[]>([]);

    useEffect(() => {
        if (!mapContext || !mapContext.map || !mapContext.api) return;
        const map = mapContext.map;
        const api = mapContext.api;

        // Create markers only once
        if (markersRef.current.length === 0) {
            markersRef.current = points.map((point, index) => {
                let rotation = 0;
                if (previousPoints && previousPoints[index]) {
                    const dx = point[0] - previousPoints[index][0];
                    const dy = point[1] - previousPoints[index][1];
                    rotation = Math.atan2(-dy, dx) * (180 / Math.PI) + 180;
                }
                return new api.Marker(map, {
                    coordinates: point,
                    icon: "/depa.png",
                    size: [80, 80],
                    rotation
                });
            });
        }
        // Cleanup on unmount
        return () => {
            markersRef.current.forEach(marker => marker.destroy && marker.destroy());
            markersRef.current = [];
        };
    // Only run on mount/unmount or if mapContext changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [mapContext]);

    useEffect(() => {
        // Update marker positions and rotations
        if (!markersRef.current.length) return;
        points.forEach((point, index) => {
            let rotation = 0;
            if (previousPoints && previousPoints[index]) {
                const dx = point[0] - previousPoints[index][0];
                const dy = point[1] - previousPoints[index][1];
                rotation = Math.atan2(-dy, dx) * (180 / Math.PI) + 180;
            }
            const marker: Marker = markersRef.current[index];
            if (marker) {
                marker.setCoordinates(point);
                marker.setRotation(rotation);
            }
        });
    }, [points, previousPoints]);

    useEffect(() => {
        // Simulate point updates
        const interval = setInterval(() => {
            velocities.current.forEach((velocity, index) => {
                velocity[0] += (Math.random() - 0.5) * 0.0001;
                velocity[1] += (Math.random() - 0.5) * 0.0001;
            });
            setPoints((prevPoints) => {
                return prevPoints.map((point, index) => [
                    point[0] + velocities.current[index][0],
                    point[1] + velocities.current[index][1],
                ]);
            });
        }, 32);

        return () => clearInterval(interval);
    }, []);

    return (<DroneMap />);
}