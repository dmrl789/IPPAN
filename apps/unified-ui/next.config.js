const assetPrefix = process.env.NEXT_PUBLIC_ASSET_PREFIX || '';

/** @type {import('next').NextConfig} */
const nextConfig = {
  assetPrefix: assetPrefix || undefined,
  // Enable static export for optimal deployment
  output: 'export',
  // Disable image optimization for static export
  images: {
    unoptimized: true,
  },
  // Trailing slash for better static hosting compatibility
  trailingSlash: true,
  env: {
    NEXT_PUBLIC_ASSET_PREFIX: assetPrefix,
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
};

module.exports = nextConfig;