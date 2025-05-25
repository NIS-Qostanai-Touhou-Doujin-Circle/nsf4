import { Link } from "@heroui/link";


export default function Page() {
    return (
        <div>
            <h1>Welcome!</h1>
            <p>Currently hosting 30 RTMP sources (out of them 30 Geolocation sources)</p>
            <p>To view the general map, go to <Link href="/map">/map</Link></p>
        </div>
    )
}
