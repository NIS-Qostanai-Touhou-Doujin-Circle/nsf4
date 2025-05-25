import { load } from '@2gis/mapgl';
import { Map } from '@2gis/mapgl/types';
import React from 'react';
import { useEffect } from 'react';

type MapContextType = {map: Map, api: typeof import("@2gis/mapgl/types/index")}
export const MapContext = React.createContext<[MapContextType | null, React.Dispatch<React.SetStateAction<MapContextType | null>>]>([null, () => {}]);
export const MapProvider = (props : { children: React.ReactNode }) => {
    const [mapInstance, setMapInstance] = React.useState<MapContextType | null>(null);

    return (
        <MapContext.Provider value={[mapInstance, setMapInstance]}>
            {props.children}
        </MapContext.Provider>
    );
};

export const DroneMap = () => {
    const [mapContext, setMapContext] = React.useContext(MapContext);
    const mapInitializedRef = React.useRef(false);
    useEffect(() => {
        console.log('[DroneMap] useEffect called');
        let map: Map | null = null;

        if (mapInitializedRef.current) {
            console.log('[DroneMap] Map already initialized, skipping');
            return;
        }

        load().then((mapglAPI) => {
            console.log('[DroneMap] Initializing map');
            map = new mapglAPI.Map('map-container', {
                center: [55.31878, 25.23584],
                zoom: 13,
                key: '481c2446-3a2e-4a14-be93-31f6d12ced05',
            });
            setMapContext({map, api: mapglAPI});
            mapInitializedRef.current = true;
        }).catch((error) => {
            console.error('Error loading map:', error);
        });

        // Delete the map instance when the component unmounts
        return () => {
            if (map) {
                console.log('[DroneMap] Destroying map');
                map.destroy();
                mapInitializedRef.current = false;
            }
        };
    }, []); // Only run once on mount

    return (
        <div style={{ width: '100%', height: '100%' }}>
            <MapWrapper />
        </div>
    );
};

const MapWrapper = React.memo(
    () => {
        return <div id="map-container" style={{ width: '100%', height: '100%' }} />;
    },
    () => true,
);
MapWrapper.displayName = 'MapWrapper';
