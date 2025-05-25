import { HomeIcon, SignalIcon, MapIcon } from '@heroicons/react/24/outline';
import { Image } from '@heroui/image';
import { Link } from '@heroui/link';

import { SearchBar } from '@/app/components/searchbar';

export const Navbar = () => {
    return (
        <nav className="w-full min-h-[100px] items-center px-4 py-2 border-b border-default-200 bg-white dark:bg-default-100">
            <div className='grid grid-cols-3 gap-4 max-w-screen-xl mx-auto *:my-auto'>
                <div className="flex items-center justify-center">
                    <Image
                        alt="Logo"
                        className="rounded-full hover:animate-spinner-ease-spin"
                        height={100}
                        src="/depa.png"
                        width={100}
                    />
                </div>
                <div className="flex justify-center">
                    <div className="w-full max-w-md">
                        <SearchBar />
                    </div>
                </div>
                <div className="flex items-center justify-center gap-8">
                    <Link className="flex items-center" color="foreground" href="/">
                        <HomeIcon className="size-5 text-zinc-500 mr-1" />
                        Home
                    </Link>
                    <Link className="flex items-center" color="foreground" href="/feed">
                        <SignalIcon className="size-5 text-zinc-500 mr-1" />
                        Feed
                    </Link>
                    <Link className="flex items-center" color="foreground" href="/map">
                        <MapIcon className="size-5 text-zinc-500 mr-1" />
                        Map
                    </Link>
                </div>
            </div>
        </nav>
    );
};
