import { SearchBar } from "@/components/searchbar";

export const Navbar = () => {
  return (
    <nav className="w-full min-h-[100px] flex items-center justify-between px-4 py-2 border-b border-default-200 bg-white dark:bg-default-100">
      <div className="flex items-center min-w-[120px]">
        {/* Logo Placeholder */}
        <div className="w-8 h-8 bg-default-200 rounded" />
      </div>
      <div className="flex-1 flex justify-center">
        <div className="max-w-md w-full">
          <SearchBar />
        </div>
      </div>
      <div className="min-w-[120px]" />
    </nav>
  );
};
