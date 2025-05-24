'use client';
import { Input } from '@heroui/input';
import { MagnifyingGlassIcon } from '@heroicons/react/24/outline';

import { useSearch } from '@/app/components/search-context';

export function SearchBar() {
    const { search, setSearch } = useSearch();

    return (
        <Input
            fullWidth
            aria-label="Search"
            classNames={{
                inputWrapper: 'bg-default-200 border-default-300 border-1.5',
                input: 'text-sm',
            }}
            labelPlacement="outside"
            placeholder="Search..."
            size="lg"
            startContent={
                <MagnifyingGlassIcon className="text-base text-default-400 pointer-events-none flex-shrink-0 size-4" />
            }
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
        />
    );
}
