/** @type {import('next').NextConfig} */
const nextConfig = {
  experimental: {
    appDir: true,
  },
  env: {
    NEXT_PUBLIC_ENABLE_FULL_UI: process.env.NEXT_PUBLIC_ENABLE_FULL_UI || '1',
    NEXT_PUBLIC_GATEWAY_URL: process.env.NEXT_PUBLIC_GATEWAY_URL || 'http://localhost:8081/api',
    NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:7080',
    NEXT_PUBLIC_WS_URL: process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:7080/ws',
    NEXT_PUBLIC_AI_ENABLED: process.env.NEXT_PUBLIC_AI_ENABLED || '1',
  },
  webpack: (config) => {
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      net: false,
      tls: false,
    };
    return config;
  },
  async rewrites() {
    return [
      {
        source: '/api/ai/:path*',
        destination: `${process.env.NEXT_PUBLIC_GATEWAY_URL}/ai/:path*`,
      },
      {
        source: '/api/blockchain/:path*',
        destination: `${process.env.NEXT_PUBLIC_GATEWAY_URL}/blockchain/:path*`,
      },
    ];
  },
};

module.exports = nextConfig;