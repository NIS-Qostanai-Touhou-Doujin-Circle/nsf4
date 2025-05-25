import { Link } from "@heroui/link";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";

export default function Page() {
    return (
        <div className="text-center text-xl">
            <h1>Welcome!</h1>
            <Divider className="my-8 w-1/2 mx-auto"/>
            <div>Currently hosting <Chip>30</Chip> RTMP sources (out of them <Chip>30</Chip> Geolocation sources)</div>
            <div>To view the general map, go to <Link href="/map">/map</Link></div>
        </div>
    )
}
