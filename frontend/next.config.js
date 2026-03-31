/** @type {import('next').NextConfig} */
const nextConfig = {
  // Allow fetching from the backend server during SSR
  async rewrites() {
    return [];
  },
};

module.exports = nextConfig;
