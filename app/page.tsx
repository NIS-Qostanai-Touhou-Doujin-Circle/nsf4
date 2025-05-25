import { Link } from "@heroui/link";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";

export default function Page() {
    return (
        <div className="text-center text-xl">
            <h1>Добро пожаловать!</h1>
            <Divider className="my-8 w-1/2 mx-auto"/>
            <div>Сейчас стримится <Chip>30</Chip> RTMP источников</div>
            <div>Чтобы посмотреть основную карту перейдите на <Link href="/map">/map</Link></div>
        </div>
    )
}
