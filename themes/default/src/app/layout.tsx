import type { Metadata } from "next";
import { Inter, Noto_Sans_SC } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import PluginSlot from "@/components/plugin-slot";

const inter = Inter({ 
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const notoSansSC = Noto_Sans_SC({
  subsets: ["latin"],
  variable: "--font-noto-sans-sc",
  display: "swap",
  weight: ["400", "500", "600", "700"],
});

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
    <html lang="zh-CN" suppressHydrationWarning className={`${inter.variable} ${notoSansSC.variable}`}>
      <head>
        {/* 插件 CSS */}
        <link rel="stylesheet" href="/api/v1/plugins/assets/plugins.css" />
        {/* 控制台 Logo */}
        <script dangerouslySetInnerHTML={{
          __html: `
            (function() {
              console.log('%c Noteva ', 'background: #4a90e2; color: white; font-size: 24px; font-weight: bold; padding: 10px 20px; border-radius: 5px;');
              console.log('%c🔗 Github: https://github.com/noteva26/Noteva', 'color: #666; font-size: 14px; margin-top: 10px;');
            })();
          `
        }} />
      </head>
      <body className="font-sans antialiased">
        {/* body_start 插槽 - 全局遮罩、加载动画 */}
        <PluginSlot name="body_start" />
        
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          {children}
          <Toaster position="top-center" />
        </ThemeProvider>
        
        {/* body_end 插槽 - JS、悬浮组件、音乐播放器 */}
        <PluginSlot name="body_end" />
        
        {/* SDK 和插件由后端自动注入到 </head> 前，无需手动引入 */}
      </body>
    </html>
  );
}
