import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Proxy API requests to the Go backend to avoid CORS issues
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: 'http://localhost:8080/:path*',
      },
    ];
  },
};

export default nextConfig;
