'use client';
import { useContext, useEffect, useState } from "react";
import { DroneMap, MapContext } from "../components/map";
import usePrevious from "../helpers/usePrevious";

export default function Page() {
    return (
        <DronesMap/>
    )
}


function DronesMap() {
    const [mapContext, setMapContext] = useContext(MapContext);
    const count = 30;
    const [points, setPoints] = useState<[number, number][]>(Array.from({ length: count }).map((i, index) => {
        const lat = 55.31878 + (Math.random() - 0.5) * 0.01;
        const lng = 25.23584 + (Math.random() - 0.5) * 0.01;
        return [lat, lng];
    }))
    const velocities = Array.from({ length: count }).map(() => [0.0001, 0.0001]);
    const previousPoints = usePrevious(points);
    const [cleanupFunctions, setCleanupFunctions] = useState<(() => void)[]>([]);
    useEffect(() => {
        if (!mapContext || !mapContext.map) return;
        const map = mapContext.map;
        const api = mapContext.api;
        // Clear previous points
        cleanupFunctions.forEach(fn => fn());
        setCleanupFunctions([]);
        const newCleanupFunctions: (() => void)[] = [];
        points.forEach((point, index) => {
            let rotation = 0;
            if (previousPoints && previousPoints[index]) {
                const dx = point[0] - previousPoints[index][0];
                const dy = point[1] - previousPoints[index][1];
                rotation = Math.atan2(-dy, dx) * (180 / Math.PI) + 180;
            }
            const marker = new api.Marker(map, {
                coordinates: point,
                icon: "/depa.png",
                size: [80, 80],
                rotation
            });
            newCleanupFunctions.push(() => marker.destroy());
        });
        setCleanupFunctions(newCleanupFunctions);
    }, [mapContext, points]);

    useEffect(() => {
        // Simulate point updates
        const interval = setInterval(() => {
            velocities.forEach((velocity, index) => {
                velocities[index][0] += (Math.random() - 0.5) * 0.0001;
                velocities[index][1] += (Math.random() - 0.5) * 0.0001;
            });
            setPoints((prevPoints) => {
                return prevPoints.map((point, index) => [
                    point[0] + velocities[index][0],
                    point[1] + velocities[index][1],
                ]);
            });
        }, 10);

        return () => clearInterval(interval);
    }, []);


    return (<DroneMap />);

}