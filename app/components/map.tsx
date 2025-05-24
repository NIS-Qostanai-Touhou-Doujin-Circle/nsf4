// import { load } from '@2gis/mapgl';
// import { Map } from '@2gis/mapgl/types';
// import React from 'react';
// import { useEffect } from 'react';

// export const DroneMap = () => {
//     useEffect(() => {
//         let map : Map | null = null;
//         load().then((mapglAPI) => {
//             map = new mapglAPI.Map('map-container', {
//                 center: [55.31878, 25.23584],
//                 zoom: 13,
//                 key: 'Your API access key',
//             });
//         });

//         // Удаляем карту при размонтировании компонента
//         return () => (map && map.destroy());
//     }, []);

//     return (
//         <div style={{ width: '100%', height: '100%' }}>
//             <MapWrapper />
//         </div>
//     );
// };
// const MapWrapper = React.memo(
//     () => {
//         return <div id="map-container" style={{ width: '100%', height: '100%' }}></div>;
//     },
//     () => true,
// );