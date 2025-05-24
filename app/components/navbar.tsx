import { SearchBar } from "@/app/components/searchbar";
import { HomeIcon, TrashIcon } from "@heroicons/react/24/outline";
import { Image } from "@heroui/image";
import { Link } from "@heroui/link";

export const Navbar = () => {
  return (
    <nav className="w-full min-h-[100px] flex items-center justify-between px-4 py-2 border-b border-default-200 bg-white dark:bg-default-100">
      <div className="flex items-center w-[200px] justify-center gap-8">
        <Link href="/" className="flex items-center" color='foreground'><HomeIcon className="size-5" />Home</Link>
        <Link href="/feed" className="flex items-center" color='foreground'><TrashIcon className="size-5" />Feed</Link>
      </div>
      <div className="flex-1 flex justify-center">
        <div className="max-w-md w-full">
          <SearchBar />
        </div>
      </div>
      <div className="w-[200px]">
        <Image src='/depa.png' alt="Logo" width={100} height={100} className="rounded-full hover:animate-spinner-ease-spin" />
      </div>
    </nav>
  );
};
