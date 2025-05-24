import { HomeIcon, TrashIcon } from '@heroicons/react/24/outline';
import { Image } from '@heroui/image';
import { Link } from '@heroui/link';

import { SearchBar } from '@/app/components/searchbar';

export const Navbar = () => {
    return (
        <nav className="w-full min-h-[100px] flex items-center justify-between px-4 py-2 border-b border-default-200 bg-white dark:bg-default-100">
            <div className="flex items-center w-[200px] justify-center gap-8">
                <Link className="flex items-center" color="foreground" href="/">
                    <HomeIcon className="size-5" />
                    Home
                </Link>
                <Link className="flex items-center" color="foreground" href="/feed">
                    <TrashIcon className="size-5" />
                    Feed
                </Link>
            </div>
            <div className="flex-1 flex justify-center">
                <div className="max-w-md w-full">
                    <SearchBar />
                </div>
            </div>
            <div className="w-[200px]">
                <Image
                    alt="Logo"
                    className="rounded-full hover:animate-spinner-ease-spin"
                    height={100}
                    src="/depa.png"
                    width={100}
                />
            </div>
        </nav>
    );
};
