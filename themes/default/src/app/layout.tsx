import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import PluginSlot from "@/components/plugin-slot";
import Script from "next/script";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Noteva",
  description: "A lightweight blog powered by Noteva",
  icons: {
    icon: "/logo.png",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <head>
        {/* 插件 CSS */}
        <link rel="stylesheet" href="/api/v1/plugins/assets/plugins.css" />
      </head>
      <body className={inter.className}>
        {/* body_start 插槽 - 全局遮罩、加载动画 */}
        <PluginSlot name="body_start" />
        
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          {children}
          <Toaster />
        </ThemeProvider>
        
        {/* body_end 插槽 - JS、悬浮组件、音乐播放器 */}
        <PluginSlot name="body_end" />
        
        {/* Noteva SDK */}
        <Script src="/noteva-sdk.js" strategy="beforeInteractive" />
        {/* 插件 JS */}
        <Script src="/api/v1/plugins/assets/plugins.js" strategy="afterInteractive" />
      </body>
    </html>
  );
}
