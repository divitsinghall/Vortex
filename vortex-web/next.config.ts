import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Enable standalone output for Docker deployment
  // This creates a minimal production build that doesn't require node_modules
  output: "standalone",

  // Proxy API requests to the Go backend to avoid CORS issues
  // In Docker, the backend service is at http://backend:8080
  // In development, it's at http://localhost:8080
  async rewrites() {
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";
    return [
      {
        source: "/api/:path*",
        destination: `${apiUrl}/:path*`,
      },
    ];
  },
};

export default nextConfig;
