/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  distDir: '../admin-dist',
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
  // Disable prefetch to avoid RSC errors in static export
  experimental: {
    missingSuspenseWithCSRBailout: false,
  },
};

module.exports = nextConfig;
