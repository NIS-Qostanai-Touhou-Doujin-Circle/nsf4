'use client';

import type { ThemeProviderProps } from 'next-themes';

import * as React from 'react';
import { HeroUIProvider } from '@heroui/system';
import { useRouter } from 'next/navigation';
import { ThemeProvider as NextThemesProvider } from 'next-themes';
import { ToastProvider } from '@heroui/toast';

import { SearchProvider } from '@/app/components/search-context';
import { MapProvider } from './components/map';

export interface ProvidersProps {
    children: React.ReactNode;
    themeProps?: ThemeProviderProps;
}

declare module '@react-types/shared' {
    interface RouterConfig {
        routerOptions: NonNullable<Parameters<ReturnType<typeof useRouter>['push']>[1]>;
    }
}
export function Providers({ children, themeProps }: ProvidersProps) {
    const router = useRouter();
    const [mounted, setMounted] = React.useState(false);

    React.useEffect(() => {
        setMounted(true);
    }, []);

    if (!mounted) {
        // To avoid flashing with bright white, render a blank screen
        return (
            <div className="flex items-center justify-center h-screen bg-black">
                <div className="w-16 h-16 border-4 border-blue-500 border-t-transparent rounded-full animate-spin" />
            </div>
        )
    }

    return (
        <HeroUIProvider navigate={router.push}>
            <NextThemesProvider {...themeProps}>
                <ToastProvider />
                <MapProvider>
                    <SearchProvider>{children}</SearchProvider>
                </MapProvider>
            </NextThemesProvider>
        </HeroUIProvider>
    );
}
