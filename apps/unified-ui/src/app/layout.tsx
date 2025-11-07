import type { Metadata, Viewport } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';
import { AIProvider } from '@/contexts/AIContext';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { WebSocketProvider } from '@/contexts/WebSocketContext';

const inter = Inter({ subsets: ['latin'] });

export const metadata: Metadata = {
  title: 'IPPAN Unified UI - AI-Powered Blockchain',
  description: 'Advanced blockchain interface with integrated AI capabilities',
  keywords: ['blockchain', 'ai', 'defi', 'smart-contracts', 'ippan'],
  authors: [{ name: 'IPPAN Contributors' }],
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={inter.className}>
        <ThemeProvider>
          <WebSocketProvider>
            <AIProvider>
              <div className="min-h-screen bg-gradient-to-br from-slate-50 to-blue-50 dark:from-slate-900 dark:to-blue-900">
                {children}
              </div>
            </AIProvider>
          </WebSocketProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}