"use client";

import { useEffect, useRef, useState } from "react";

interface PluginSlotProps {
  name: string;
  className?: string;
}

/**
 * 插件注入点组件
 * 
 * 用法：
 * <PluginSlot name="body_end" />
 * <PluginSlot name="article_content_bottom" />
 * 
 * 插件可以通过以下方式注入内容：
 * Noteva.slots.register('body_end', '<div>My Plugin Content</div>');
 */
export function PluginSlot({ name, className }: PluginSlotProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [mounted, setMounted] = useState(false);

  // 只在客户端挂载后渲染，避免水合错误
  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!mounted || !ref.current) return;
    
    // 等待 Noteva SDK 加载完成
    const tryRender = () => {
      if (typeof window !== "undefined" && (window as any).Noteva) {
        const Noteva = (window as any).Noteva;
        Noteva.slots.render(name, ref.current);
      } else {
        // SDK 还没加载，等待一下
        setTimeout(tryRender, 100);
      }
    };
    
    tryRender();
  }, [name, mounted]);

  // 服务端渲染时返回空 div，避免水合不匹配
  if (!mounted) {
    return <div data-noteva-slot={name} className={className} />;
  }

  return (
    <div
      ref={ref}
      data-noteva-slot={name}
      className={className}
      suppressHydrationWarning
    />
  );
}

export default PluginSlot;
